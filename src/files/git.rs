use same_file;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf, absolute};

pub fn is_git_ignored(filename: &PathBuf, watch: &PathBuf) -> bool {
    let abs_path = absolute(filename).unwrap_or(filename.clone());
    //let all_rules = collect_ignore_rules(&abs_path, watch);
    let all_rules = GitIgnoreRules::from_dir(&abs_path, watch);

    // Check if a negative rule matches, if yes, it is not ignored, no matter
    // the other matches
    for ignore_rules in &all_rules {
        let ignore_path = &ignore_rules.rule_path;
        for rule in &ignore_rules.rules {
            if !rule.is_negated {
                continue;
            }
            if rule.file_matches(&abs_path, &ignore_path) {
                return false;
            }
            // if matches_rule(&abs_path, rule, &ignore_rules.rule_path) {
            //     return false;
            // }
        }
    }

    // Second pass, non-negated rules
    for ignore_rules in &all_rules {
        let ignore_path = &ignore_rules.rule_path;
        for rule in &ignore_rules.rules {
            if rule.is_negated {
                continue;
            }
            if rule.file_matches(&abs_path, &ignore_path) {
                return false;
            }
            // if matches_rule(&abs_path, rule, &ignore_rules.rule_path) {
            //     return true;
            // }
        }
    }

    false
}

// ------------------------------------------------------------------------------------------------
// private

// OMG what did the people designing this pattern matching where thinking ??

// From the official docs:
// https://git-scm.com/docs/gitignore
// PATTERN FORMAT
// A blank line matches no files, so it can serve as a separator for readability.
// A line starting with # serves as a comment. Put a backslash ("\") in front of the first hash for patterns that begin with a hash.
// Trailing spaces are ignored unless they are quoted with backslash ("\").
// An optional prefix "!" which negates the pattern; any matching file excluded by a previous pattern will become included again. It is not possible to re-include a file if a parent directory of that file is excluded. Git doesn’t list excluded directories for performance reasons, so any patterns on contained files have no effect, no matter where they are defined. Put a backslash ("\") in front of the first "!" for patterns that begin with a literal "!", for example, "\!important!.txt".
// The slash "/" is used as the directory separator. Separators may occur at the beginning, middle or end of the .gitignore search pattern.
// If there is a separator at the beginning or middle (or both) of the pattern, then the pattern is relative to the directory level of the particular .gitignore file itself. Otherwise the pattern may also match at any level below the .gitignore level.
// If there is a separator at the end of the pattern then the pattern will only match directories, otherwise the pattern can match both files and directories.
// For example, a pattern doc/frotz/ matches doc/frotz directory, but not a/doc/frotz directory; however frotz/ matches frotz and a/frotz that is a directory (all paths are relative from the .gitignore file).
// An asterisk "*" matches anything except a slash. The character "?" matches any one character except "/". The range notation, e.g. [a-zA-Z], can be used to match one of the characters in a range. See fnmatch(3) and the FNM_PATHNAME flag for a more detailed description.
// Two consecutive asterisks ("**") in patterns matched against full pathname may have special meaning:
// A leading "**" followed by a slash means match in all directories. For example, "**/foo" matches file or directory "foo" anywhere, the same as pattern "foo". "**/foo/bar" matches file or directory "bar" anywhere that is directly under directory "foo".
// A trailing "/**" matches everything inside. For example, "abc/**" matches all files inside directory "abc", relative to the location of the .gitignore file, with infinite depth.
// A slash followed by two consecutive asterisks then a slash matches zero or more directories. For example, "a/**/b" matches "a/b", "a/x/b", "a/x/y/b" and so on.
// Other consecutive asterisks are considered regular asterisks and will match according to the previous rules.

#[derive(Debug, PartialEq, Clone)]
enum GitIgnoreRuleElements {
    /// A literal string, with escapes removed
    Literal(String),
    /// Directory separator
    Slash,
    /// Single asterisk (*)
    Asterisk,
    /// Double asterisk (**)
    DoubleAsterisk,
    /// Single character wildcard (?)
    QuestionMark,
    /// Character range, e.g., [a-zA-Z]
    /// single char [c] is represented: (false, vec![(c,c)])
    /// Negated ranges [!a-z] is represented: (true, vec![(a,z)])
    CharRange((bool, Vec<(char, char)>)),
}

