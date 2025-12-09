use anyhow::Result;
use std::collections::HashSet;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use std::process::ExitStatus;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

const MAX_CONCURRENT_WORKERS: usize = 3;

// Same module
use crate::command::QueueMessage;
use crate::command::execution_report::ExecOutput;
use crate::command::execution_report::{ExecCode, ExecMessage, ExecStart};
use crate::command::exit_code;

use crate::args::{Args, FILE_SUBSTITUTION, FILES_SUBSTITUTION};
use crate::errors::{ArgumentError, ProgramError, RuntimeError, arg_error, runtime_error};
use crate::event::Event;

use super::exit_code::ExitCode;

macro_rules! send_msg_unchecked {
    ($tx:ident, $q_msg:expr) => {
        let _ = $tx.send(Event::Exec($q_msg));
    };
}

pub struct Queue {
    /// Prepared command to which we need to add the args / env variables
    command_base: Command,
    /// Command to execute, to pass to the shell (i.e. sh -c "command to execute with args")
    command: String,
    /// Files that have been updated - pending command execution
    /// First pathbuf is the file, second is the watched file/dir
    files: HashSet<(PathBuf, PathBuf)>,
    /// Do we keep the command outputs
    pipe_command_output: bool,
    /// Do we configure a particular working dir for commands
    working_dir: Option<String>,
    /// Execution mode
    batch_exec: bool,
    /// Execute commands also if files are deleted
    deleted_files: bool,
    /// Handle to receive QueueMessages
    rx: Receiver<QueueMessage>,
    /// Handle to send Execution Updates from the runner
    report_tx: Sender<Event>,
    /// Timestamp of the last file update
    last_update: Option<std::time::Instant>,
    /// Total command count.
    command_count: usize,
    /// Do we abort previous commands?
    abort_previous: bool,
    /// Abort signal for workers
    abort: Arc<AtomicBool>,
    /// worker handles
    workers: Vec<JoinHandle<()>>,
}

impl Queue {
    pub fn start(
        args: &Args,
        report_tx: Sender<Event>,
    ) -> Result<Sender<QueueMessage>, ProgramError> {
        let (tx, rx) = crossbeam_channel::unbounded();

        // Parse the command and prep it
        if args.command.len() != 1 {
            return Err(runtime_error!(
                InternalError,
                format!(
                    "Args.command should have been reduced to a single element {:?}",
                    args.command
                )
            ));
        }

        let shell_parts = shell_words::split(args.shell).map_err(|_| {
            arg_error!(
                CommandParseError,
                args.shell.to_string(),
                "Failed to parse shell command".to_string()
            )
        })?;

        let mut command = Command::new(&shell_parts[0]);
        for arg in &shell_parts[1..] {
            command.arg(arg);
        }

        // Env variables.
        for env_var in &args.env {
            let mut parts = env_var.splitn(2, "=");
            let key = parts.next();
            let value = parts.next().unwrap_or(""); // Default env variable value will be ""

            if key.is_none() {
                return Err(arg_error!(InvalidEnvironmentVariable, env_var.to_owned()));
            }
            command.env(key.unwrap(), value);
        }

        let mut queue = Self {
            command_base: command,
            command: args.command[0].clone(),
            files: HashSet::new(),
            pipe_command_output: !args.quiet,
            working_dir: args.current_working_dir.clone(),
            batch_exec: args.batch_exec,
            deleted_files: args.deleted,
            rx,
            report_tx,
            last_update: None,
            command_count: 0,
            abort_previous: args.abort_previous,
            abort: Arc::new(AtomicBool::new(false)),
            workers: Vec::with_capacity(MAX_CONCURRENT_WORKERS),
        };

        std::thread::spawn(move || queue.run());
        Ok(tx)
    }

    fn get_command(&self) -> Command {
        let mut command = Command::new(self.command_base.get_program());
        command.args(self.command_base.get_args());
        self.command_base.get_envs().for_each(|(k, v)| {
            if let Some(value) = v {
                command.env(k, value);
            }
        });

        if self.pipe_command_output {
            command.stdout(Stdio::piped());
            command.stderr(Stdio::piped());
        } else {
            command.stdout(Stdio::null());
            command.stderr(Stdio::null());
        }

        command
    }

    pub fn run(&mut self) {
        loop {
            // Receive messages
            match self.rx.recv_timeout(Duration::from_millis(100)) {
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
                Err(RecvTimeoutError::Timeout) => {}
                Err(e) => {
                    eprintln!("Channel error: {e:?}");
                    break;
                }
            }
            // remove finished workers
            self.workers.retain(|w| !w.is_finished());

            // See if we want to execute something
            if let Some(t) = self.last_update
                && t.elapsed() > std::time::Duration::from_millis(200)
            {
                let tx_result = self.execute();

                if let Err(e) = tx_result {
                    eprintln!("Exec Tx Report Channel error: {e:?}");
                    return;
                }

                if self.files.is_empty() {
                    self.last_update = None;
                }
            }
        }
    }

