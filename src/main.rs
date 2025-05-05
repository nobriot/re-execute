use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use log::*;
use notify::*;
use std::path::{PathBuf, absolute};
use std::time::Duration;

// static PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
// static PROGRAM_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod args;
use args::Args;

pub mod errors;
use errors::ProgramErrors;

pub mod files;
use files::utils::{extension_matches, should_be_ignored};

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
    debug!("We received {:?}", args);

    if args.files.is_empty() {
        args.files.push(String::from("."))
    }
    let args = args;

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher: Box<dyn Watcher> = if RecommendedWatcher::kind() == WatcherKind::PollWatcher {
        let config =
            Config::default().with_poll_interval(Duration::from_millis(args.poll_interval));
        Box::new(PollWatcher::new(tx, config).unwrap())
    } else {
        Box::new(RecommendedWatcher::new(tx, Config::default()).unwrap())
    };

    for f in &args.files {
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
                    // debug!("File modified: {:?}", event.paths);

                    for p in &event.paths {
                        if !should_be_ignored(p, &args) {
                            // debug!("Ignoring update for {:?}", p);
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
    let p = absolute(file).expect("Could not determine abs path").canonicalize();

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

    info!("Registering a {:?} watch for {:?}", watch_mode, watch_target.as_path());
    watcher.watch(watch_target.as_path(), watch_mode).unwrap();

    Ok(())
}