#[derive(Debug)]
struct GitIgnoreRule {
    /// Pattern
    pattern: Vec<GitIgnoreRuleElements>,
    /// Is the pattern negated
    is_negated: bool,
    /// Regular matching, from the root of the .gitignore file, or at any dir level in between
    match_all_levels: bool,
    /// Do we match files and dirs, or dirs only
    dirs_only: bool,
}

impl GitIgnoreRule {
    /// Creates a GitIgnoreRule from a line
    fn from_str<S: AsRef<str>>(line: S) -> Option<Self> {
        let mut pattern = Vec::new();
        let line: &str = line.as_ref();

        if line.is_empty() || line.starts_with("#") {
            return None;
        }

        let is_negated = line.starts_with("!");
        let match_all_levels = line[..line.len() - 1].chars().filter(|c| *c == '/').count() == 0;
        let dirs_only = line.ends_with("/");

        let line = if is_negated { &line[1..] } else { line };

        // Trim whitespaces at the end if they are not preceeded with a backslash
        let mut spaces_to_trim = 0;
        let mut rev_chars = line.chars().rev().peekable();
        while let Some(' ') = rev_chars.next() {
            if let Some(c) = rev_chars.peek() {
                if *c != '\\' {
                    spaces_to_trim += 1;
                }
            }
        }
        let line = &line[..line.len() - spaces_to_trim];

        let mut chars = line.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '\\' => {
                    // Handle escaped characters
                    if let Some(escaped) = chars.next() {
                        pattern.push(GitIgnoreRuleElements::Literal(escaped.to_string()));
                    }
                }
                '/' => pattern.push(GitIgnoreRuleElements::Slash),
                '*' => {
                    // Handle single or double asterisks
                    if chars.peek() == Some(&'*') {
                        chars.next();
                        pattern.push(GitIgnoreRuleElements::DoubleAsterisk);
                    } else {
                        pattern.push(GitIgnoreRuleElements::Asterisk);
                    }
                }
                '?' => pattern.push(GitIgnoreRuleElements::QuestionMark),
                '[' => {
                    // Character ranges
                    // Disclaimer: this is probably not exactly the same as fnmatch ...
                    let mut range = Vec::new();
                    let mut negated = false;

                    if chars.peek() == Some(&'!') || chars.peek() == Some(&'^') {
                        negated = true;
                        chars.next();
                    }

                    while let Some(start_char) = chars.next() {
                        if start_char == ']' {
                            break;
                        }
                        if chars.peek() == Some(&'-') {
                            chars.next(); // Consume '-'
                            if let Some(end_char) = chars.next() {
                                range.push((start_char, end_char));
                            }
                        } else {
                            range.push((start_char, start_char));
                        }
                    }

                    pattern.push(GitIgnoreRuleElements::CharRange((negated, range)));
                }
                l => {
                    // Handle literals
                    let mut literal = l.to_string();
                    while let Some(&next) = chars.peek() {
                        if next == '/' || next == '*' || next == '?' || next == '[' || next == '\\'
                        {
                            break;
                        }
                        literal.push(chars.next().unwrap());
                    }
                    pattern.push(GitIgnoreRuleElements::Literal(literal));
                }
            }
        }

        Some(GitIgnoreRule { pattern, is_negated, match_all_levels, dirs_only })
    }

    /// Checks if the current git ignore rule matches a file within a dir
    pub fn file_matches<D>(&self, file: &Path, dir: &D) -> bool
    where
        D: AsRef<Path> + std::fmt::Debug,
    {
        // We take the part of the file that is relative to the dir
        let candidate = match file.strip_prefix(dir) {
            Ok(path) => path.to_string_lossy(),
            Err(_) => return false,
        };

        if self.match_all_levels {
            let mut current = candidate.as_ref();
            loop {
                if self.string_matches(current.as_ref(), &self.pattern) {
                    return true;
                }
                if let Some(i) = current.find('/') {
                    current = &current[i + 1..];
                } else {
                    return false;
                }
            }
        } else {
            self.string_matches(candidate.as_ref(), &self.pattern)
        }
    }

    /// Checks if a file name (string) is matching a collection of GitIgnoreRule
    fn string_matches(&self, file: &str, rule: &[GitIgnoreRuleElements]) -> bool {
        let mut p_chars = file.chars().peekable();
        let mut rule_elements = rule.iter().peekable();

        // Ignore the first /, it's to indicate relative mode
        if let Some(GitIgnoreRuleElements::Slash) = rule_elements.peek() {
            let _ = rule_elements.next();
            // We have empty rules, just return false
            if rule_elements.peek().is_none() {
                return false;
            }
            // If we just pop'ed a slash, but the string also happens to be prepended with a slash, remove it also
            if let Some('/') = p_chars.peek() {
                let _ = p_chars.next();
            }
        }

        while let Some(rule_element) = rule_elements.next() {
            match rule_element {
                GitIgnoreRuleElements::Literal(l) => {
                    // Match all chars from the literal:
                    for l_char in l.chars() {
                        let p_char = p_chars.next();
                        if p_char.is_none() {
                            return false;
                        }
                        let p_char = p_char.unwrap();
                        if p_char != l_char {
                            return false;
                        }
                    }
                }
                GitIgnoreRuleElements::Slash => {
                    // Just match a slash from the path.
                    let p = p_chars.next();
                    if p.is_some() && p != Some('/') {
                        return false;
                    }
                    // No slash but more rules will also not be a match
                    if p.is_none() && rule_elements.peek().is_some() {
                        return false;
                    }
                }
                GitIgnoreRuleElements::Asterisk => {
                    // if no more rules, after the *, so it matches anything until a slash.
                    if rule_elements.peek().is_none() {
                        if p_chars.peek().is_none() {
                            // We need at least 1 char to match a *
                            return false;
                        }
                        while let Some(&c) = p_chars.peek() {
                            if c == '/' {
                                break;
                            }
                            p_chars.next();
                        }
                        continue;
                    }

                    // If there are more rules and we got a /, we can already tell it does not match
                    if let Some(&c) = p_chars.peek() {
                        if c == '/' {
                            return false;
                        }
                    }

                    // Else we have to match any number of characters and try to apply the rest
                    // There is probably a better way than cloning here...
                    let remaining_rules: Vec<_> = rule_elements.cloned().collect();

                    // Now try to fit the remainder of the string with the rules
                    // TODO: There is probably some pruning possible here.
                    let file: String = p_chars.collect();
                    for i in 0..file.len() {
                        if self.string_matches(&file[i..], &remaining_rules) {
                            return true;
                        }
                    }

                    return false;
                }
                GitIgnoreRuleElements::DoubleAsterisk => {
                    // Try to match the rest, including accross directories
                    if rule_elements.peek().is_none() {
                        // No more rules, after the **, so it matches anything really.
                        return true;
                    }
                    // Else pick up the remaining rules:
                    // There is probably a better way than cloning here...
                    let remaining_rules: Vec<_> = rule_elements.cloned().collect();

                    // Now try to fit the remainder of the string with the rules
                    // TODO: There is probably some pruning possible here.
                    let file: String = p_chars.collect();
                    if !file.contains('/') {
                        // ** and we are trying anything that does not contain a slash.
                        // We can conclude it's a match
                        return true;
                    }

                    // Try ignoring the ** and match the rest first:
                    let mut remainder = file.as_str();
                    if self.string_matches(remainder, &remaining_rules) {
                        return true;
                    }

                    // Else try stripping directories
                    while let Some(i) = remainder.find('/') {
                        remainder = &remainder[i..];
                        if self.string_matches(remainder, &remaining_rules) {
                            return true;
                        }
                        // Remove the slash for the next attempt
                        remainder = &remainder[1..];
                    }

                    return false;
                }
                GitIgnoreRuleElements::QuestionMark => {
                    // Match a single character except '/'
                    let c = p_chars.next();
                    if c.is_none() {
                        return false;
                    }
                    let c = c.unwrap();
                    if c == '/' {
                        return false;
                    }
                }
                GitIgnoreRuleElements::CharRange((negated, ranges)) => {
                    let c = p_chars.next();
                    if c.is_none() {
                        return false;
                    }
                    let c = c.unwrap();

                    let mut matched = false;
                    for &(start, end) in ranges {
                        if c >= start && c <= end {
                            matched = true;
                        }
                    }
                    if (matched && *negated) || (!matched && !negated) {
                        return false;
                    }
                }
            }
        }

        // We have a match if we consumed all chars from the candidate path
        // If dirs only, we assume it's a match if we consumed all the "rules"
        // if we matched until a directory separator, it's also a match
        let p = p_chars.next();
        p.is_none() || self.dirs_only || p.unwrap() == '/'
    }
}

