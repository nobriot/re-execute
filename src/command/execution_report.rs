use crate::command::exit_code::ExitCode;

#[derive(Debug)]
pub enum ExecutionUpdate {
    Start(ExecutionStart),
    Finish(ExecutionReport),
}

#[derive(Debug)]
pub struct ExecutionStart {
    /// ID of the command being run
    pub command_number: usize,
    /// List of files associated with the run
    pub files: Vec<String>,
}

#[derive(Debug)]
pub struct ExecutionOutput {
    /// ID of the command being run
    pub command_number: usize,
    /// stdout update
    pub stdout: Option<String>,
    /// stderr udpate
    pub stderr: Option<String>,
}

#[derive(Debug)]
pub struct ExecutionReport {
    /// ID of the command being run
    pub command_number: usize,
    /// Exit code
    pub exit_code: ExitCode,
    /// Captured stdout
    pub stdout: Option<String>,
    /// Captured stderr
    pub stderr: Option<String>,
}
