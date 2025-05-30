use crate::{
    args::{Args, FILE_SUBSTITUTION, FILES_SUBSTITUTION},
    command::{execution_report::ExecutionUpdate, exit_code::get_exit_code_string},
};
use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::time::Duration;

// static PROGRAM_VERSION: &str = env!("CARGO_PKG_VERSION");
pub static PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
const DEFAULT_TICK_DURATION_MS: u64 = 100;
// Homm can't decide
// const TICK_STRINGS: [&str; 8] = ["▰▱▱▱▱▱▱", "▰▰▱▱▱▱▱", "▰▰▰▱▱▱▱", "▰▰▰▰▱▱▱", "▰▰▰▰▰▱▱", "▰▰▰▰▰▰▱", "▰▰▰▰▰▰▰", "▰▱▱▱▱▱▱"];
// const TICK_STRINGS: [&str; 4] = ["▹▹▹", "▸▹▹", "▹▸▹", "▹▹▸"];
const TICK_STRINGS: [&str; 8] = ["⢹", "⢺", "⢼", "⣸", "⣇", "⡧", "⡗", "⡏"];

/// Helper to manage the output on the screen while
/// the programm is running
pub struct Output {
    /// MultiProgress handle
    multi: MultiProgress,
    /// Keeping track of the progress bar handles here
    progress_bars: HashMap<usize, ProgressBar>,
    /// Keeping track of the list of files for each progress bar
    file_list_cache: HashMap<usize, String>,
}

impl Output {
    /// Creates a new instance
    pub fn new() -> Self {
        Self {
            multi: MultiProgress::new(),
            progress_bars: HashMap::new(),
            file_list_cache: HashMap::new(),
        }
    }

    /// Prints the top level title
    pub fn print_title(&self, args: &Args) {
        let mut command = args.command.join(" ");
        for s in &[FILES_SUBSTITUTION, FILE_SUBSTITUTION] {
            command = command.replace(s, &s.italic().bold().to_string().as_str());
        }
        let result = self.multi.println(format!("{} | {}", PROGRAM_NAME.bold(), command.green()));

        if let Err(e) = result {
            eprintln!("Error printing title: {:?}", e);
        }
    }

    /// Updates progress bars based on an exec report
    pub fn update(&mut self, update: ExecutionUpdate) {
        match update {
            ExecutionUpdate::Start(report) => {
                let pb = self.multi.insert(report.command_number, ProgressBar::new_spinner());
                let files = report.files.join(", ");
                pb.set_style(Self::progress_bar_style());
                pb.set_prefix(
                    format!("#{}.", (report.command_number + 1)).bright_black().to_string(),
                );
                pb.set_message(format!("| {}: {}", "files".bold(), files));
                pb.enable_steady_tick(Duration::from_millis(DEFAULT_TICK_DURATION_MS));
                self.progress_bars.insert(report.command_number, pb);
                self.file_list_cache.insert(report.command_number, files);
            }
            ExecutionUpdate::Finish(report) => {
                let index = report.command_number;
                let pb = self.progress_bars.get_mut(&index).unwrap();
                let files = self.file_list_cache.get(&index).expect("No cache error");

                pb.set_style(Self::progress_bar_finished_style());
                pb.set_prefix(
                    format!("#{}. {}", (index + 1), get_exit_code_string(report.exit_code),)
                        .bright_black()
                        .to_string(),
                );
                pb.set_message(format!("| {}: {}", "files".bold(), files));

                if let Some(c) = report.exit_code {
                    if c != 0 {
                        // FIXME
                        // println!("stdout: {:?}", report.stdout);
                        // println!("stderr: {:?}", report.stderr);
                    }
                }

                pb.finish();
            }
        }
    }

    /// Returns the default / pre-configured progress style
    fn progress_bar_style() -> ProgressStyle {
        ProgressStyle::default_spinner()
            .tick_strings(&TICK_STRINGS)
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

    fn progress_bar_finished_style() -> ProgressStyle {
        ProgressStyle::default_spinner()
            .template(format!("{{prefix}} {{wide_msg}} {}", "[{elapsed}]".blue()).as_str())
            .expect("no finished template error")
    }
}
