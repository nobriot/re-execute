use std::path::PathBuf;

pub enum QueueMessage {
    Abort,
    RestartBackoff,
    /// Insert an update of a file.
    /// First PathBuf is the updated file / Second is the top level watch
    AddFile(PathBuf, PathBuf),
}
