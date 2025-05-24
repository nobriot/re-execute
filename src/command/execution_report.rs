use crate::command::exit_code::ExitCode;
use std::time::Duration;

#[derive(Debug)]
pub struct ExecutionReport {
    /// Exit code
    pub exit_code: ExitCode,
    /// Execution time for the command
    pub time: Duration,
    /// Captured stdout
    pub stdout: Option<String>,
    /// Captured stderr
    pub stderr: Option<String>,
}
