use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use crossbeam_channel::{Receiver, Select, Sender, unbounded};
//use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use notify::*;
use std::path::{PathBuf, absolute};
use std::time::Duration;
use term_events::TermEvents;

pub mod event;
use event::Event;

pub mod args;
use args::Args;

pub mod errors;
use errors::ProgramErrors;

pub mod files;
use files::utils::should_be_ignored;

pub mod command;
use command::Queue;
use command::QueueMessage;

pub mod output;
pub mod term_events;
use output::Output;

fn main() {
    // Disable user input directly in the console
    //enable_raw_mode().expect("Could not enable raw mode");
    //set_echo(false);
    let result = run();
    //disable_raw_mode().expect("Could not disable raw mode");

    match result {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{} {} {:?}", output::PROGRAM_NAME.bold(), "error".red(), e);
            std::process::exit(1);
        }
    }
}

fn run() -> Result<()> {
    let mut args = Args::parse();
    args.validate()?;
    let args = args;

    // Stores tuples (watcher, rx, top-level file)
    let mut file_watchers: Vec<Box<dyn Watcher>> = Vec::new();
    let mut rx_with_path: Vec<(Receiver<Event>, PathBuf)> = Vec::new();

    for f in &args.files {
        let (tx, rx) = unbounded::<Event>(); //std::sync::mpsc::channel();
        let mut watcher = get_watcher(tx, &args);
        let p = register_watch_for_file(&mut watcher, f)?;
        file_watchers.push(watcher);
        rx_with_path.push((rx, p));
    }

    let (event_tx, event_rx) = unbounded::<Event>();

    // Start the command queue
    let tx_clone = event_tx.clone();
    let command_queue_tx = Queue::start(&args, tx_clone);
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
    let rxs = rxs;

    // Event loop
    loop {
        let operation = select.select();
        let index = operation.index();
        let rx = rxs[index];

        match operation.recv(rx) {
            Ok(Event::FileWatch(file_watch)) => match file_watch {
                Ok(event) => match event.kind {
                    EventKind::Modify(_) | EventKind::Remove(_) => {
                        let (_, watch) = &rx_with_path[index];
                        for p in &event.paths {
                            if should_be_ignored(p, &args, watch) {
                                continue;
                            }

                            command_queue_tx
                                .send(QueueMessage::AddFile(p.clone(), watch.clone()))?;
                        }
                    }
                    _ => {}
                },
                Err(error) => return Err(ProgramErrors::FileWatchError(error.to_string()).into()),
                //_ => {}
            },
            Ok(Event::Exec(update)) => output.update(update),
            Ok(Event::Term(TermEvents::Quit)) => {
                let _ = command_queue_tx.send(QueueMessage::Abort);
                output.println("Quitting...");
                output.finish();
                return Ok(());
            }
            Ok(Event::Term(TermEvents::Resize(..))) => {
                output.redraw();
            }
            //Ok(Event::Key(_)) => {}
            Err(e) => {
                return Err(ProgramErrors::ChannelReceiveError(e.to_string()).into());
            }
        }
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

/// Gets the recommended watcher using the Sender
fn get_watcher(tx: Sender<Event>, args: &Args) -> Box<dyn Watcher> {
    if RecommendedWatcher::kind() == WatcherKind::PollWatcher {
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
