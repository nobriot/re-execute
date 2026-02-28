use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

/// Guard that enables raw mode on creation and disables it on drop,
/// ensuring the terminal is always restored even on panic.
pub struct RawModeGuard;

impl RawModeGuard {
    pub fn new() -> std::io::Result<Self> {
        enable_raw_mode()?;
        // Raw mode disables OPOST (output post-processing), which means \n no
        // longer produces \r\n. Re-enable it so indicatif output renders correctly.
        #[cfg(unix)]
        Self::enable_output_processing();
        Ok(Self)
    }

    #[cfg(unix)]
    fn enable_output_processing() {
        unsafe {
            let mut termios: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(libc::STDIN_FILENO, &mut termios) == 0 {
                termios.c_oflag |= libc::OPOST;
                let _ = libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &termios);
            }
        }
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}
