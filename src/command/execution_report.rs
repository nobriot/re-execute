use crate::command::exit_code::ExitCode;

#[derive(Debug)]
pub enum ExecMessage {
    Start(ExecStart),
    Output(ExecOutput),
    Finish(ExecCode),
}

#[derive(Debug)]
pub struct ExecStart {
    /// ID of the command being run
    pub command_number: usize,
    /// List of files associated with the run
    pub files: Vec<String>,
}

#[derive(Debug)]
pub struct ExecOutput {
    /// ID of the command being run
    pub command_number: usize,
    /// stdout update
    pub stdout: Option<String>,
    /// stderr udpate
    pub stderr: Option<String>,
}

#[derive(Debug)]
pub struct ExecCode {
    /// ID of the command being run
    pub command_number: usize,
    /// Exit code
    pub exit_code: ExitCode,
}
