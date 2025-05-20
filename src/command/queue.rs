use crate::args::{Args, FILE_SUBSTITUTION, FILES_SUBSTITUTION};
use crate::command::QueueMessage;
use crate::errors::ProgramErrors;
use anyhow::Result;
use log::*;
use std::collections::HashSet;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Instant;

use super::execution_report::ExecutionReport;
use super::exit_code;

pub struct Queue {
    /// Command to execute
    command: String,
    /// Raw command arguments, may contains FILE placeholders
    args: Vec<String>,
    /// Files that have been updated - pending command execution
    files: HashSet<(PathBuf, PathBuf)>,
    /// Indicates if we execute the command 1 time per modified file
    single_file_execution: bool,
    rx: std::sync::mpsc::Receiver<QueueMessage>,
    last_update: Option<std::time::Instant>,
}

impl Queue {
    pub fn new(args: &Args) -> Result<std::sync::mpsc::Sender<QueueMessage>, ProgramErrors> {
        // TODO: use &str instead of String
        let command_tokens = shell_words::split(&args.command);

        if let Err(e) = command_tokens {
            return Err(ProgramErrors::CommandParseError(args.command.clone(), e.to_string()));
        }
        let command_tokens = command_tokens.unwrap();
        if command_tokens.is_empty() {
            return Err(ProgramErrors::CommandParseError(
                args.command.clone(),
                String::from("Empty command"),
            ));
        }

        let single_file_execution = command_tokens[1..].iter().any(|s| s == FILE_SUBSTITUTION);
        if single_file_execution && command_tokens[1..].iter().any(|s| s == FILES_SUBSTITUTION) {
            return Err(ProgramErrors::CommandParseError(
                args.command.clone(),
                format!("Command cannot contain both {FILE_SUBSTITUTION} and {FILES_SUBSTITUTION}"),
            ));
        }

        let (tx, rx) = std::sync::mpsc::channel();
        let mut queue = Self {
            command: command_tokens[0].clone(),
            args: command_tokens[1..].to_vec(),
            files: HashSet::new(),
            single_file_execution,
            rx,
            last_update: None,
        };

        std::thread::spawn(move || queue.run());
        Ok(tx)
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
                    debug!("Adding file: {:?} / Path: {:?}", p, watch);
                    let _ = self.files.insert((p, watch));
                    self.last_update = Some(std::time::Instant::now());
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    //FIXME: This is busy looping
                }
                Err(e) => {
                    warn!("Channel error: {:?}", e);
                    break;
                }
            }
            if let Some(t) = self.last_update {
                //debug!("elapsed: {:?}", t.elapsed());
                if t.elapsed() > std::time::Duration::from_millis(200) {
                    let exec_result = self.execute();
                    if exec_result.is_err() {
                        eprintln!("Error with the queue trying to execute commands");
                        break;
                    }
                    if self.files.is_empty() {
                        self.last_update = None;
                    }
                }
            }
        }
    }

    pub fn execute(&mut self) -> Result<ExecutionReport, ProgramErrors> {
        if self.files.is_empty() {
            return Err(ProgramErrors::BadInternalState);
        }

        // Choose arguments based on the placeholders
        let p: Vec<PathBuf> = if self.single_file_execution {
            let paths = self.files.iter().next().unwrap().clone();
            self.files.remove(&paths);
            vec![paths.0]
        } else {
            self.files.drain().map(|(p, _)| p).collect()
        };

        let mut command = Command::new(self.command.clone());
        for arg in &self.args {
            match arg {
                a if a == FILE_SUBSTITUTION => command.args(p.clone()),
                a if a == FILES_SUBSTITUTION => command.args(p.clone()),
                a => command.args([a]),
            };
        }
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        info!("Running command: '{:?}'", command);

        let start = Instant::now();
        let mut child = command.spawn()?;
        let status = child.wait()?;

        let stdout = if let Some(mut stdout) = child.stdout.take() {
            let mut output = String::new();
            let _ = stdout.read_to_string(&mut output);
            //println!("Command output: {:?}", output);
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

        Ok(ExecutionReport {
            exit_code: exit_code::get_exit_code(status),
            time: start.elapsed(),
            stdout,
            stderr,
        })
    }
}
