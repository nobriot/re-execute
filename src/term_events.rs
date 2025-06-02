use crate::event::Event;
use crossbeam_channel::Sender;
use crossterm::event::{Event as CrosstermEvent, KeyCode};
use std::time::Duration;

#[derive(Debug)]
pub enum TermEvents {
    /// User wishes to quit
    Quit,
    ///Terminal resize (columns, rows)
    Resize(u16, u16),
}

pub fn monitor_key_inputs(tx: Sender<Event>) {
    loop {
        if crossterm::event::poll(Duration::from_millis(100)).unwrap() {
            match crossterm::event::read().unwrap() {
                CrosstermEvent::FocusGained => {}
                CrosstermEvent::FocusLost => {}
                CrosstermEvent::Key(key_event) => match key_event.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                        let _ = tx.send(Event::Term(TermEvents::Quit));
                        return;
                    }
                    _ => {}
                },
                CrosstermEvent::Mouse(_) => {}
                CrosstermEvent::Paste(_) => {}
                CrosstermEvent::Resize(c, r) => {
                    let _ = tx.send(Event::Term(TermEvents::Resize(c, r)));
                }
            }
        }
    }
}
