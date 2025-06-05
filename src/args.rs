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
  By default if no placeholder is present, one command will be run for all executed files"#
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
    //
    /// Current Working Directory for the
    /// command being executed. Choices
    /// are from
    /// 1. Where re-execute was invoked
    /// 2. The directory where the watch is happening
    /// 3. The directory of the file itself
    //#[arg(long)]
    //pub current_working_dir: TBD,

    /// Environment variables to set when the command is executed
    //#[arg(short, long)]
    //pub env: Vec<String>,

    /// Display the current time when running the command
    #[arg(short, long)]
    pub time: bool,

    /// Suppress child programs stdout/stderr
    #[arg(short, long)]
    pub quiet: bool,

    /// Include hidden files and directories in updated files
    #[arg(long, short = 'H')]
    pub hidden: bool,

    /// Do no respect .gitignore files.
    #[arg(short = 'I', long)]
    pub no_gitignore: bool,

    /// Invoke the command also when files are deleted and no longer exist
    #[arg(short, long)]
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
            *s = s.strip_prefix(".").unwrap_or(s).to_string();
        });

        // If no files are passed, we watch the current directory for changes
        if self.files.is_empty() {
            self.files.push(String::from("."));
        }

        // Ensure we have a command to execute
        if self.command.is_empty() {
            return Err(ProgramErrors::EmptyCommand);
        }

        // Assemble the command in 1 piece
        let command = self.command.join(" ");

        // Fill up whether we execute once or one time per file
        self.batch_exec = !command.contains(FILE_SUBSTITUTION); // self.command.iter().any(|s| s.contains(FILE_SUBSTITUTION));
        if command.contains(FILES_SUBSTITUTION) {
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

        // Just replace the command with a single string
        self.command = vec![command];

        // Fill up the default shell
        self.shell = DEFAULT_SHELL;

        //dbg!(&self);
        Ok(())
    }

    // /// Pre-parses the bits of the commands to assemble.
    // /// e.g.
    // /// input ["sleep", "10", "echo", "{file} was modified"]
    // /// returns [Literal(0, 0, 5), Space, Literal(1, 0, 2), Literal(2, 0, 4), Space, FilesPlaceholder, Literal(3, 6, 19), Space]
    // fn parse_command(command: &Vec<String>) -> Result<Vec<CommandChunk>, ProgramErrors> {
    //     let mut parsed = Vec::new();

    //     for (i, words) in command.iter().enumerate() {
    //         if words == FILE_SUBSTITUTION {
    //             parsed.push(CommandChunk::FilePlaceholder);
    //         } else if words == FILES_SUBSTITUTION {
    //             parsed.push(CommandChunk::FilesPlaceholder);
    //         } else if words.contains(FILE_SUBSTITUTION) {
    //             let mut s = 0;
    //             let e = words.len();
    //             let mut search: &str = &words[s..e];
    //             while let Some(index) = search.find(FILE_SUBSTITUTION) {
    //                 if index > i {
    //                     parsed.push(CommandChunk::Literal(i, s, index));
    //                 }
    //                 parsed.push(CommandChunk::FilesPlaceholder);

    //                 if index + FILE_SUBSTITUTION.len() < e {
    //                     s = index + FILE_SUBSTITUTION.len();
    //                     search = &words[s..];
    //                 }
    //             }
    //             parsed.push(CommandChunk::Literal(i, s, e));
    //         } else if words.contains(FILES_SUBSTITUTION) {
    //             let mut s = 0;
    //             let e = words.len();
    //             let mut search: &str = &words[s..e];
    //             while let Some(index) = search.find(FILES_SUBSTITUTION) {
    //                 if index > i {
    //                     parsed.push(CommandChunk::Literal(i, s, index));
    //                 }
    //                 parsed.push(CommandChunk::FilesPlaceholder);

    //                 if index + FILES_SUBSTITUTION.len() < e {
    //                     s = index + FILES_SUBSTITUTION.len();
    //                     search = &words[s..];
    //                 }
    //             }
    //             parsed.push(CommandChunk::Literal(i, s, e));
    //         } else {
    //             parsed.push(CommandChunk::Literal(i, 0, words.len()));
    //         }

    //         // Join the String from the Vec with spaces
    //         if i != words.len() - 1 {
    //             parsed.push(CommandChunk::Space);
    //         }
    //     }

    //     return Ok(parsed);
    // }
}

// /// Parts of the commands - parsed
// /// from the initial command received
// #[derive(Debug)]
// enum CommandChunk {
//     /// This is just a reference to a part of the String
//     /// contained in the command.
//     /// First usize is index of the Vec<String> inside the vec
//     /// Second/Third usize are start/end indices of the String[s..]
//     Literal(usize, usize, usize),
//     /// This is a space (used to join argments from the String)
//     Space,
//     /// This is the '{files}' literal
//     FilesPlaceholder,
//     /// This is the '{file}' literal
//     FilePlaceholder,
// }
