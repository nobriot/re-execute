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
        let line = if is_negated { &line[1..] } else { line };

        let mut chars =
            if is_negated { line[1..].chars().peekable() } else { line.chars().peekable() };

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
                _ => {
                    // Handle literals
                    let mut literal = c.to_string();
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

        Some(GitIgnoreRule { pattern, is_negated })
    }

    /// Checks if the current git ignore rule matches a file within a dir
    pub fn file_matches<D>(&self, file: &Path, dir: &D) -> bool
    where
        D: AsRef<Path>,
    {
        // We take the part of the file that is relative to the dir
        let candidate = match file.strip_prefix(dir) {
            Ok(path) => path.to_string_lossy(),
            Err(_) => return false,
        };

        self.string_matches(candidate.as_ref(), &self.pattern)
    }

    /// Checks if a file name (string) is matching a collection of GitIgnoreRule
    fn string_matches(&self, file: &str, rule: &Vec<GitIgnoreRuleElements>) -> bool {
        let mut p_chars = file.chars().peekable();
        let mut rule_elements = rule.iter().peekable();

        while let Some(rule_element) = rule_elements.next() {
            match rule_element {
                GitIgnoreRuleElements::Literal(l) => {
                    // Match all chars from the literal:
                    let mut l_chars = l.chars();
                    while let Some(l_char) = l_chars.next() {
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
                    if p_chars.next() != Some('/') {
                        return false;
                    }
                }
                GitIgnoreRuleElements::Asterisk => {
                    // Consume until the next '/' or end of string
                    while let Some(&c) = p_chars.peek() {
                        if c == '/' {
                            break;
                        }
                        p_chars.next();
                    }
                }
                GitIgnoreRuleElements::DoubleAsterisk => {
                    // Try to match the rest, including accross directories
                    if rule_elements.peek().is_none() {
                        // No more rules, after the **, so it matches anything really.
                        return true;
                    }
                    // Else pick up the remaining rules:
                    // There is probably a better way than cloning here...
                    let remaining_rules: Vec<_> = rule_elements.map(|r| r.clone()).collect();

                    // Now try to fit the remainder of the string with the rules
                    let file: String = p_chars.collect();
                    for i in 0..file.len() {
                        if self.string_matches(&file[i..], &remaining_rules) {
                            return true;
                        }
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
                            matched = !negated;
                        }
                    }
                    if !matched {
                        return false;
                    }
                }
            }
        }

        // We have a match if we consumed all chars from the candidate path
        p_chars.next().is_none()
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
