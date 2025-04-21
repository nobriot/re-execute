use std::path::PathBuf;

pub enum QueueMessage {
    Abort,
    RestartBackoff,
    AddFile(PathBuf),
}
