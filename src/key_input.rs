use crate::event::Event;
use crossbeam_channel::Sender;
use crossterm::event::{Event as CrosstermEvent, KeyCode};
use std::time::Duration;

#[derive(Debug)]
pub enum KeyInputMessage {
    Quit,
}

pub fn monitor_key_inputs(tx: Sender<Event>) {
    loop {
        if crossterm::event::poll(Duration::from_millis(100)).unwrap() {
            if let CrosstermEvent::Key(key_event) = crossterm::event::read().unwrap() {
                match key_event.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                        let _ = tx.send(Event::Key(KeyInputMessage::Quit));
                        return;
                    }
                    _ => {}
                }
            }
        }
    }
}
