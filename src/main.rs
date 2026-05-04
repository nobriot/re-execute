use anyhow::Result;
use colored::Colorize;
use crossbeam_channel::{Receiver, Select, Sender, tick, unbounded};
use notify::*;
use std::path::{PathBuf, absolute};
use std::time::Duration;
use term_events::TermEvents;

pub mod event;
use event::Event;

pub mod args;
use args::Args;

pub mod errors;
use errors::{ProgramError, RuntimeError, runtime_error};

pub mod files;
use files::utils::should_be_ignored;

pub mod command;
use command::Queue;
use command::QueueMessage;

pub mod logging;
pub mod term_events;
pub mod tui;
use tui::Output;
use tui::RawModeGuard;

fn main() {
    let _raw_mode = RawModeGuard::new().expect("Could not enable raw mode");
    let result = run();
    drop(_raw_mode);

    match result {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{}: {} {:?}", tui::PROGRAM_NAME.bold(), "error".red(), e);
            std::process::exit(1);
        }
    }
}

fn run() -> Result<()> {
    let mut args = Args::try_parse()?;
    args.validate()?;
    let args = args;

    logging::setup(args.log_file.as_deref());
    log::info!("Starting {} v{}", tui::PROGRAM_NAME, env!("CARGO_PKG_VERSION"));
    log::debug!("Parsed arguments: {:?}", args);

    let mut file_watchers: Vec<Box<dyn Watcher>> = Vec::new();
    let mut rx_with_path: Vec<(Receiver<Event>, PathBuf)> = Vec::new();

    for f in &args.files {
        let (tx, rx) = unbounded::<Event>();
        let mut watcher = get_watcher(tx, &args);
        let p = register_watch_for_file(&mut watcher, f)?;
        file_watchers.push(watcher);
        rx_with_path.push((rx, p));
    }

    let (event_tx, event_rx) = unbounded::<Event>();

    // Start the command queue
    let tx_clone = event_tx.clone();
    let command_queue_tx = Queue::start(&args, tx_clone)?;
    // Start listening on keys
    std::thread::spawn(move || term_events::monitor_key_inputs(event_tx));

    // Printout / output
    let mut output = Output::new(&args);

    let mut select = Select::new();
    let mut rxs = Vec::new();

    for (rx, _) in &rx_with_path {
        select.recv(rx);
        rxs.push(rx);
    }
    select.recv(&event_rx);
    rxs.push(&event_rx);

    // Ticker that fires every 100 ms to flush buffered output in one render cycle.
    let flush_tick = tick(Duration::from_millis(100));
    select.recv(&flush_tick);
    let flush_tick_index = rxs.len(); // index of the tick receiver in the select

    let rxs = rxs;
    let mut paused = false;

    // Event loop
    loop {
        let operation = select.select();
        let index = operation.index();

        // Handle the flush tick separately (different channel type).
        if index == flush_tick_index {
            let _ = operation.recv(&flush_tick);
            output.tick_spinners();
            output.flush_output();
            continue;
        }

        let rx = rxs[index];

        match operation.recv(rx) {
            Ok(Event::FileWatch(file_watch)) => {
                // if the program is paused, ignore file updates
                if paused {
                    continue;
                }
                match file_watch {
                    Ok(event) => match event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                            let (_, watch) = &rx_with_path[index];
                            for p in &event.paths {
                                if should_be_ignored(p, &args, watch) {
                                    continue;
                                }

                                log::debug!("File change accepted: {:?} ({:?})", p, event.kind);
                                command_queue_tx
                                    .send(QueueMessage::AddFile(p.clone(), watch.clone()))?;
                            }
                        }
                        _ => {}
                    },
                    Err(error) => {
                        log::error!("File watch error: {}", error);
                        return Err(runtime_error!(FileWatchError, error.to_string()).into());
                    }
                }
            }
            Ok(Event::Exec(update)) => output.update(update),
            Ok(Event::Term(TermEvents::Quit)) => {
                log::info!("Quit signal received, shutting down");
                let _ = command_queue_tx.send(QueueMessage::Abort);
                output.finish();
                return Ok(());
            }
            Ok(Event::Term(TermEvents::Resize(..))) => {
                output.redraw();
            }
            Ok(Event::Term(TermEvents::ClearScreen)) => {
                output.clear_output();
            }
            Ok(Event::TogglePause) => {
                paused = !paused;
                output.set_pause(paused);
            }
            Ok(Event::AbortOngoingCommands) => {
                log::debug!("Request to abort command received");
                command_queue_tx.send(QueueMessage::AbortOngoingCommands)?;
            }
            Err(e) => {
                return Err(runtime_error!(ChannelReceiveError, e.to_string()).into());
            }
        }
    }
}

/// Updates the watcher to watch the file pointed by &str, if it exists
/// Returns a Result with the PathBuf
fn register_watch_for_file(
    watcher: &mut Box<dyn Watcher>,
    file: &str,
) -> Result<PathBuf, ProgramError> {
    let p = absolute(file)
        .map_err(|e| runtime_error!(FileError, file.to_string(), e.to_string()))?
        .canonicalize()
        .map_err(|e| runtime_error!(FileError, file.to_string(), e.to_string()))?;

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

    log::info!("Watching {:?} ({:?})", watch_target.display(), watch_mode);
    watcher
        .watch(watch_target.as_path(), watch_mode)
        .map_err(|e| runtime_error!(FileWatchError, e.to_string()))?;

    Ok(p)
}

/// Gets the recommended watcher using the Sender
fn get_watcher(tx: Sender<Event>, args: &Args) -> Box<dyn Watcher> {
    if args.force_poll || RecommendedWatcher::kind() == WatcherKind::PollWatcher {
        log::debug!("Using PollWatcher (interval: {}ms)", args.poll_interval);
        let config =
            Config::default().with_poll_interval(Duration::from_millis(args.poll_interval));
        Box::new(
            PollWatcher::new(
                move |res| {
                    tx.send(Event::FileWatch(res)).expect("Could not send watch event to channel");
                },
                config,
            )
            .unwrap(),
        )
    } else {
        log::debug!("Using RecommendedWatcher ({:?})", RecommendedWatcher::kind());
        Box::new(
            RecommendedWatcher::new(
                move |res| {
                    tx.send(Event::FileWatch(res)).expect("Could not send watch event to channel");
                },
                Config::default(),
            )
            .unwrap(),
        )
    }
}
