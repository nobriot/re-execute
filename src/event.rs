use crate::command::execution_report::ExecMessage;
use crate::term_events::TermEvents;

/// Generic event that can be reported to the main thread
#[derive(Debug)]
pub enum Event {
    // FileWatch event, a file changed, was updated, etc.
    FileWatch(notify::Result<notify::Event>),
    // A notification about a command being executed
    Exec(ExecMessage),
    // A notification from a terminal event
    Term(TermEvents),
}
