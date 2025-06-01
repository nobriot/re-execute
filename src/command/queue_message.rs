use std::path::PathBuf;

/// Messages issued to the command queue
pub enum QueueMessage {
    /// Tell the queue to stop.
    Abort,
    /// Tell the queue to wait longer before executing the file update command
    RestartBackoff,
    /// Insert an update of a file.
    /// First PathBuf is the updated file / Second is the top level watch
    AddFile(PathBuf, PathBuf),
}
