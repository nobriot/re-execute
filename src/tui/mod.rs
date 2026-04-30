pub mod output;
pub use output::Output;
pub use output::PROGRAM_NAME;

pub mod term;
pub use term::RawModeGuard;

pub mod duration;
pub use duration::format_duration;
