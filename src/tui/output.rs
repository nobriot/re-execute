use crate::{
    args::{Args, FILE_SUBSTITUTION, FILES_SUBSTITUTION},
    command::{execution_report::ExecMessage, exit_code::get_exit_code_string},
};
use chrono::Local;
use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::time::Duration;

// static PROGRAM_VERSION: &str = env!("CARGO_PKG_VERSION");
pub static PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
const DEFAULT_TICK_DURATION_MS: u64 = 100;
// const TICK_STRINGS: [&str; 8] = ["⢹", "⢺", "⢼", "⣸", "⣇", "⡧", "⡗", "⡏"];
const TICK_CHARS: &str = "⣼⣹⢻⠿⡟⣏⣧⣶ ";
const NUMBER_OF_PB_ON_SCREEN: usize = 5;

/// Information saved for each command / progress bar
struct CommandCache {
    pub progress_bar: ProgressBar,
    pub file_list: String,
    pub time: Option<String>,
}

/// Helper to manage the output on the screen while
/// the programm is running
pub struct Output {
    /// Top level title
    title: String,
    /// MultiProgress handle
    multi: MultiProgress,
    /// Caching information associated with each command
    cache: HashMap<usize, CommandCache>,
    /// Whether we print programs' output or not
    quiet: bool,
    /// Whether we print the time at each command execution
    time: bool,
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
            cache: HashMap::new(),
            quiet: args.quiet,
            time: args.time,
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
            eprintln!("Error printing title: {e:?}");
        }
    }

    /// Prints the top level title
    pub fn print_title(&mut self) {
        let pb = self.multi.insert(0, ProgressBar::no_length());
        pb.set_style(Self::progress_bar_plain_style());
        pb.set_message(self.title.clone());
        pb.finish();
        let cache = CommandCache { progress_bar: pb, file_list: String::from(""), time: None };
        self.cache.insert(0, cache);
    }

    /// Checks the index of the last progress bar and remove old
    /// progress bar that should not be on screen anymore
    pub fn remove_old_progress_bars(&mut self, last_index: usize) {
        if last_index <= NUMBER_OF_PB_ON_SCREEN {
            return;
        }
        let pop_index = last_index - NUMBER_OF_PB_ON_SCREEN;
        let pop_pb = self.cache.remove(&pop_index);

        if pop_pb.is_none() {
            return;
        }
        let pop_pb = pop_pb.unwrap().progress_bar;
        self.multi.remove(&pop_pb);
    }

    /// Finishes all the progres bars
    pub fn finish(&mut self) {
        for c in self.cache.values() {
            c.progress_bar.finish();
        }
    }

    /// Redraws active progress bars
    pub fn redraw(&mut self) {
        for c in self.cache.values() {
            c.progress_bar.tick();
        }
    }

    /// Updates progress bars based on an exec report
    pub fn update(&mut self, update: ExecMessage) {
        match update {
            ExecMessage::Start(report) => {
                let index = report.command_number + 1;
                self.remove_old_progress_bars(index);
                let pb = self.multi.insert(index, ProgressBar::new_spinner());
                let files = report.files.join(", ");
                let time = if self.time { Some(Self::get_local_time()) } else { None };

                pb.set_style(Self::progress_bar_style());
                let prefix = if time.is_some() {
                    format!("#{}. {}", index, time.as_ref().unwrap())
                } else {
                    format!("#{index}.")
                };
                pb.set_prefix(prefix.bright_black().to_string());
                pb.set_message(format!("{}: {}", self.file_str.bold(), files));
                pb.enable_steady_tick(Duration::from_millis(DEFAULT_TICK_DURATION_MS));

                let c = CommandCache { progress_bar: pb, file_list: files, time };
                self.cache.insert(index, c);
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
                let cache = self.cache.get_mut(&index);
                // If progress bar disappeared (due to scrolling), we just ignore the update
                if cache.is_none() {
                    return;
                }
                let cache = cache.unwrap();
                let pb = &cache.progress_bar;

                pb.set_style(Self::progress_bar_finished_style());
                let prefix = if cache.time.is_some() {
                    format!(
                        "#{}. {} {}",
                        index,
                        cache.time.as_ref().unwrap(),
                        get_exit_code_string(report.exit_code)
                    )
                } else {
                    format!("#{}. {}", index, get_exit_code_string(report.exit_code))
                };
                pb.set_prefix(prefix.bright_black().to_string());
                pb.set_message(format!("{}: {}", self.file_str.bold(), cache.file_list));
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
            .template("{wide_msg}")
            .expect("no default template error")
    }

    /// Style for finished progress bars
    fn progress_bar_finished_style() -> ProgressStyle {
        ProgressStyle::default_spinner()
            .template(format!("{{prefix}} {{wide_msg}} {}", "[{elapsed}]".blue()).as_str())
            .expect("no finished template error")
    }

    fn get_local_time() -> String {
        let now = Local::now();
        now.format("%H:%M:%S").to_string()
    }
}
