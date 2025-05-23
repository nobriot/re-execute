use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use log::*;
use notify::*;
use std::path::{PathBuf, absolute};
use std::sync::mpsc::TryRecvError;
use std::time::Duration;

// static PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
// static PROGRAM_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod args;
use args::Args;

pub mod errors;
use errors::ProgramErrors;

pub mod files;
use files::utils::should_be_ignored;

pub mod command;
use command::Queue;
use command::QueueMessage;

fn main() {
    match run() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("run-on-change {} {:?}", "error".red(), e);
            std::process::exit(1);
        }
    }
}

fn run() -> Result<ProgramErrors> {
    // Logging for debug
    env_logger::builder().format_timestamp_millis().init();

    let mut args = Args::parse();
    args.validate()?;
    debug!("We received {:?}", args);
    let args = args;

    // Stores tuples (watcher, rx, top-level file)
    let mut file_watchers = Vec::new();

    for f in &args.files {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher: Box<dyn Watcher> =
            if RecommendedWatcher::kind() == WatcherKind::PollWatcher {
                let config =
                    Config::default().with_poll_interval(Duration::from_millis(args.poll_interval));
                Box::new(PollWatcher::new(tx, config).unwrap())
            } else {
                Box::new(RecommendedWatcher::new(tx, Config::default()).unwrap())
            };

        let p = register_watch_for_file(&mut watcher, f)?;
        file_watchers.push((watcher, rx, p));
    }

    let command_queue_tx = Queue::new(&args);

    // Watch event loop
    loop {
        for (_, rx, watch) in &file_watchers {
            match rx.try_recv() {
                Ok(event) if event.is_ok() => {
                    let event = event.unwrap();
                    match event.kind {
                        EventKind::Modify(_) | EventKind::Remove(_) => {
                            // debug!("File modified: {:?}", event.paths);
                            for p in &event.paths {
                                if should_be_ignored(p, &args, watch) {
                                    debug!("Ignoring update for {:?}", p);
                                    continue;
                                }

                                command_queue_tx
                                    .send(QueueMessage::AddFile(p.clone(), watch.clone()))?;
                            }
                        }
                        _ => {}
                    }
                }
                Ok(event) if event.is_err() => {
                    error!("Watch file error: {:?}", event);
                }
                Err(TryRecvError::Empty) => {
                    continue;
                }
                Err(error) => return Err(ProgramErrors::FileWatchError(error.to_string()).into()),
                _ => {}
            }
        }
        // Make sure not to busy loop
        std::thread::yield_now();
    }
}

/// Updates the watcher to watch the file pointed by &str, if it exists
/// Returns a Result with the PathBuf
fn register_watch_for_file(
    watcher: &mut Box<dyn Watcher>,
    file: &str,
) -> Result<PathBuf, ProgramErrors> {
    let p = absolute(file)
        .map_err(|e| ProgramErrors::FileError(file.to_string(), e.to_string()))?
        .canonicalize()
        .map_err(|e| ProgramErrors::FileError(file.to_string(), e.to_string()))?;

    let watch_mode =
        if p.is_dir() { RecursiveMode::Recursive } else { RecursiveMode::NonRecursive };

    // Check the files we have to monitor
    // Register a watch on the parent it is a file. (see explanation in
    // Watcher.watch)
    //
    // On some platforms, if the `path` is renamed or removed while being watched,
    // behaviour may be unexpected. See discussions in [\#165](https://github.com/notify-rs/notify/issues/165) and [\#166](https://github.com/notify-rs/notify/issues/166). If less surprising behaviour is wanted
    // one may non-recursively watch the *parent* directory as well and manage
    // related events.
    let watch_target = if p.is_dir() {
        p.clone()
    } else {
        p.parent().expect("Could not find parent dir for p").to_path_buf()
    };

    info!("Registering a {:?} watch for {:?}", watch_mode, watch_target.as_path());
    watcher.watch(watch_target.as_path(), watch_mode).unwrap();

    Ok(p)
}
