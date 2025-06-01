use anyhow::Result;
use std::collections::HashSet;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use std::process::ExitStatus;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

// Same module
use crate::command::QueueMessage;
use crate::command::execution_report::ExecOutput;
use crate::command::execution_report::{ExecCode, ExecMessage, ExecStart};
use crate::command::exit_code;

use crate::args::{Args, FILE_SUBSTITUTION, FILES_SUBSTITUTION};
use crate::errors::ProgramErrors;

use super::exit_code::ExitCode;

pub struct Queue {
    /// Shell to use to to spawn the command
    shell: &'static str,
    /// Command to execute, with arguments
    command: Vec<String>,
    /// Files that have been updated - pending command execution
    /// First pathbuf is the file, second is the watched file/dir
    files: HashSet<(PathBuf, PathBuf)>,
    /// Execution mode
    batch_exec: bool,
    /// Execute commands also if files are deleted
    deleted_files: bool,
    /// Handle to receive QueueMessages
    rx: Receiver<QueueMessage>,
    /// Handle to send Execution Updates from the runner
    report_tx: Sender<ExecMessage>,
    /// Timestamp of the last file update
    last_update: Option<std::time::Instant>,
    /// Total command count.
    command_count: usize,
    /// Do we abort previous commands?
    abort_previous: bool,
    /// Abort signal for workers
    abort: Arc<AtomicBool>,
}

impl Queue {
    pub fn start(
        args: &Args,
        report_tx: Sender<ExecMessage>,
    ) -> std::sync::mpsc::Sender<QueueMessage> {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut queue = Self {
            shell: args.shell,
            command: args.command.clone(),
            files: HashSet::new(),
            batch_exec: args.batch_exec,
            deleted_files: args.deleted,
            rx,
            report_tx,
            last_update: None,
            command_count: 0,
            abort_previous: args.abort_previous,
            abort: Arc::new(AtomicBool::new(false)),
        };

        std::thread::spawn(move || queue.run());
        tx
    }

    pub fn run(&mut self) {
        loop {
            match self.rx.try_recv() {
                Ok(QueueMessage::Abort) => break,
                Ok(QueueMessage::RestartBackoff) => {
                    if !self.files.is_empty() {
                        self.last_update = Some(std::time::Instant::now());
                    }
                }
                Ok(QueueMessage::AddFile(p, watch)) => {
                    let _ = self.files.insert((p, watch));
                    self.last_update = Some(std::time::Instant::now());
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(e) => {
                    eprintln!("Channel error: {:?}", e);
                    break;
                }
            }
            if let Some(t) = self.last_update {
                if t.elapsed() > std::time::Duration::from_millis(200) {
                    let tx_result = self.execute();

                    if let Err(e) = tx_result {
                        eprintln!("Exec Tx Report Channel error: {:?}", e);
                        return;
                    }

                    if self.files.is_empty() {
                        self.last_update = None;
                    }
                }
            }
        }
    }

    /// Picks up the next file-batch and spawn a thread executing the
    /// command
    pub fn execute(&mut self) -> Result<(), ProgramErrors> {
        if self.files.is_empty() {
            return Err(ProgramErrors::BadInternalState);
        }

        // Choose arguments based on the placeholders
        let mut p: Vec<PathBuf> = if !self.batch_exec {
            let paths = self.files.iter().next().unwrap().clone();
            self.files.remove(&paths);
            vec![paths.0]
        } else {
            self.files.drain().map(|(p, _)| p).collect()
        };
        assert!(!p.is_empty(), "p should not be empty. Files: {:?}, ", self.files);

        // Remove deleted files unless we want them
        if !self.deleted_files {
            p.retain(|p| p.exists());
        }
        if p.is_empty() {
            return Ok(());
        }
        let p = p; // Immutable now
        // dbg!(&p);

        let shell_parts = shell_words::split(self.shell).map_err(|_| {
            ProgramErrors::CommandParseError(
                self.shell.to_string(),
                "Failed to parse shell command".to_string(),
            )
        })?;

        let mut command = Command::new(&shell_parts[0]);
        for arg in &shell_parts[1..] {
            command.arg(arg);
        }

        // File the arguments, replace the placeholders
        for arg in &self.command {
            match arg {
                //FIXME: do this job once in args and just keep a pre-parsed vector for next time
                a if a == FILE_SUBSTITUTION => command.arg(p[0].clone()),
                a if a == FILES_SUBSTITUTION => command.args(p.clone()),
                a if a.contains(FILE_SUBSTITUTION) => {
                    command.arg(a.replace(FILE_SUBSTITUTION, p[0].to_string_lossy().as_ref()))
                }
                a if a.contains(FILES_SUBSTITUTION) => command.arg(a.replace(
                    FILES_SUBSTITUTION,
                    p.iter().map(|pb| pb.to_string_lossy()).collect::<Vec<_>>().join(" ").as_str(),
                )),
                a => command.args([a]),
            };
        }
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        // Abort previous commands if needed
        if self.abort_previous {
            //FIXME: There is for sure a better way to do this
            self.abort.store(true, Ordering::SeqCst);
            std::thread::yield_now();

            self.abort.store(false, Ordering::SeqCst);
        }

        //dbg!(&command);
        let command_number = self.command_count;
        self.command_count += 1;
        self.report_tx
            .send(ExecMessage::Start(ExecStart {
                command_number,
                files: p
                    .iter()
                    .map(|pb| pb.file_name().unwrap().to_string_lossy().into_owned())
                    .collect(),
            }))
            .map_err(|e| ProgramErrors::CommandExecutionError(e.to_string()))?;

        let tx_clone = self.report_tx.clone();
        let abort = self.abort.clone();
        std::thread::spawn(move || run_command(command_number, command, tx_clone, abort));

        Ok(())
    }
}

pub fn run_command(
    command_number: usize,
    mut command: Command,
    report_tx: Sender<ExecMessage>,
    abort: Arc<AtomicBool>,
) {
    let mut child = command.spawn().expect("Command could not start");

    // Send stdout updates to tx reports
    let stdout = BufReader::new(child.stdout.take().unwrap());
    let stdout_tx = report_tx.clone();
    let stdout_handle = std::thread::spawn(move || {
        for line in stdout.lines() {
            let line = line.unwrap();
            let _ = stdout_tx.send(ExecMessage::Output(ExecOutput {
                command_number,
                stdout: Some(line),
                stderr: None,
            }));
        }
    });

    // Send stderr updates to tx reports
    let stderr = BufReader::new(child.stderr.take().unwrap());
    let stderr_tx = report_tx.clone();
    let stderr_handle = std::thread::spawn(move || {
        for line in stderr.lines() {
            let line = line.unwrap();
            let _ = stderr_tx.send(ExecMessage::Output(ExecOutput {
                command_number,
                stdout: None,
                stderr: Some(line),
            }));
        }
    });

    // Check atomic bool / try wait
    let status: Option<ExitStatus> = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                // Command is running, wait more
            }
            Err(_) => break None,
        }

        if abort.load(Ordering::SeqCst) {
            let _ = child.kill();
        }
    };

    stdout_handle.join().unwrap();
    stderr_handle.join().unwrap();

    let exit_code: ExitCode = match status {
        Some(s) => exit_code::get_exit_code(s),
        None => None,
    };

    let _ = report_tx.send(ExecMessage::Finish(ExecCode { command_number, exit_code }));
}
