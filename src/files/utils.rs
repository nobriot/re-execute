use crate::Args;
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

/// Checks if a file should be ignored after an update
pub fn should_be_ignored(filename: &PathBuf, args: &Args) -> bool {
    extension_matches(filename, args.extensions.as_slice())
        && !is_git_ignored(filename)
        && !is_hidden(filename)
}

/// Checks if the filename extensions is part of our allow-list
/// Returns true if the allow-list is empty
/// if the extension "" is passed, files without extension will match
pub fn extension_matches(filename: &PathBuf, allowed_extensions: &[String]) -> bool {
    if allowed_extensions.is_empty() {
        return true;
    }

    let ext = filename.extension();
    if ext.is_none() {
        return allowed_extensions.iter().any(|ext| ext.is_empty());
    }
    is_some_or_return!(ext, false);
    let ext = ext.unwrap().to_owned().into_string();
    is_ok_or_return!(ext, false);
    let ext = ext.unwrap();

    allowed_extensions.contains(&ext.to_lowercase())
}

pub fn is_git_ignored(filename: &PathBuf) -> bool {
    todo!();
}

/// Checks if the file or any parent directory is hidden
pub fn is_hidden(filename: &PathBuf) -> bool {
    todo!();
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::{path::PathBuf, str::FromStr};

    #[test]
    fn test_extension_matches_exact() {
        let filename =
            PathBuf::from_str("/home/test/my-file.rs").expect("Could not create PathBuf");
        assert!(extension_matches(&filename, &[String::from("rs")]))
    }

    #[test]
    fn test_extension_matches_empty_allow_list() {
        let filename = PathBuf::from_str("file.txt").expect("Could not create PathBuf");
        assert!(extension_matches(&filename, &[]))
    }

    #[test]
    fn test_extension_matches_subset() {
        let filename = PathBuf::from_str("file.txt").expect("Could not create PathBuf");
        assert!(!extension_matches(&filename, &[String::from("xt"), String::from("tx")]))
    }

    #[test]
    fn test_extension_matches_double_extension() {
        let filename =
            PathBuf::from_str("a/path/file.txt.ignored").expect("Could not create PathBuf");
        assert!(!extension_matches(&filename, &[
            String::from("txt"),
            String::from(""),
            String::from("txt.ignored"),
            String::from("gnored")
        ]))
    }

    #[test]
    fn test_extension_matches_double_extension_happy_case() {
        let filename = PathBuf::from_str(".txt.ignored").expect("Could not create PathBuf");
        assert!(extension_matches(&filename, &[String::from("txt"), String::from("ignored")]))
    }

    #[test]
    fn test_extension_matches_no_ext() {
        let filename = PathBuf::from_str("path/to/my_file").expect("Could not create PathBuf");
        assert!(extension_matches(&filename, &[String::from("")]))
    }

    #[test]
    fn test_extension_matches_case() {
        let filename = PathBuf::from_str(".txt.jPeG").expect("Could not create PathBuf");
        assert!(extension_matches(&filename, &[String::from("jpeg")]))
    }
}
