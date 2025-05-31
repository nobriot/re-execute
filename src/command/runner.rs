use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::mpsc::Sender;

use crate::command::execution_report::{ExecCode, ExecMessage, ExecStart};
use crate::command::exit_code;

use super::execution_report::ExecOutput;

pub struct Runner {}
//
// hmm

struct CommandRun {
    command_number: usize,
    command: Command,
    report_tx: Sender<ExecMessage>,
}

impl CommandRun {
    pub fn new(command_number: usize, command: Command, report_tx: Sender<ExecMessage>) -> Self {
        Self { command_number, command, report_tx }
    }

    pub fn run_command(&mut self) {
        let mut child = self.command.spawn().expect("Command could not start");

        // Send stdout updates to tx reports
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let stdout_tx = self.report_tx.clone();
        let command_number = self.command_number;
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
        let stderr_tx = self.report_tx.clone();
        let command_number = self.command_number;
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

        let status = child.wait().expect("command could not finish");
        stdout_handle.join().unwrap();
        stderr_handle.join().unwrap();

        let _ = self.report_tx.send(ExecMessage::Finish(ExecCode {
            command_number: self.command_number,
            exit_code: exit_code::get_exit_code(status),
        }));
    }
}
