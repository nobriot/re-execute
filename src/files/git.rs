use same_file;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf, absolute};

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
            if matches_rule(&abs_path, rule, &ignore_rules.rule_path) {
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
            if matches_rule(&abs_path, rule, &ignore_rules.rule_path) {
                return true;
            }
        }
    }

    false
}

// ------------------------------------------------------------------------------------------------
// private

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
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }
            let is_negated = line.starts_with("!");
            let pattern = if is_negated { &line[1..] } else { line };
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
    // FIXME: does not support []
    // e.g. *.py[cov]
    // println!("Checking {:?} against {:?} - top level {:?}", file, rule, dir);
    let file_str = file.strip_prefix(dir).unwrap_or(file).to_string_lossy();
    if rule.pattern.contains("*") {
        // Handle those wildcards
        // TODO: This does not handle all cases ... e.g. if greediness swallows a part match
        // ALso does not respect directory levels
        // Also we need handling of ?
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
