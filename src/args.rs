use crate::errors::ProgramErrors;
use clap::Parser;

/// Use this placeholder to substitute individual updated files in the command
pub static FILE_SUBSTITUTION: &str = "{file}";
/// Use this placeholder to substitute the list of updated files in the command
pub static FILES_SUBSTITUTION: &str = "{files}";

#[derive(Parser, Debug)]
#[command(name = "rex", max_term_width = 80)]
#[command(about = "Run commands when files are updated")]
#[command(version)]
pub struct Args {
    /// List of files to watch. Will watch everything in the current
    /// directory if not specified
    #[arg(short, long, value_name = "file")]
    pub files: Vec<String>,

    /// Command/program to run
    /// Use {file} to include the updated file as argument
    /// Use {files} to include the updated files as argument
    pub command: String,

    /// List of file extensions to watch.
    #[arg(short, long)]
    pub extensions: Vec<String>,

    /// Poll interval in ms for file updates
    #[arg(long, default_value_t = 200)]
    pub poll_interval: u64,

    /// Shell to use to run the command / program
    /// TODO
    #[arg(long)]
    pub shell: Option<String>,

    /// Suppress program's stdout
    /// TODO
    #[arg(short, long)]
    pub quiet: bool,

    /// Search hidden files and directories
    /// TODO
    #[arg(long, short = 'H')]
    pub hidden: bool,

    /// Do no respect .gitignore files
    /// TODO
    #[arg(short = 'I', long)]
    pub no_gitignore: bool,
}

impl Args {
    pub fn validate() -> Result<(), ProgramErrors> {
        todo!();
    }
}
