use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

/// Sets up logging for the application.
///
/// When `log_file` is provided, log lines are written to the file with
/// timestamps. When no log file is given, logging is silently disabled.
pub fn setup(log_file: Option<&Path>) {
    let file =
        log_file.and_then(|path| match OpenOptions::new().create(true).append(true).open(path) {
            Ok(f) => Some(Mutex::new(f)),
            Err(e) => {
                eprintln!("Warning: cannot open log file {}: {e}", path.display());
                None
            }
        });

    // No log file → nothing to do
    if file.is_none() {
        return;
    }

    let mut log_builder =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));

    // Write only to the log file, never to stderr.
    log_builder.target(env_logger::Target::Pipe(Box::new(std::io::sink())));

    log_builder.format(move |buf, record| {
        let level = record.level();

        if let Some(ref file_mutex) = file
            && let Ok(mut f) = file_mutex.lock()
        {
            let now = chrono::Local::now();
            let _ =
                writeln!(f, "{} [{}] {}", now.format("%Y-%m-%d %H:%M:%S"), level, record.args());
        }

        // Write nothing to the default target (sink)
        writeln!(buf)
    });

    log_builder.init();
}
