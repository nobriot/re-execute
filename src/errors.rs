use thiserror::Error;

macro_rules! runtime_error {
    ($err: ident) => {
        ProgramError::RuntimeError(RuntimeError::$err)
    };
    ($err: ident, $($e:expr),*) => {
        ProgramError::RuntimeError(RuntimeError::$err($($e),*))
    };
}
macro_rules! arg_error {
    ($err: ident) => {
        ProgramError::ArgumentError(ArgumentError::$err)
    };
    ($err: ident, $($e:expr),*) => {
        ProgramError::ArgumentError(ArgumentError::$err($($e),*))
    };
}
pub(crate) use arg_error;
pub(crate) use runtime_error;

#[derive(Error, Debug)]
pub enum ProgramError {
    #[error("Argument error: {0}")]
    ArgumentError(#[from] ArgumentError),

    #[error("Runtime error: {0}")]
    RuntimeError(#[from] RuntimeError),
}

impl std::convert::From<std::io::Error> for ProgramError {
    fn from(value: std::io::Error) -> Self {
        Self::RuntimeError(RuntimeError::CommandExecutionError(value.to_string()))
    }
}

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Error watching file: {0}")]
    FileWatchError(String),

    #[error("File error: {0} {1}")]
    FileError(String, String),

    #[error("Failed to execute command: {0}")]
    CommandExecutionError(String),

    #[error("Internal Error: {0}")]
    InternalError(String),

    #[error("Channel Error: {0}")]
    ChannelReceiveError(String),
}

#[derive(Error, Debug)]
pub enum ArgumentError {
    #[error("Failed to parse command: {0} - {1}")]
    CommandParseError(String, String),

    #[error("Invalid environment variable: {0}")]
    InvalidEnvironmentVariable(String),

    #[error("Invalid current working directory: {0}")]
    InvalidCurrentWorkingDirectory(String),

    #[error("Invalid regular expression: {0} {1}")]
    InvalidRegex(String, String),

    #[error("Command to execute is empty")]
    EmptyCommand,
}
