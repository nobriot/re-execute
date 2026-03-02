use crate::{
    args::{Args, FILE_SUBSTITUTION, FILES_SUBSTITUTION},
    command::{execution_report::ExecMessage, exit_code::get_exit_code_string},
};
use chrono::Local;
use colored::Colorize;
use crossterm::{ExecutableCommand, cursor, terminal};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::collections::{HashMap, VecDeque};

pub static PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
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
    /// Pending output lines awaiting a flush render cycle
    pending_output: Vec<String>,
    /// Footer help bar showing keyboard shortcuts
    help_bar: Option<ProgressBar>,
    /// Indication if the program is paused or not
    paused: bool,
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
            pending_output: Vec::new(),
            help_bar: None,
            paused: false,
        };

        output.generate_title();
        output.add_help_bar();
        output.clear_output();
        output
    }

    /// Caches an output line for redraw and queues it for the next flush.
    /// Does not render immediately — call flush_output() to render.
    pub fn println<I>(&mut self, message: I)
    where
        I: AsRef<str>,
    {
        let s = message.as_ref().to_string();
        if self.output_lines.len() >= MAX_CACHED_OUTPUT_LINES {
            self.output_lines.pop_front();
        }
        self.output_lines.push_back(s.clone());
        self.pending_output.push(s);
    }

    /// Advances every active spinner by one frame.
    /// Called from the main-thread 100 ms timer so there is no background
    /// draw thread competing with our rendering.
    pub fn tick_spinners(&mut self) {
        for cache in self.cache.values() {
            if !cache.progress_bar.is_finished() {
                cache.progress_bar.tick();
            }
        }
    }

    /// Flushes all buffered output lines to the terminal in a single render cycle.
    /// Uses suspend() so all pending lines are printed inside one
    /// clear-bars → print-all → redraw-bars pass, instead of one full redraw per
    /// line (which caused visible bar jumping at high output volumes).
    pub fn flush_output(&mut self) {
        if self.pending_output.is_empty() {
            return;
        }
        let available = self.available_output_lines();
        let lines = std::mem::take(&mut self.pending_output);
        // Only print the most-recent lines that fit above the UI.
        let start = lines.len().saturating_sub(available);
        self.multi.suspend(|| {
            for line in &lines[start..] {
                println!("{}", line);
            }
        });
    }

    /// Returns how many lines of child-process output can be displayed without
    /// overflowing into the title / progress-bar area.
    fn available_output_lines(&self) -> usize {
        let term_height = terminal::size().map(|(_, r)| r as usize).unwrap_or(24);
        // title area  : blank line + separator + title       = 3 lines
        // progress bars: up to NUMBER_OF_PB_ON_SCREEN bars   = 0..5 lines
        // help bar     : separator + help text               = 2 lines
        // buffer       : breathing room                      = 2 lines
        let bar_count = (self.cache.len().saturating_sub(1)).min(NUMBER_OF_PB_ON_SCREEN);
        let ui_lines = 3 + bar_count + 2 + 2;
        term_height.saturating_sub(ui_lines)
    }

    /// Prints the top level title with a separator line above it
    pub fn generate_title(&mut self) {
        let pb = self.multi.insert(0, ProgressBar::no_length());
        pb.set_style(Self::title_style());
        pb.set_message(format!("{}\n{}", Self::separator_line(None), self.title));
        pb.finish();
        let cache = CommandCache { progress_bar: pb, file_list: String::from(""), time: None };
        self.cache.insert(0, cache);
    }

    /// Adds the help bar at the bottom of the MultiProgress
    fn add_help_bar(&mut self) {
        let separator = Self::separator_line(None);
        let pause_or_resume = if self.paused { "resume" } else { "pause" };
        let help_text = format!(
            "  {} quit  {}  {} clear  {}  {} {}",
            "q/Ctrl-c".cyan().bold(),
            "·".bright_black(),
            "Ctrl-l".cyan().bold(),
            "·".bright_black(),
            "k".cyan().bold(),
            pause_or_resume,
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
    /// With an optional message at the beginning of the separator
    fn separator_line(message: Option<&str>) -> String {
        let term_width = terminal::size().map(|(c, _)| c as usize).unwrap_or(80);
        if let Some(m) = message {
            let message_width = unicode_width::UnicodeWidthStr::width(m);
            if term_width < message_width + 1 {
                // Message does not fit - we just skip it.
                "─".repeat(term_width).cyan().to_string()
            } else {
                // formats in cyan ─message───────
                format!(
                    "{}{}{}",
                    "─".cyan(),
                    m.cyan(),
                    "─".repeat(term_width - message_width - 1).cyan()
                )
            }
        } else {
            "─".repeat(term_width).cyan().to_string()
        }
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
        self.pending_output.clear();
        self.redraw();
    }

    /// Tells the output if the program is currently paused or not
    pub fn set_pause(&mut self, paused: bool) {
        self.paused = paused;
        self.redraw();
    }

    /// Redraws all progress bars with the current terminal width.
    /// Clears the progress bar area (plus a buffer for wrapped lines),
    /// replays cached stdout, then recreates bars at the new width.
    pub fn redraw(&mut self) {
        // Disconnect all existing bars from the old MultiProgress before replacing
        // it.  Active (non-finished) ProgressBars call abandon() on Drop, which
        // triggers a draw on the old multi.  After we replace self.multi and clear
        // the screen, those Drop-triggered draws write a ghost render of the
        // title/separator to stdout — the visible "duplicate separator" bug.
        // Setting the draw target to hidden removes each bar from the old multi's
        // tracking list and redirects any future draws to a no-op sink.
        for cache in self.cache.values() {
            cache.progress_bar.set_draw_target(ProgressDrawTarget::hidden());
        }
        if let Some(ref hb) = self.help_bar {
            hb.set_draw_target(ProgressDrawTarget::hidden());
        }

        let _ = self.multi.clear();

        // Move cursor to top-left and clear the entire visible terminal.
        let mut stdout = std::io::stdout();
        let _ = stdout.execute(cursor::MoveTo(0, 0));
        let _ = stdout.execute(terminal::Clear(terminal::ClearType::All));

        self.multi = MultiProgress::new();

        // Replay the most-recent output lines that fit above the UI.
        let available = self.available_output_lines();
        let skip = self.output_lines.len().saturating_sub(available);
        // Use one multi.println() per line so indicatif cursor tracking stays
        // correct (it assumes single-line messages for its cursor arithmetic).
        for line in self.output_lines.iter().skip(skip) {
            let _ = self.multi.println(line);
        }
        // All output_lines are now rendered; clear pending to avoid double-print.
        self.pending_output.clear();

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
                let message = if self.paused { Some("paused") } else { None };
                pb.set_message(format!("{}\n{}", Self::separator_line(message), self.title));
                pb.finish();
                pb
            } else {
                let pb = self.multi.insert(index, ProgressBar::new_spinner());
                if was_finished {
                    pb.set_style(Self::progress_bar_finished_style());
                } else {
                    pb.set_style(Self::progress_bar_style());
                    // No enable_steady_tick; tick_spinners() drives animation.
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
                // Do NOT call enable_steady_tick — that spawns a background draw thread
                // which races with our main-thread rendering.  Spinners are advanced
                // manually by tick_spinners() from the 100 ms flush timer.

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
            .template(format!("{{prefix}} {{wide_msg}} {}", "[{elapsed}] ".blue()).as_str())
            .expect("no finished template error")
    }

    fn get_local_time() -> String {
        let now = Local::now();
        now.format("%H:%M:%S").to_string()
    }
}
