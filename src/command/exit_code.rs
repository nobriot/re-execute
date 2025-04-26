use std::process::ExitStatus;

#[cfg(unix)]
pub fn get_exit_code(status: ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.code().or_else(|| status.signal().map(|s| s + 128))
}

#[cfg(not(unix))]
pub fn get_exit_code(status: ExitStatus) -> Option<i32> {
    status.code()
}
