use colored::Colorize;
use std::process::ExitStatus;

pub type ExitCode = Option<i32>;

#[cfg(unix)]
pub fn get_exit_code(status: ExitStatus) -> ExitCode {
    use std::os::unix::process::ExitStatusExt;
    status.code().or_else(|| status.signal().map(|s| s + 128))
}

#[cfg(not(unix))]
pub fn get_exit_code(status: ExitStatus) -> ExitCode {
    status.code()
}

pub fn get_exit_code_string(exit_code: ExitCode) -> String {
    if let Some(c) = exit_code {
        match c {
            0 => format!("{:<3}", "0".green()),
            130 => format!("{:<3}", "130".yellow()),
            c => format!("{:<3}", c).red().to_string(),
        }
    } else {
        format!("{:<3}", "?? ".bold().bright_yellow())
    }
}