#[derive(Debug)]
struct GitIgnoreRules {
    /// List of rules found in the file
    pub rules: Vec<GitIgnoreRule>,
    /// Directory where the rule file is located
    pub rule_path: PathBuf,
}

impl GitIgnoreRules {
    /// Creates an instead from a file
    fn from_ignore_file(path: &Path) -> Self {
        let mut rules = Vec::new();

        if let Ok(file) = std::fs::File::open(path) {
            for line in BufReader::new(file).lines().map_while(Result::ok) {
                let rule = GitIgnoreRule::from_str(line);
                if let Some(r) = rule {
                    rules.push(r);
                }
            }
        } else {
            eprintln!("Error reading contents of {:?}", path);
        }

        Self { rules, rule_path: path.to_path_buf() }
    }

    /// Starts collecting GitIgnoreRules from the path, going up to the watch directory
    fn from_dir(path: &Path, watch: &PathBuf) -> Vec<Self> {
        let mut rules: Vec<Self> = Vec::new();
        let mut current_path = if path.is_dir() { Some(path) } else { path.parent() };

        while let Some(dir) = current_path {
            for ignore_file_name in &[".gitignore"] {
                let ignore_path = dir.join(ignore_file_name);
                if !ignore_path.exists() {
                    continue;
                }
                rules.push(Self::from_ignore_file(ignore_path.as_ref()));
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_pattern_from_str() {
        let rule = GitIgnoreRule::from_str("*.log").unwrap();
        assert_eq!(
            rule.pattern,
            vec![
                GitIgnoreRuleElements::Asterisk,
                GitIgnoreRuleElements::Literal(".log".to_string())
            ]
        );
        assert!(!rule.is_negated);

        // Test negated pattern
        let rule = GitIgnoreRule::from_str("!important.log").unwrap();
        assert_eq!(rule.pattern, vec![GitIgnoreRuleElements::Literal("important.log".to_string())]);
        assert!(rule.is_negated);

        // Test character range
        let rule = GitIgnoreRule::from_str("[a-z].txt").unwrap();
        assert_eq!(
            rule.pattern,
            vec![
                GitIgnoreRuleElements::CharRange((false, vec![('a', 'z')])),
                GitIgnoreRuleElements::Literal(".txt".to_string())
            ]
        );

        // Test comments
        let rule = GitIgnoreRule::from_str("#foo[bar].txt");
        assert!(rule.is_none());

        // Empty line
        let rule = GitIgnoreRule::from_str("");
        assert!(rule.is_none());
    }

    #[test]
    fn test_file_matches() {
        // .gitignore file to check against a path
        let dir = tempdir().unwrap();
        let dir = dir.path();
        let ignore_file_path = dir.join(".gitignore");
        File::create(&ignore_file_path).unwrap(); // Create an empty file

        let rule = GitIgnoreRule::from_str("*.log").unwrap();
        assert!(rule.file_matches(dir.join("error.log").as_path(), &dir));
        assert!(!rule.file_matches(dir.join("error.txt").as_path(), &dir));

        let rule = GitIgnoreRule::from_str("!important.log").unwrap();
        assert!(rule.file_matches(dir.join("important.log").as_path(), &dir));
        assert!(rule.is_negated);

        let rule = GitIgnoreRule::from_str("**/temp/*").unwrap();
        assert!(rule.file_matches(dir.join("foo/temp/file.txt").as_path(), &dir));
        assert!(!rule.file_matches(dir.join("foo/temp/").as_path(), &dir));

        let rule = GitIgnoreRule::from_str("a/**/b").unwrap();
        assert!(rule.file_matches(dir.join("a/x/y/b").as_path(), &dir));
        assert!(!rule.file_matches(dir.join("a/x/y/c").as_path(), &dir));
    }

    #[test]
    fn test_from_ignore_file() {
        let dir = tempdir().unwrap();
        let ignore_file_path = dir.path().join(".gitignore");

        // Create a .gitignore file
        let mut file = File::create(&ignore_file_path).unwrap();
        writeln!(file, "*.log").unwrap();
        writeln!(file, "!important.log").unwrap();

        let rules = GitIgnoreRules::from_ignore_file(&ignore_file_path);
        assert_eq!(rules.rules.len(), 2);

        assert_eq!(
            rules.rules[0].pattern,
            vec![
                GitIgnoreRuleElements::Asterisk,
                GitIgnoreRuleElements::Literal(".log".to_string())
            ]
        );
        assert!(!rules.rules[0].is_negated);

        assert_eq!(
            rules.rules[1].pattern,
            vec![GitIgnoreRuleElements::Literal("important.log".to_string())]
        );
        assert!(rules.rules[1].is_negated);
    }

    #[test]
    fn test_from_dir() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        // Create .gitignore files
        let root_ignore = dir.path().join(".gitignore");
        let mut file = File::create(&root_ignore).unwrap();
        writeln!(file, "*.log").unwrap();

        let sub_ignore = subdir.join(".gitignore");
        let mut file = File::create(&sub_ignore).unwrap();
        writeln!(file, "!important.log").unwrap();

        let rules = GitIgnoreRules::from_dir(&subdir, &dir.path().to_path_buf());
        assert_eq!(rules.len(), 2);

        // Check root .gitignore
        assert_eq!(
            rules[1].rules[0].pattern,
            vec![
                GitIgnoreRuleElements::Asterisk,
                GitIgnoreRuleElements::Literal(".log".to_string())
            ]
        );

        // Check subdir .gitignore
        assert_eq!(
            rules[0].rules[0].pattern,
            vec![GitIgnoreRuleElements::Literal("important.log".to_string())]
        );
        assert!(rules[0].rules[0].is_negated);
    }

    #[test]
    fn test_complex_patterns() {
        let dir = tempdir().unwrap();
        let dir = dir.path();
        let ignore_file_path = dir.join(".gitignore");
        let mut file = File::create(&ignore_file_path).unwrap();

        // Rules
        writeln!(file, "**/foo/**/bar").unwrap(); // 0
        writeln!(file, "dir/").unwrap(); // 1
        writeln!(file, "[ei]*.log").unwrap(); // 2
        writeln!(file, "[a-c]*.txt").unwrap(); // 3
        writeln!(file, "[!c-f]*.txt").unwrap(); // 4

        let rules = GitIgnoreRules::from_ignore_file(&ignore_file_path);

        // Test double asterisk across directories
        assert!(rules.rules[0].file_matches(dir.join("a/foo/b/bar").as_path(), &dir));
        assert!(rules.rules[0].file_matches(dir.join("foo/bar").as_path(), &dir));
        assert!(rules.rules[0].file_matches(dir.join("a/foo/baz/bar").as_path(), &dir));
        assert!(rules.rules[0].file_matches(dir.join("a/foo/baz/bar/hey").as_path(), &dir));

        // Test trailing slash for directories
        assert!(rules.rules[1].file_matches(dir.join("dir/").as_path(), &dir));
        assert!(rules.rules[1].file_matches(dir.join("dir/file.txt").as_path(), &dir));
        assert!(!rules.rules[1].file_matches(dir.join("directory/").as_path(), &dir));
        assert!(!rules.rules[1].file_matches(dir.join("dir2").as_path(), &dir));
        assert!(!rules.rules[1].file_matches(dir.join("dir2/").as_path(), &dir));

        // test with range and wildcard
        assert!(rules.rules[2].file_matches(dir.join("error.log").as_path(), &dir));
        assert!(rules.rules[2].file_matches(dir.join("important.log").as_path(), &dir));
        assert!(!rules.rules[2].file_matches(dir.join("unimportant.log").as_path(), &dir));

        // Test character ranges
        assert!(rules.rules[3].file_matches(dir.join("a_file.txt").as_path(), &dir));
        assert!(rules.rules[3].file_matches(dir.join("b_file.txt").as_path(), &dir));
        assert!(!rules.rules[3].file_matches(dir.join("d_file.txt").as_path(), &dir));

        // Test negated character ranges
        assert!(rules.rules[4].file_matches(dir.join("b_file.txt").as_path(), &dir));
        assert!(!rules.rules[4].file_matches(dir.join("d_file.txt").as_path(), &dir));
        assert!(!rules.rules[4].file_matches(dir.join("e_file.txt").as_path(), &dir));
    }

    #[test]
    fn test_edge_cases() {
        // Test .gitignore file
        let dir = tempdir().unwrap();
        let dir = dir.path();
        let ignore_file_path = dir.join(".gitignore");
        File::create(&ignore_file_path).unwrap(); // Create an empty file

        let rules = GitIgnoreRules::from_ignore_file(&ignore_file_path);
        assert!(rules.rules.is_empty());

        // Test file not under the watched directory
        let rule = GitIgnoreRule::from_str("*.log").unwrap();

        // file_matches(dir.join("a/foo/b/bar").as_path(), &dir));
        assert!(!rule.file_matches(dir.join("outside/error.log").as_path(), &dir));

        // Test pattern with escaped characters
        let rule = GitIgnoreRule::from_str(r"\!important.log").unwrap();
        assert!(rule.file_matches(dir.join("!important.log").as_path(), &dir));
        assert!(!rule.file_matches(dir.join("important.log").as_path(), &dir));

        // Test pattern with trailing spaces
        let rule = GitIgnoreRule::from_str("*.log   ").unwrap();
        assert!(rule.file_matches(dir.join("error.log").as_path(), &dir));
        assert!(!rule.file_matches(dir.join("error.lot").as_path(), &dir));

        // Again, but escaped
        let rule = GitIgnoreRule::from_str("*.log\\ \\  ").unwrap();
        assert!(rule.file_matches(dir.join("error.log  ").as_path(), &dir));
        assert!(!rule.file_matches(dir.join("error.log ").as_path(), &dir));
        assert!(!rule.file_matches(dir.join("error.log").as_path(), &dir));

        // testing the ?
        let rule = GitIgnoreRule::from_str("a/f??/bar").unwrap();
        assert!(rule.file_matches(dir.join("a/foo/bar").as_path(), &dir));
        assert!(rule.file_matches(dir.join("a/fii/bar").as_path(), &dir));
        assert!(!rule.file_matches(dir.join("a/f/i/bar").as_path(), &dir));
        assert!(!rule.file_matches(dir.join("a/fo/bar").as_path(), &dir));

        // Just a slash should do nothing special
        let rule = GitIgnoreRule::from_str("/").unwrap();
        assert!(!rule.file_matches(dir.join("").as_path(), &dir));
        assert!(!rule.file_matches(dir.join("file.txt").as_path(), &dir));

        // Test pattern with only slashes
        let rule = GitIgnoreRule::from_str("/target").unwrap();
        assert!(rule.file_matches(dir.join("target/debug").as_path(), &dir));
        assert!(!rule.file_matches(dir.join("target2/debug").as_path(), &dir));
    }

    #[test]
    fn test_combined_rules() {
        let dir = tempdir().unwrap();
        let dir = dir.path();
        let ignore_file_path = dir.join(".gitignore");

        // Create a .gitignore file with multiple rules
        let mut file = File::create(&ignore_file_path).unwrap();
        writeln!(file, "*.log").unwrap();
        writeln!(file, "!important.log").unwrap();
        writeln!(file, "temp/").unwrap();
        writeln!(file, "**/cache/**").unwrap();

        let rules = GitIgnoreRules::from_ignore_file(&ignore_file_path);

        // Test ignored files
        assert!(rules.rules[0].file_matches(dir.join("error.log").as_path(), &dir));
        assert!(rules.rules[1].file_matches(dir.join("important.log").as_path(), &dir));
        assert!(rules.rules[1].is_negated);
        assert!(rules.rules[2].file_matches(dir.join("temp/file.txt").as_path(), &dir));
        assert!(rules.rules[3].file_matches(dir.join("foo/cache/bar").as_path(), &dir));
        assert!(rules.rules[3].file_matches(dir.join("foo/cache/bar/baz").as_path(), &dir));
        // FIXME: I guess in theory the following test should work.
        // Though here we do not care much about directories, files updates are only for files
        // assert!(rules.rules[3].file_matches(dir.join("foo/cache/").as_path(), &dir));
        assert!(!rules.rules[3].file_matches(dir.join("foo/cache").as_path(), &dir));
    }
}
