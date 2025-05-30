use crate::errors::ProgramErrors;
use clap::Parser;

/// Use this placeholder to substitute individual updated files in the command
pub static FILE_SUBSTITUTION: &str = "{file}";
/// Use this placeholder to substitute the list of updated files in the command
pub static FILES_SUBSTITUTION: &str = "{files}";

#[cfg(not(windows))]
pub const DEFAULT_SHELL: &str = "sh -c";

#[cfg(windows)]
pub const DEFAULT_SHELL: &str = "cmd.exe";

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"), max_term_width = 80)]
#[command(about = "Run commands when files are updated")]
#[command(version)]
pub struct Args {
    /// List of files or directories to watch. Will watch everything in the
    /// current directory if none is specified
    #[arg(short, long = "file", name = "file/dir")]
    pub files: Vec<String>,

    /// Command/program to run
    #[arg(
        trailing_var_arg = true,
        help = "Command/program to run",
        long_help = r#"Command/program to run
Placeholders:
  Use {file} to substitue the updated file in the command
  Use {files} to substitue all updated files in the command
  By default if no context is present, one command will be run for all executed files"#
    )]
    pub command: Vec<String>,

    /// List of file extensions to watch.
    #[arg(short, long = "extension", name = "extension")]
    pub extensions: Vec<String>,

    /// Poll interval in ms for file updates
    #[arg(long, default_value_t = 200)]
    pub poll_interval: u64,

    //// Regex to match files against
    // TODO: Implement me
    // #[arg(short, long)]
    // pub regex: Vec<String>,

    //
    /// Suppress child programs stdout/stderr
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

    /// Indicates if we abort previous ongoing commands
    /// Happens only by default if no substitution is specified
    #[arg(short, long)]
    pub abort_previous: bool,

    /// Shell used to spawn the command
    /// Not possible to specify manually for now
    #[clap(skip)]
    pub shell: &'static str,

    /// Indicates is we batch execute, i.e. 1 exec for all modified files
    /// or if it is one execution per modified file
    #[clap(skip)]
    pub batch_exec: bool,
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

        // If no files are passed, we watch the current directory for changes
        if self.files.is_empty() {
            self.files.push(String::from("."))
        }

        // Ensure we have a command to execute
        if self.command.is_empty() {
            return Err(ProgramErrors::EmptyCommand);
        }

        // Fill up whether we execute once or one time per file
        self.batch_exec = !self.command.iter().any(|s| s.contains(FILE_SUBSTITUTION));
        if self.command.iter().any(|s| s.contains(FILES_SUBSTITUTION)) {
            if !self.batch_exec {
                // If substitutions are used, it's only single files or all files
                return Err(ProgramErrors::CommandParseError(
                    self.command.join(" "),
                    format!(
                        "Command cannot contain both {FILE_SUBSTITUTION} and {FILES_SUBSTITUTION}"
                    ),
                ));
            }
        } else if self.batch_exec {
            self.abort_previous = true;
        }

        self.shell = DEFAULT_SHELL;

        //dbg!(&self);
        Ok(())
    }
}
