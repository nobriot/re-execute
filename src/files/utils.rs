use std::path::PathBuf;

macro_rules! is_some_or_return {
    ($opt:expr, $ret:expr) => {
        if !$opt.is_some() {
            return $ret;
        }
    };
}

macro_rules! is_ok_or_return {
    ($res:expr, $ret:expr) => {
        if !$res.is_ok() {
            return $ret;
        }
    };
}

/// Checks if the filename extensions is part of our allow-list
/// Returns true if the allow-list is empty
pub fn extension_matches(filename: &PathBuf, allowed_extensions: &[String]) -> bool {
    //debug!("extension_matches : {:?} {:?}", filename, allowed_extensions);

    if allowed_extensions.is_empty() {
        return true;
    }

    let ext = filename.extension();
    is_some_or_return!(ext, false);
    let ext = ext.unwrap().to_owned().into_string();
    is_ok_or_return!(ext, false);
    let ext = ext.unwrap();

    allowed_extensions.contains(&ext)
}

pub fn is_gitignored(filename: &PathBuf, gitignore: &PathBuf) -> bool {
    todo!();
}

/// Checks if the file or any parent up to `parent_levels` is hidden
pub fn is_hidden(filename: &PathBuf, parent_levels: usize) -> bool {
    todo!();
}
