use crate::errors::ProgramErrors;
use clap::Parser;

/// Use this placeholder to substitute individual updated files in the command
pub static FILE_SUBSTITUTION: &str = "{file}";
/// Use this placeholder to add the extension of the updated file
pub static FILE_EXT_SUBSTITUTION: &str = "{file-ext}";
/// Use this placeholder to add the basename of the updated file
pub static FILE_BASENAME_SUBSTITUTION: &str = "{file-basename}";
/// Use this placeholder to substitute the list of updated files in the command
pub static FILES_SUBSTITUTION: &str = "{files}";
/// Use this placeholder to add the extension of the updated file
pub static FILES_EXT_SUBSTITUTION: &str = "{files-ext}";
/// Use this placeholder to add the basename of the updated files
pub static FILES_BASENAME_SUBSTITUTION: &str = "{files-basename}";

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
    /// Running one command per updated file:
    /// Use {file} to include the updated file as argument
    /// Use {file-basename} to include the basename of the file as argument
    /// Running one command for all updated files:
    /// Use {files-basename} to include the updated files as argument
    /// Use {files} to include the updated files as argument
    pub command: String,

    /// List of file extensions to watch.
    #[arg(short, long)]
    pub extensions: Vec<String>,

    /// Poll interval in ms for file updates
    #[arg(long, default_value_t = 200)]
    pub poll_interval: u64,

    /// Suppress program's stdout
    /// TODO
    #[arg(short, long)]
    pub quiet: bool,

    /// Search hidden files and directories
    #[arg(long, short = 'H')]
    pub hidden: bool,

    /// Do no respect .gitignore files
    #[arg(short = 'I', long)]
    pub no_gitignore: bool,

    /// Invoke the command also when files are deleted and no longer exist
    #[arg(long)]
    pub deleted: bool,
}

impl Args {
    pub fn validate(&mut self) -> Result<(), ProgramErrors> {
        // Remove all trailings dots if the user has given extensions with
        // `.txt` instead of `txt`
        // Also convert all extensions to lowercase to compare
        self.extensions.iter_mut().for_each(|s| {
            *s = s.to_lowercase();
            *s = s.strip_prefix(".").unwrap_or(s).to_string()
        });
        println!("Extensions: {:?}", self.extensions);

        // If no files are passed, we watch the current directory for changes
        if self.files.is_empty() {
            self.files.push(String::from("."))
        }

        Ok(())
    }
}
