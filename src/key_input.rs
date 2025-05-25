use crossterm::event::{Event, KeyCode};
use std::sync::mpsc::Sender;
use std::time::Duration;

#[derive(Debug)]
pub enum KeyInputMessage {
    Quit,
}

pub fn monitor_key_inputs(tx: Sender<KeyInputMessage>) {
    loop {
        if crossterm::event::poll(Duration::from_millis(100)).unwrap() {
            if let Event::Key(key_event) = crossterm::event::read().unwrap() {
                match key_event.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                        let _ = tx.send(KeyInputMessage::Quit);
                        return;
                    }
                    _ => {}
                }
            }
        }
    }
}
