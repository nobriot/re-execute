use super::args::{Args, FILES_SUBSTITUTION, FILE_SUBSTITUTION};
use super::errors::ProgramErrors;
use anyhow::Result;
use log::*;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct CommandQueue {
    command: String,
    args: Vec<String>,
    /// Files that have been updated
    files: Vec<PathBuf>,
    /// Indicates if we execute the command 1 time per modified file
    single_file_execution: bool,
}

impl CommandQueue {
    pub fn new(args: &Args) -> Result<Self, ProgramErrors> {
        // FIXME: use &str instead of String
        let command_tokens = shell_words::split(&args.command);

        if let Err(e) = command_tokens {
            return Err(ProgramErrors::CommandParseError(
                args.command.clone(),
                e.to_string(),
            ));
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

        Ok(Self {
            command: command_tokens[0].clone(),
            args: command_tokens[1..].to_vec(),
            files: Vec::new(),
            single_file_execution,
        })
    }

    pub fn push(&mut self, file: &Path) {
        self.files.push(file.to_path_buf())
    }

    pub fn run() {
        todo!();
    }

    pub fn execute(&mut self) -> Result<(), ProgramErrors> {
        if self.files.is_empty() {
            //warn!("Weird, we tried to execute with empty file queue");
            return Ok(());
        }

        let p: Vec<PathBuf> = if self.single_file_execution {
            self.files.pop().into_iter().collect()
        } else {
            self.files.drain(..).collect()
        };

        debug!("Run command for {:?}", p);
        let mut command = Command::new(self.command.clone());
        for arg in &self.args {
            match arg {
                a if a == FILE_SUBSTITUTION => command.args(p.clone()),
                a if a == FILES_SUBSTITUTION => command.args(p.clone()),
                a => command.args([a]),
            };
        }

        info!("Running command: {:?}", command);
        let mut child = command.spawn()?;

        if let Some(output) = child.stdout.take() {
            println!("Command output: {:?}", output);
            // Handle CommandOutputPolicy::Pipe
            // discard(output);
        }

        let status = child.wait()?;
        println!("status:  -> {:?}", status);

        Ok(())
    }
}