    /// Picks up the next file-batch and spawn a thread executing the
    /// command
    pub fn execute(&mut self) -> Result<(), ProgramError> {
        if self.files.is_empty() {
            return Err(runtime_error!(
                InternalError,
                "Trying to execute commands with an empty queue".into()
            ));
        }

        // Remove deleted files unless we want them
        if !self.deleted_files {
            self.files.retain(|(p, _)| p.exists());
        }

        if self.files.is_empty() {
            return Ok(());
        }

        // Abort previous commands if needed
        if self.abort_previous && !self.workers.is_empty() {
            self.abort.store(true, Ordering::SeqCst);
            // We could probably use a rendezvous channel or something like that to make
            // sure the other threads have read the value.
            std::thread::sleep(Duration::from_millis(100));
        }
        self.abort.store(false, Ordering::SeqCst);

        // Choose arguments based on the placeholders
        let p: Vec<PathBuf> = if !self.batch_exec {
            let paths = self.files.iter().next().unwrap().clone();
            self.files.remove(&paths);
            vec![paths.0]
        } else {
            self.files.drain().map(|(p, _)| p).collect()
        };
        assert!(!p.is_empty(), "p should not be empty. Files: {:?}, ", self.files);

        // Start assembling the command
        let mut command = self.get_command();

        if let Some(cwd) = &self.working_dir {
            command.current_dir(cwd);
        }

        // File the arguments, replace the placeholders
        if self.command.contains(FILE_SUBSTITUTION) {
            command.arg(self.command.replace(FILE_SUBSTITUTION, p[0].to_string_lossy().as_ref()));
        } else if self.command.contains(FILES_SUBSTITUTION) {
            command.arg(self.command.replace(
                FILES_SUBSTITUTION,
                p.iter().map(|pb| pb.to_string_lossy()).collect::<Vec<_>>().join(" ").as_str(),
            ));
        } else {
            command.arg(&self.command);
        }

        // Queue house keeping.
        let command_number = self.command_count;
        self.command_count += 1;
        self.report_tx
            .send(Event::Exec(ExecMessage::Start(ExecStart {
                command_number,
                files: p
                    .iter()
                    .map(|pb| pb.file_name().unwrap().to_string_lossy().into_owned())
                    .collect(),
            })))
            .map_err(|e| runtime_error!(CommandExecutionError, e.to_string()))?;

        let tx_clone = self.report_tx.clone();
        let abort = self.abort.clone();
        let pipe_output = self.pipe_command_output;
        self.workers.push(std::thread::spawn(move || {
            run_command(command_number, command, tx_clone, abort, pipe_output)
        }));

        Ok(())
    }
}

pub fn run_command(
    command_number: usize,
    mut command: Command,
    report_tx: Sender<Event>,
    abort: Arc<AtomicBool>,
    pipe_output: bool,
) {
    let mut child = command.spawn().expect("Command could not start");

    // Send stdout updates to tx reports
    if pipe_output {
        let tx_clone = report_tx.clone();
        let _ = pipe_child_streams_to_events(&mut child, tx_clone, command_number);
    }

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
        // Avoid polling with too much excitement and avoid a CPU spin
        std::thread::sleep(Duration::from_millis(40));
    };

    let exit_code: ExitCode = match status {
        Some(s) => exit_code::get_exit_code(s),
        None => None,
    };

    send_msg_unchecked!(report_tx, ExecMessage::Finish(ExecCode { command_number, exit_code }));
}

fn pipe_child_streams_to_events(
    child: &mut std::process::Child,
    report_tx: Sender<Event>,
    command_number: usize,
) -> (JoinHandle<()>, JoinHandle<()>) {
    // Send stdout updates to tx reports
    let stdout = BufReader::new(child.stdout.take().unwrap());
    let stdout_tx = report_tx.clone();
    let stdout_handle = std::thread::spawn(move || {
        for line in stdout.lines() {
            let line = line.unwrap();
            send_msg_unchecked!(
                stdout_tx,
                ExecMessage::Output(ExecOutput {
                    command_number,
                    stdout: Some(line),
                    stderr: None,
                })
            );
        }
    });

    // Send stderr updates to tx reports
    let stderr = BufReader::new(child.stderr.take().unwrap());
    let stderr_tx = report_tx.clone();
    let stderr_handle = std::thread::spawn(move || {
        for line in stderr.lines() {
            let line = line.unwrap();
            send_msg_unchecked!(
                stderr_tx,
                ExecMessage::Output(ExecOutput {
                    command_number,
                    stdout: None,
                    stderr: Some(line),
                })
            );
        }
    });

    (stdout_handle, stderr_handle)
}
