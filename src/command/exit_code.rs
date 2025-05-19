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
