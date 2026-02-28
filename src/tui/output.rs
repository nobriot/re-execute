use crate::{
    args::{Args, FILE_SUBSTITUTION, FILES_SUBSTITUTION},
    command::{execution_report::ExecMessage, exit_code::get_exit_code_string},
};
use chrono::Local;
use colored::Colorize;
use crossterm::{ExecutableCommand, cursor, terminal};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::{HashMap, VecDeque};
use std::time::Duration;

// static PROGRAM_VERSION: &str = env!("CARGO_PKG_VERSION");
pub static PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
const DEFAULT_TICK_DURATION_MS: u64 = 100;
// const TICK_STRINGS: [&str; 8] = ["⢹", "⢺", "⢼", "⣸", "⣇", "⡧", "⡗", "⡏"];
const TICK_CHARS: &str = "⣼⣹⢻⠿⡟⣏⣧⣶ ";
const NUMBER_OF_PB_ON_SCREEN: usize = 5;
const MAX_CACHED_OUTPUT_LINES: usize = 100;

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
    /// Ring buffer of recent stdout/stderr lines for redraw
    output_lines: VecDeque<String>,
    /// Footer help bar showing keyboard shortcuts
    help_bar: Option<ProgressBar>,
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
            output_lines: VecDeque::with_capacity(MAX_CACHED_OUTPUT_LINES),
            help_bar: None,
        };

        output.generate_title();
        output.add_help_bar();
        output.clear_output();
        output
    }

    /// Prints a line at the top of the bars and caches it for redraw
    pub fn println<I>(&mut self, message: I)
    where
        I: AsRef<str>,
    {
        if self.output_lines.len() >= MAX_CACHED_OUTPUT_LINES {
            self.output_lines.pop_front();
        }
        self.output_lines.push_back(message.as_ref().to_string());

        let result = self.multi.println(message);

        if let Err(e) = result {
            eprintln!("Error printing title: {e:?}");
        }
    }

    /// Prints the top level title with a separator line above it
    pub fn generate_title(&mut self) {
        let pb = self.multi.insert(0, ProgressBar::no_length());
        pb.set_style(Self::title_style());
        pb.set_message(format!("{}\n{}", Self::separator_line(), self.title));
        pb.finish();
        let cache = CommandCache { progress_bar: pb, file_list: String::from(""), time: None };
        self.cache.insert(0, cache);
    }

    /// Adds the help bar at the bottom of the MultiProgress
    fn add_help_bar(&mut self) {
        let separator = Self::separator_line();
        let help_text = format!(
            "  {} quit  {}  {} clear",
            "q/Ctrl-c".cyan().bold(),
            "·".bright_black(),
            "Ctrl-l".cyan().bold(),
        );
        let pb = self.multi.add(ProgressBar::no_length());
        pb.set_style(
            ProgressStyle::default_bar()
                .template(&format!("{separator}\n{help_text}"))
                .expect("no help bar template error"),
        );
        pb.finish();
        self.help_bar = Some(pb);
    }

    /// Removes the help bar so new progress bars are inserted above it
    fn remove_help_bar(&mut self) {
        if let Some(pb) = self.help_bar.take() {
            self.multi.remove(&pb);
        }
    }

    /// Returns a separator line of ─ characters spanning the terminal width
    fn separator_line() -> String {
        let width = terminal::size().map(|(c, _)| c as usize).unwrap_or(80);
        "─".repeat(width).cyan().to_string()
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

    /// Clears the cached output lines and redraws the screen
    pub fn clear_output(&mut self) {
        self.output_lines.clear();
        self.redraw();
    }

    /// Redraws all progress bars with the current terminal width.
    /// Clears the progress bar area (plus a buffer for wrapped lines),
    /// replays cached stdout, then recreates bars at the new width.
    pub fn redraw(&mut self) {
        let _ = self.multi.clear();

        // Move cursor to top-left and clear the entire visible terminal.
        let mut stdout = std::io::stdout();
        let _ = stdout.execute(cursor::MoveTo(0, 0));
        let _ = stdout.execute(terminal::Clear(terminal::ClearType::All));

        self.multi = MultiProgress::new();

        // Replay cached output lines
        for line in &self.output_lines {
            let _ = self.multi.println(line);
        }

        let mut indices: Vec<usize> = self.cache.keys().cloned().collect();
        indices.sort_unstable();

        for &index in &indices {
            let old_cache = self.cache.get(&index).unwrap();
            let was_finished = old_cache.progress_bar.is_finished();
            let old_prefix = old_cache.progress_bar.prefix().to_string();
            let file_list = old_cache.file_list.clone();
            let time = old_cache.time.clone();

            let pb = if index == 0 {
                let pb = self.multi.insert(0, ProgressBar::no_length());
                pb.set_style(Self::title_style());
                pb.set_message(format!("{}\n{}", Self::separator_line(), self.title));
                pb.finish();
                pb
            } else {
                let pb = self.multi.insert(index, ProgressBar::new_spinner());
                if was_finished {
                    pb.set_style(Self::progress_bar_finished_style());
                } else {
                    pb.set_style(Self::progress_bar_style());
                    pb.enable_steady_tick(Duration::from_millis(DEFAULT_TICK_DURATION_MS));
                }
                pb.set_prefix(old_prefix);
                pb.set_message(format!("{}: {}", self.file_str.bold(), file_list));
                if was_finished {
                    pb.finish();
                }
                pb
            };

            self.cache.insert(index, CommandCache { progress_bar: pb, file_list, time });
        }

        self.add_help_bar();
    }

    /// Updates progress bars based on an exec report
    pub fn update(&mut self, update: ExecMessage) {
        match update {
            ExecMessage::Start(report) => {
                let index = report.command_number + 1;
                self.remove_old_progress_bars(index);
                self.remove_help_bar();
                let pb = self.multi.insert(index, ProgressBar::new_spinner());
                let files = report.files.join(", ");
                let time = if self.time { Some(Self::get_local_time()) } else { None };

                pb.set_style(Self::progress_bar_style());
                let prefix = if let Some(ref t) = time {
                    format!("#{}. {}", index, t)
                } else {
                    format!("#{index}.")
                };
                pb.set_prefix(prefix.bright_black().to_string());
                pb.set_message(format!("{}: {}", self.file_str.bold(), files));
                pb.enable_steady_tick(Duration::from_millis(DEFAULT_TICK_DURATION_MS));

                let c = CommandCache { progress_bar: pb, file_list: files, time };
                self.cache.insert(index, c);
                self.add_help_bar();
            }
            ExecMessage::Output(report) => {
                if self.quiet {
                    return;
                }
                // TODO: We could consider prepeding output with the command number and avoid
                // mixing them
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
                let prefix = if let Some(t) = &cache.time {
                    format!("#{}. {} {}", index, t, get_exit_code_string(report.exit_code))
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
                    "[{elapsed}] ".blue()
                )
                .as_str(),
            )
            .expect("no default template error")
    }

    /// Style for the title bar (separator + title), uses {msg} to support
    /// multi-line
    fn title_style() -> ProgressStyle {
        ProgressStyle::default_bar()
            .template("\n{msg}")
            .expect("no title template error")
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
