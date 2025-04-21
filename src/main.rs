use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use log::*;
use notify::*;
use std::path::{self, PathBuf};
use std::time::Duration;

// static PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
// static PROGRAM_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod args;
use args::Args;

pub mod errors;
use errors::ProgramErrors;

pub mod command;
use command::Queue;
use command::QueueMessage;

macro_rules! is_some_or_return {
    ($opt:expr, $ret:expr) => {
        if !$opt.is_some() {
            return $ret;
        }
    };
}

macro_rules! is_ok_or_return {
    ($res:expr, $ret:expr) => {
        if !$res.is_ok() {
            return $ret;
        }
    };
}

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

    let args = Args::parse();
    debug!("We received {:?}", args);

    let files = if args.files.is_empty() {
        vec![String::from(".")]
    } else {
        // TODO: Remove clone
        args.files.clone()
    };

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher: Box<dyn Watcher> = if RecommendedWatcher::kind() == WatcherKind::PollWatcher {
        let config =
            Config::default().with_poll_interval(Duration::from_millis(args.poll_interval));
        Box::new(PollWatcher::new(tx, config).unwrap())
    } else {
        Box::new(RecommendedWatcher::new(tx, Config::default()).unwrap())
    };

    for f in &files {
        register_watch_for_file(&mut watcher, f)?;
    }

    let command_queue_tx = Queue::new(&args)?;

    // test
    loop {
        match rx.recv() {
            Ok(event) if event.is_ok() => {
                let event = event.unwrap();
                //println!("Received Event: {:?}", event);
                if let EventKind::Modify(_) = event.kind {
                    debug!("File modified: {:?}", event.paths);

                    for p in &event.paths {
                        if !extension_matches(p, args.extensions.as_slice()) {
                            debug!("Ignoring update for {:?}", p);
                            continue;
                        }

                        command_queue_tx.send(QueueMessage::AddFile(p.clone()))?;
                    }
                }
            }
            Err(error) => return Err(ProgramErrors::FileWatchError(error.to_string()).into()),
            _ => {}
        }
    }
}

fn register_watch_for_file(
    watcher: &mut Box<dyn Watcher>,
    file: &str,
) -> Result<(), ProgramErrors> {
    let p = path::absolute(file).expect("Could not determine abs path").canonicalize();

    if let Err(e) = p {
        return Err(ProgramErrors::FileError(file.to_string(), e.to_string()));
    }
    let p = p.unwrap();

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

    info!("Registering a watch for {:?} / {:?}", watch_target.as_path(), watch_mode);
    watcher.watch(watch_target.as_path(), watch_mode).unwrap();

    Ok(())
}

/// Checks if the filename extensions is part of our allow-list
/// Returns true if the allow-list is empty
fn extension_matches(filename: &PathBuf, allowed_extensions: &[String]) -> bool {
    //debug!("extension_matches : {:?} {:?}", filename, allowed_extensions);

    if allowed_extensions.is_empty() {
        return true;
    }

    let ext = filename.extension();
    is_some_or_return!(ext, false);
    let ext = ext.unwrap().to_owned().into_string();
    is_ok_or_return!(ext, false);
    let ext = ext.unwrap();

    allowed_extensions.contains(&ext)
}
