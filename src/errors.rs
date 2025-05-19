use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProgramErrors {
    #[error("Error watching file: {0}")]
    FileWatchError(String),

    #[error("File error: {0} {1}")]
    FileError(String, String),

    #[error("Failed to parse command: {0} error: {1}")]
    CommandParseError(String, String),

    #[error("Failed to execute command: {0}")]
    CommandExecutionError(String),
}

impl std::convert::From<std::io::Error> for ProgramErrors {
    fn from(value: std::io::Error) -> Self {
        Self::CommandExecutionError(value.to_string())
    }
}
