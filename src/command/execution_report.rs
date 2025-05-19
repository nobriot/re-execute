use crate::command::exit_code::ExitCode;

pub struct ExecutionReport {
    /// Exit code
    pub exit_code: ExitCode,
    /// Execution time for the command
    pub time: u64,
    /// Captured stdout
    pub stdout: Option<String>,
    /// Captured stderr
    pub stderr: Option<String>,
}
