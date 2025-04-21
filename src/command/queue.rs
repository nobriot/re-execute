use crate::args::{Args, FILES_SUBSTITUTION, FILE_SUBSTITUTION};
use crate::command::QueueMessage;
use crate::errors::ProgramErrors;
use anyhow::Result;
use log::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Queue {
    /// Command to execute
    command: String,
    /// Raw command arguments, may contains FILE placeholders
    args: Vec<String>,
    /// Files that have been updated - pending command execution
    files: HashSet<PathBuf>,
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
                Ok(QueueMessage::RestartBackoff) => todo!(),
                Ok(QueueMessage::AddFile(p)) => {
                    debug!("Adding file: {:?}", p);
                    let _ = self.files.insert(p);
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
                    // TODO: Process result
                    let _ = self.execute();
                    if self.files.is_empty() {
                        self.last_update = None;
                    }
                }
            }
        }
    }

    pub fn execute(&mut self) -> Result<(), ProgramErrors> {
        if self.files.is_empty() {
            warn!("We screwed up, and tried to execute with empty file queue");
            return Ok(());
        }

        let p: Vec<PathBuf> = if self.single_file_execution {
            let path = self.files.iter().next().unwrap().clone();
            self.files.remove(&path);
            vec![path]
        } else {
            self.files.drain().collect()
        };

        let mut command = Command::new(self.command.clone());
        for arg in &self.args {
            match arg {
                a if a == FILE_SUBSTITUTION => command.args(p.clone()),
                a if a == FILES_SUBSTITUTION => command.args(p.clone()),
                a => command.args([a]),
            };
        }

        info!("Running command: '{:?}'", command);
        let mut child = command.spawn()?;

        let status = child.wait()?;

        if let Some(output) = child.stdout.take() {
            println!("Command output: {:?}", output);
            // Handle CommandOutputPolicy::Pipe
            // discard(output);
        }
        println!("status:  -> {:?}", status);

        Ok(())
    }
}
