use crate::Args;
use crate::files::git::is_git_ignored;

use regex::Regex;
use std::path::{Path, PathBuf};

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

/// Checks if a file update should be ignored
///
pub fn should_be_ignored(filename: &PathBuf, args: &Args, watch: &PathBuf) -> bool {
    if !extension_matches(filename, args.extensions.as_slice()) {
        return true;
    }
    if !args.deleted && !filename.exists() {
        return true;
    }
    if !has_regex_match(&args.regexps, filename, watch) {
        return true;
    }
    if !args.no_gitignore && is_git_ignored(filename, watch) {
        return true;
    }
    if !args.hidden && is_hidden(filename, watch) {
        return true;
    }

    false
}

/// Checks if the filename extensions is part of our allow-list
/// Returns true if the allow-list is empty
/// if the extension "" is passed, files without extension will match
pub fn extension_matches(filename: &Path, allowed_extensions: &[String]) -> bool {
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

/// Checks if the file or any parent directory is hidden
/// up to the watch directory level.
pub fn is_hidden(filename: &Path, watch: &PathBuf) -> bool {
    let mut path = filename.to_path_buf();

    loop {
        if is_file_hidden(&path) {
            return true;
        }
        if !path.pop() {
            break;
        }

        if path == *watch {
            break;
        }
    }

    false
}

/// Checks if the file or any parent directory is hidden
/// up to the watch directory level.
pub fn has_regex_match(regex: &[Regex], filename: &Path, watch: &PathBuf) -> bool {
    // No regex given, we accept all files
    if regex.is_empty() {
        return true;
    }

    // first stop, we actually clip the part of the filename
    let path = relative_path_within_dir(filename, watch);

    // Now check that all the regexes are matching
    for r in regex {
        if !r.is_match(&path) {
            return false;
        }
    }

    true
}
// ------------------------------------------------------------------------------------------------
// private

/// Checks if a single file is hidden.
fn is_file_hidden(filename: &Path) -> bool {
    if let Some(basename) = filename.file_name() {
        if basename.to_string_lossy().starts_with(".") {
            return true;
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if let Ok(metadata) = std::fs::metadata(filename) {
            const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
            if metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0 {
                return true;
            }
        }
    }

    false
}

/// Returns a String showing the relative path of a
/// filename located inside a directory
fn relative_path_within_dir<P, Q>(filename: P, dir: Q) -> String
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let filename = &filename.as_ref().to_string_lossy();
    let dir = &dir.as_ref().to_string_lossy();

    match filename.strip_prefix(dir.as_ref()) {
        Some(rel) => {
            let mut idx = 0;
            while idx < rel.len() && rel[idx..].starts_with("/") {
                idx += 1;
            }
            rel[idx..].to_owned()
        }
        None => filename.clone().into_owned(),
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::{path::PathBuf, str::FromStr};

    #[test]
    fn test_extension_matches_exact() {
        let filename =
            PathBuf::from_str("/home/test/my-file.rs").expect("Could not create PathBuf");
        assert!(extension_matches(&filename, &[String::from("rs")]));
    }

    #[test]
    fn test_extension_matches_empty_allow_list() {
        let filename = PathBuf::from_str("file.txt").expect("Could not create PathBuf");
        assert!(extension_matches(&filename, &[]));
    }

    #[test]
    fn test_extension_matches_subset() {
        let filename = PathBuf::from_str("file.txt").expect("Could not create PathBuf");
        assert!(!extension_matches(&filename, &[String::from("xt"), String::from("tx")]));
    }

    #[test]
    fn test_extension_matches_double_extension() {
        let filename =
            PathBuf::from_str("a/path/file.txt.ignored").expect("Could not create PathBuf");
        assert!(!extension_matches(
            &filename,
            &[
                String::from("txt"),
                String::from(""),
                String::from("txt.ignored"),
                String::from("gnored")
            ]
        ));
    }

    #[test]
    fn test_extension_matches_double_extension_happy_case() {
        let filename = PathBuf::from_str(".txt.ignored").expect("Could not create PathBuf");
        assert!(extension_matches(&filename, &[String::from("txt"), String::from("ignored")]));
    }

    #[test]
    fn test_extension_matches_no_ext() {
        let filename = PathBuf::from_str("path/to/my_file").expect("Could not create PathBuf");
        assert!(extension_matches(&filename, &[String::from("")]));
    }

    #[test]
    fn test_extension_matches_case() {
        let filename = PathBuf::from_str(".txt.jPeG").expect("Could not create PathBuf");
        assert!(extension_matches(&filename, &[String::from("jpeg")]));
    }

    #[test]
    fn test_is_hidden() {
        let filename = PathBuf::from_str("/a/path/.with/hidden_dir/file.jPeG").expect("test error");
        let watch = PathBuf::from_str("/a/path/.with/hidden_dir").expect("test error");
        assert!(!is_hidden(&filename, &watch));
        let watch = PathBuf::from_str("/").expect("test error");
        assert!(is_hidden(&filename, &watch));
    }

    #[test]
    fn test_is_hidden_file_itself() {
        let filename = PathBuf::from_str("/a/path/with/hidden_dir/.file.txt").expect("test error");
        let watch = PathBuf::from_str("/a/").expect("test error");
        assert!(is_hidden(&filename, &watch));
    }

    #[test]
    fn test_is_not_hidden() {
        let filename =
            PathBuf::from_str("/.a/path/with/not_hidden_dir/file.txt").expect("test error");
        let watch = PathBuf::from_str("/.a/path").expect("test error");
        assert!(!is_hidden(&filename, &watch));
    }

    #[test]
    fn test_relative_filename() {
        let filename = Path::new("/home/user/.config/app/Cache/Cache_Data/index-dir/temp-index");
        let watch = PathBuf::from_str("/home/user/.config").expect("Could not create PathBuf");
        assert_eq!(
            relative_path_within_dir(&filename, &watch),
            String::from("app/Cache/Cache_Data/index-dir/temp-index")
        );
    }
}
