use crate::Args;
use same_file;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf, absolute};

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

#[derive(Debug)]
struct IgnoreRule {
    /// Pattern
    pattern: String,
    /// Is the pattern negated
    is_negated: bool,
}

#[derive(Debug)]
struct IgnoreRules {
    /// List of rules found in the file
    pub rules: Vec<IgnoreRule>,
    /// Directory where the rule file is located
    pub rule_path: PathBuf,
}

/// Parse an ignore file into IgnoreRules
fn parse_ignore_file(path: &Path) -> IgnoreRules {
    let mut rules = IgnoreRules { rules: Vec::new(), rule_path: path.to_path_buf() };

    if let Ok(file) = std::fs::File::open(path) {
        for line in BufReader::new(file).lines().flatten() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }
            let is_negated = line.starts_with("!");
            let pattern = if is_negated { &line[1..] } else { &line };
            rules.rules.push(IgnoreRule { pattern: pattern.to_string(), is_negated });
        }
    } else {
        eprintln!("Error reading contents of {:?}", path);
    }

    rules
}

/// Collect all the ignore rules from .gitignore and the like in the
/// current and parent directories.
fn collect_ignore_rules(path: &Path, watch: &PathBuf) -> Vec<IgnoreRules> {
    let mut rules: Vec<IgnoreRules> = Vec::new();
    let mut current_path = if path.is_dir() { Some(path) } else { path.parent() };

    while let Some(dir) = current_path {
        for ignore_file_name in &[".gitignore"] {
            let ignore_path = dir.join(ignore_file_name);
            if !ignore_path.exists() {
                continue;
            }
            rules.push(parse_ignore_file(ignore_path.as_ref()));
        }

        // Abort collecting if one of the path cannot be read
        // (doesn't exist or lack of permissions)
        if same_file::is_same_file(dir, watch).unwrap_or(true) {
            break;
        }
        current_path = dir.parent();
    }

    rules
}

/// Simple pattern matching from the gitignore file format
/// It does not take into account the negation.
fn matches_rule(file: &Path, rule: &IgnoreRule, dir: &Path) -> bool {
    // println!("Checking {:?} against {:?} - top level {:?}", file, rule, dir);
    //
    let file_str = file.strip_prefix(dir).unwrap_or(file).to_string_lossy();
    if rule.pattern.contains("*") {
        // Handle those wildcards
        // TODO: This does not handle all cases ... e.g. if greediness swallows a part match
        // ALso does not respect directory levels
        let parts: Vec<&str> = rule.pattern.split("*").collect();
        let mut idx = 0;
        let mut matched = true;
        for part in parts {
            if let Some(found) = file_str[idx..].find(part) {
                idx += found + part.len();
            } else {
                matched = false;
                break;
            }
        }

        matched
    } else {
        // TODO: Handle / parts, ?
        file_str.contains(&rule.pattern)
    }
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

pub fn is_git_ignored(filename: &PathBuf, watch: &PathBuf) -> bool {
    let abs_path = absolute(filename).unwrap_or(filename.clone());
    let all_rules = collect_ignore_rules(&abs_path, watch);

    // Check if a negative rule matches, if yes, it is not ignored, no matter
    // the other matches
    for ignore_rules in &all_rules {
        for rule in &ignore_rules.rules {
            if !rule.is_negated {
                continue;
            }
            if matches_rule(&abs_path, &rule, &ignore_rules.rule_path) {
                return false;
            }
        }
    }

    // Second pass, non-negated rules
    for ignore_rules in &all_rules {
        for rule in &ignore_rules.rules {
            if rule.is_negated {
                continue;
            }
            if matches_rule(&abs_path, &rule, &ignore_rules.rule_path) {
                return true;
            }
        }
    }

    false
}

/// Checks if the file or any parent directory is hidden
pub fn is_hidden(filename: &PathBuf, watch: &PathBuf) -> bool {
    let mut path = filename.clone();

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

/// Checks if a single file is hidden.
fn is_file_hidden(filename: &PathBuf) -> bool {
    if let Some(basename) = filename.file_name() {
        if basename.to_string_lossy().starts_with(".") {
            return true;
        }
    }

    #[cfg(windows)]
    {
        if let Ok(metadata) = fs::metadata(filename) {
            const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
            if metadata.file_attribute() && FILE_ATTRIBUTE_HIDDEN != 0 {
                return true;
            }
        }
    }

    false
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
        assert!(!extension_matches(&filename, &[
            String::from("txt"),
            String::from(""),
            String::from("txt.ignored"),
            String::from("gnored")
        ]));
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
}
