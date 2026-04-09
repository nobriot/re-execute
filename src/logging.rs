use std::fs::OpenOptions;
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::sync::Mutex;

/// Sets up logging for the application.
///
/// When `log_file` is provided, log lines are written to both stderr and the
/// file (with timestamps, since the file has no journald to provide them).
/// When running in a terminal, stderr gets colored output with timestamps.
/// Under systemd (no terminal), stderr omits timestamps (journald adds its
/// own).
pub fn setup(log_file: Option<&Path>) {
    let file =
        log_file.and_then(|path| match OpenOptions::new().create(true).append(true).open(path) {
            Ok(f) => {
                eprintln!("Logging to {}", path.display());
                Some(Mutex::new(f))
            }
            Err(e) => {
                eprintln!("Warning: cannot open log file {}: {e}", path.display());
                None
            }
        });

    let is_terminal = io::stderr().is_terminal();

    let mut log_builder =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));

    log_builder.format(move |buf, record| {
        let level = record.level();
        let module = record.module_path().unwrap_or("");

        if is_terminal {
            let now = chrono::Local::now();
            let style = buf.default_level_style(level);
            writeln!(
                buf,
                "{} [{style}{}{style:#} {}] {}",
                now.format("%Y/%m/%d %H:%M"),
                level,
                module,
                record.args()
            )?;
        } else {
            writeln!(buf, "[{} {}] {}", level, module, record.args())?;
        }

        // Append to log file (with timestamp, since the file has none)
        if let Some(ref file_mutex) = file
            && let Ok(mut f) = file_mutex.lock()
        {
            let now = chrono::Local::now();
            let _ =
                writeln!(f, "{} [{}] {}", now.format("%Y-%m-%d %H:%M:%S"), level, record.args());
        }

        Ok(())
    });

    log_builder.init();
}
