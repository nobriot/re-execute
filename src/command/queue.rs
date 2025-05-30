use crate::args::{Args, FILE_SUBSTITUTION, FILES_SUBSTITUTION};
use crate::command::QueueMessage;
use crate::errors::ProgramErrors;
use anyhow::Result;
use std::collections::HashSet;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use std::sync::mpsc::{Receiver, Sender};

// Same module
use crate::command::execution_report::{ExecutionReport, ExecutionStart, ExecutionUpdate};
use crate::command::exit_code;

pub struct Queue {
    /// Command to execute, with arguments
    command: Vec<String>,
    /// Files that have been updated - pending command execution
    files: HashSet<(PathBuf, PathBuf)>,
    /// Execution mode
    batch_exec: bool,
    /// Execute commands also if files are deleted
    deleted_files: bool,
    rx: Receiver<QueueMessage>,
    report_tx: Sender<ExecutionUpdate>,
    last_update: Option<std::time::Instant>,
    command_count: usize,
}

impl Queue {
    pub fn new(
        args: &Args,
        report_tx: Sender<ExecutionUpdate>,
    ) -> std::sync::mpsc::Sender<QueueMessage> {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut queue = Self {
            command: args.command.clone(),
            files: HashSet::new(),
            batch_exec: args.batch_exec,
            deleted_files: args.deleted,
            rx,
            report_tx,
            last_update: None,
            command_count: 0,
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
                    // println!("Adding file: {:?} / Path: {:?}", p, watch);
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
                //debug!("elapsed: {:?}", t.elapsed());
                if t.elapsed() > std::time::Duration::from_millis(200) {
                    let _ = self.execute();
                    // let exec_result = self.execute();
                    // if let Ok(report) = exec_result {
                    //     // dbg!(&report);
                    //     let tx_result = self.report_tx.send(ExecutionUpdate::Finish(report));
                    //     if let Err(e) = tx_result {
                    //         eprintln!("Exec Tx Report Channel error: {:?}", e);
                    //         break;
                    //     }
                    // } else {
                    //     eprintln!("Error with the queue trying to execute commands");
                    //     break;
                    // }

                    if self.files.is_empty() {
                        self.last_update = None;
                    }
                }
            }
        }
    }

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

        // Remove deleted files unless we want them
        if !self.deleted_files {
            p.retain(|p| p.exists());
        }
        let p = p; // Immutable now
        // dbg!(&p);

        let mut command = Command::new(&self.command[0]);
        // let concatenated_args = self.args.join(" ");
        // let arg_tokens = shell_words::split(&concatenated_args)
        //     .map_err(|e| ProgramErrors::CommandParseError(self.command.clone(), e.to_string()))?;
        //println!("{:?}", &self.command);
        //println!("{:?}", arg_tokens);

        // File the arguments, replace the placeholders
        if !p.is_empty() {
            for arg in &self.command[1..] {
                match arg {
                    a if a == FILE_SUBSTITUTION => command.arg(p[0].clone()),
                    a if a == FILES_SUBSTITUTION => command.args(p.clone()),
                    a if a.contains(FILE_SUBSTITUTION) => {
                        command.arg(a.replace(FILE_SUBSTITUTION, p[0].to_string_lossy().as_ref()))
                    }
                    a if a.contains(FILES_SUBSTITUTION) => command.arg(
                        a.replace(
                            FILES_SUBSTITUTION,
                            p.iter()
                                .map(|pb| pb.to_string_lossy())
                                .collect::<Vec<_>>()
                                .join(" ")
                                .as_str(),
                        ),
                    ),
                    a => command.args([a]),
                };
            }
        }
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        //dbg!(&command);
        // println!("Running command: '{:?}'", command);
        let command_number = self.command_count;
        self.command_count += 1;
        if let Err(e) = self.report_tx.send(ExecutionUpdate::Start(ExecutionStart {
            command_number,
            files: p
                .iter()
                .map(|pb| pb.file_name().unwrap().to_string_lossy().into_owned())
                .collect(),
        })) {
            eprintln!("Error running command: {:?}", e);
            return Err(ProgramErrors::CommandExecutionError(e.to_string()));
        }

        let tx_clone = self.report_tx.clone();
        std::thread::spawn(move || run_command(command_number, command, tx_clone));

        // let mut child = command.spawn()?;
        // let status = child.wait()?;

        // let stdout = if let Some(mut stdout) = child.stdout.take() {
        //     let mut output = String::new();
        //     let _ = stdout.read_to_string(&mut output);
        //     //println!("Command output: {:?}", output);
        //     Some(output)
        // } else {
        //     None
        // };
        // let stderr = if let Some(mut stderr) = child.stderr.take() {
        //     let mut output = String::new();
        //     let _ = stderr.read_to_string(&mut output);
        //     Some(output)
        // } else {
        //     None
        // };

        // if let Err(e) = self.report_tx.send(ExecutionUpdate::Finish(ExecutionReport {
        //     command_number,
        //     exit_code: exit_code::get_exit_code(status),
        //     stdout,
        //     stderr,
        // })) {
        //     return Err(ProgramErrors::CommandExecutionError(e.to_string()));
        // }
        Ok(())
    }
}

pub fn run_command(
    command_number: usize,
    mut command: Command,
    report_tx: Sender<ExecutionUpdate>,
) {
    let mut child = command.spawn().expect("Command could not start");
    let status = child.wait().expect("command could not finish");

    let stdout = if let Some(mut stdout) = child.stdout.take() {
        let mut output = String::new();
        let _ = stdout.read_to_string(&mut output);
        Some(output)
    } else {
        None
    };
    let stderr = if let Some(mut stderr) = child.stderr.take() {
        let mut output = String::new();
        let _ = stderr.read_to_string(&mut output);
        Some(output)
    } else {
        None
    };

    let _ = report_tx.send(ExecutionUpdate::Finish(ExecutionReport {
        command_number,
        exit_code: exit_code::get_exit_code(status),
        stdout,
        stderr,
    }));
}
