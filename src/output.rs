use crate::{
    args::{Args, FILE_SUBSTITUTION, FILES_SUBSTITUTION},
    command::{execution_report::ExecMessage, exit_code::get_exit_code_string},
};
use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::time::Duration;

// static PROGRAM_VERSION: &str = env!("CARGO_PKG_VERSION");
pub static PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
const DEFAULT_TICK_DURATION_MS: u64 = 100;
// const TICK_STRINGS: [&str; 8] = ["⢹", "⢺", "⢼", "⣸", "⣇", "⡧", "⡗", "⡏"];
const TICK_CHARS: &str = "⣼⣹⢻⠿⡟⣏⣧⣶ ";
const NUMBER_OF_PB_ON_SCREEN: usize = 10;

/// Helper to manage the output on the screen while
/// the programm is running
pub struct Output {
    /// Top level title
    title: String,
    /// MultiProgress handle
    multi: MultiProgress,
    /// Keeping track of the progress bar handles here
    progress_bars: HashMap<usize, ProgressBar>,
    /// Keeping track of the list of files for each progress bar
    file_list_cache: HashMap<usize, String>,
    /// Whether we print programs' output or not
    quiet: bool,
    /// Are we printing "files" or "file"
    file_str: &'static str,
}

impl Output {
    /// Creates a new instance
    pub fn new(args: &Args) -> Self {
        let mut command = args.command.join(" ");
        for s in &[FILES_SUBSTITUTION, FILE_SUBSTITUTION] {
            command = command.replace(s, s.italic().bold().to_string().as_str());
        }
        let title = format!("{} | {}", PROGRAM_NAME.bold(), command.green());

        let mut output = Self {
            title,
            multi: MultiProgress::new(),
            progress_bars: HashMap::new(),
            file_list_cache: HashMap::new(),
            quiet: args.quiet,
            file_str: if args.batch_exec { "files" } else { "file" },
        };

        output.print_title();
        output
    }

    /// Prints a line at the top of the bars
    pub fn println<I>(&self, message: I)
    where
        I: AsRef<str>,
    {
        let result = self.multi.println(message);

        if let Err(e) = result {
            eprintln!("Error printing title: {:?}", e);
        }
    }

    pub fn print_title(&mut self) {
        let pb = self.multi.insert(0, ProgressBar::no_length());
        pb.set_style(Self::progress_bar_plain_style());
        pb.set_message(self.title.clone());
        pb.finish();
        self.progress_bars.insert(0, pb);
    }

    /// Checks the index of the last progress bar and remove old
    /// progress bar that should not be on screen anymore
    pub fn remove_old_progress_bars(&mut self, last_index: usize) {
        if last_index <= NUMBER_OF_PB_ON_SCREEN {
            return;
        }
        let pop_index = last_index - NUMBER_OF_PB_ON_SCREEN;
        let pop_pb = self.progress_bars.remove(&pop_index);

        if pop_pb.is_none() {
            return;
        }
        let pop_pb = pop_pb.unwrap();
        self.multi.remove(&pop_pb);
    }

    /// Updates progress bars based on an exec report
    pub fn update(&mut self, update: ExecMessage) {
        match update {
            ExecMessage::Start(report) => {
                let index = report.command_number + 1;
                self.remove_old_progress_bars(index);
                let pb = self.multi.insert(index, ProgressBar::new_spinner());
                let files = report.files.join(", ");
                pb.set_style(Self::progress_bar_style());
                pb.set_prefix(format!("#{}.", index).bright_black().to_string());
                pb.set_message(format!("{}: {}", self.file_str.bold(), files));
                pb.enable_steady_tick(Duration::from_millis(DEFAULT_TICK_DURATION_MS));
                self.progress_bars.insert(index, pb);
                self.file_list_cache.insert(index, files);
            }
            ExecMessage::Output(report) => {
                if self.quiet {
                    return;
                }
                // TODO: We could consider prepeding output with the command number and avoid mixing them
                if let Some(stdout) = report.stdout {
                    self.println(stdout);
                }
                if let Some(stderr) = report.stderr {
                    self.println(stderr);
                }
            }
            ExecMessage::Finish(report) => {
                let index = report.command_number + 1;
                let pb = self.progress_bars.get_mut(&index).unwrap();
                let files = self.file_list_cache.get(&index).expect("No cache error");

                pb.set_style(Self::progress_bar_finished_style());
                pb.set_prefix(
                    format!("#{}. {}", index, get_exit_code_string(report.exit_code))
                        .bright_black()
                        .to_string(),
                );
                pb.set_message(format!("{}: {}", self.file_str.bold(), files));
                pb.finish();
            }
        }
    }

    /// Returns the default / pre-configured progress style
    fn progress_bar_style() -> ProgressStyle {
        ProgressStyle::default_spinner()
            //.tick_strings(&TICK_STRINGS)
            .tick_chars(TICK_CHARS)
            .template(
                format!(
                    "{{prefix}} {}   {{wide_msg}} {}",
                    "{spinner}".magenta(),
                    "[{elapsed}]".blue()
                )
                .as_str(),
            )
            .expect("no default template error")
    }

    /// Plain progress bar, used to print lines basically
    fn progress_bar_plain_style() -> ProgressStyle {
        ProgressStyle::default_bar()
            .template("{msg}")
            .expect("no default template error")
    }

    /// Style for finished progress bars
    fn progress_bar_finished_style() -> ProgressStyle {
        ProgressStyle::default_spinner()
            .template(format!("{{prefix}} {{wide_msg}} {}", "[{elapsed}]".blue()).as_str())
            .expect("no finished template error")
    }
}
