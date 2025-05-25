use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use command::execution_report::ExecutionUpdate;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use key_input::KeyInputMessage;
use notify::*;
use std::path::{PathBuf, absolute};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

static PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
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
use command::exit_code::get_exit_code_string;

pub mod key_input;

fn main() {
    match run() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{} {} {:?}", PROGRAM_NAME.bold(), "error".red(), e);
            std::process::exit(1);
        }
    }
}

fn run() -> Result<()> {
    let mut args = Args::parse();
    args.validate()?;
    let args = args;

    // Stores tuples (watcher, rx, top-level file)
    let mut file_watchers: Vec<(
        Box<dyn Watcher>,
        Receiver<std::result::Result<Event, Error>>,
        PathBuf,
    )> = Vec::new();

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

    let (report_tx, report_rx) = std::sync::mpsc::channel::<ExecutionUpdate>();
    let (key_input_tx, key_input_rx) = std::sync::mpsc::channel::<KeyInputMessage>();

    // Start the command key , key input listener
    let command_queue_tx = Queue::new(&args, report_tx);
    std::thread::spawn(move || key_input::monitor_key_inputs(key_input_tx));

    // UI progress bars
    let multi_p = MultiProgress::new();
    let mut pbs = Vec::new();

    // Event loop
    loop {
        // Receive FileWatch updates
        for (_, rx, watch) in &file_watchers {
            match rx.try_recv() {
                Ok(event) if event.is_ok() => {
                    let event = event.unwrap();
                    match event.kind {
                        EventKind::Modify(_) | EventKind::Remove(_) => {
                            // debug!("File modified: {:?}", event.paths);
                            for p in &event.paths {
                                if should_be_ignored(p, &args, watch) {
                                    // println!("Ignoring update for {:?}", p);
                                    // test
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
                    eprintln!("Watch file error: {:?}", event);
                }
                Err(TryRecvError::Empty) => {}
                Err(error) => return Err(ProgramErrors::FileWatchError(error.to_string()).into()),
                _ => {}
            }
        }

        // Receive Execution report updates
        match report_rx.try_recv() {
            Ok(ExecutionUpdate::Start(report)) => {
                let pb = multi_p.insert(report.command_number, ProgressBar::new_spinner());

                pb.set_style(
                    ProgressStyle::default_spinner()
                        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
                        .template("{prefix} {spinner} - {wide_msg} [{elapsed}]")
                        .expect("no template error"),
                );
                pb.set_prefix(format!(
                    "#{:3}. {:40}",
                    report.command_number + 1,
                    report.files.join(", ")
                ));
                pb.enable_steady_tick(Duration::from_millis(100));
                pbs.insert(report.command_number, pb);
            }
            Ok(ExecutionUpdate::Finish(report)) => {
                //println!("Finished {}", report.command_number);
                let pb = pbs.get_mut(report.command_number).unwrap();

                pb.set_message(format!("code: {}", get_exit_code_string(report.exit_code)));

                if let Some(c) = report.exit_code {
                    if c != 0 {
                        println!("stdout: {:?}", report.stdout);
                        println!("stderr: {:?}", report.stderr);
                    }
                }

                pb.finish();
            }
            Err(TryRecvError::Empty) => {}
            Err(e) => {
                return Err(ProgramErrors::CommandExecutionError(e.to_string()).into());
            }
        }

        // Receive user key inputs
        match key_input_rx.try_recv() {
            Ok(KeyInputMessage::Quit) => {
                println!("Quitting!");
                let _ = command_queue_tx.send(QueueMessage::Abort);
                return Ok(());
            }
            Err(TryRecvError::Empty) => {}
            Err(e) => {
                dbg!(e);
                return Err(ProgramErrors::BadInternalState.into());
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

    // println!("Registering a {:?} watch for {:?}", watch_mode, watch_target.as_path());
    watcher.watch(watch_target.as_path(), watch_mode).unwrap();

    Ok(p)
}
