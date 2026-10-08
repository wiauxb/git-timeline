//! A compiled `/` search query: regex with smart case, hash-prefix matching.

use std::ops::Range;

use regex::{Regex, RegexBuilder};

#[derive(Debug)]
pub struct Query {
    pattern: String,
    re: Regex,
    /// `\A(?:<pattern>)`, case-insensitive: hashes only match by prefix.
    hash_re: Regex,
}

impl Query {
    /// Err = the regex parse error, as one line. The pattern must not be empty.
    pub fn compile(pattern: &str) -> Result<Query, String> {
        let re = RegexBuilder::new(pattern).case_insensitive(!smart_case(pattern)).build().map_err(one_line)?;
        let hash_re = RegexBuilder::new(&format!(r"\A(?:{pattern})"))
            .case_insensitive(true)
            .build()
            .map_err(one_line)?;
        Ok(Query { pattern: pattern.to_string(), re, hash_re })
    }

    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Byte ranges of the non-empty, non-overlapping matches.
    pub fn find_iter<'a>(&'a self, hay: &'a str) -> impl Iterator<Item = Range<usize>> + 'a {
        self.re.find_iter(hay).filter(|m| !m.is_empty()).map(|m| m.range())
    }

    pub fn is_match(&self, hay: &str) -> bool {
        self.find_iter(hay).next().is_some()
    }

    /// Length of the non-empty match anchored at the start of `hex`.
    pub fn hash_prefix(&self, hex: &str) -> Option<usize> {
        self.hash_re.find(hex).map(|m| m.len()).filter(|&n| n > 0)
    }
}

/// True ⇒ case-sensitive: the pattern has an uppercase letter not right after a `\`.
fn smart_case(pattern: &str) -> bool {
    let mut escaped = false;
    for c in pattern.chars() {
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c.is_uppercase() {
            return true;
        }
    }
    false
}

/// `regex` syntax errors span several lines (pattern + caret + message); keep the message.
fn one_line(e: regex::Error) -> String {
    let s = e.to_string();
    let last = s.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("invalid regex");
    last.trim().trim_start_matches("error: ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smart_case_rules() {
        let q = Query::compile("foo").unwrap();
        assert!(q.is_match("a FOO b"));
        let q = Query::compile("Foo").unwrap();
        assert!(!q.is_match("a FOO b"));
        assert!(q.is_match("a Foo b"));
        let q = Query::compile(r"\Sfoo").unwrap();
        assert!(q.is_match("xFOO"));
    }

    #[test]
    fn empty_matches_ignored() {
        let q = Query::compile("a*").unwrap();
        assert!(!q.is_match("bbb"));
        assert_eq!(q.find_iter("baab").collect::<Vec<_>>(), vec![1..3]);
        assert!(!Query::compile("^").unwrap().is_match("x"));
    }

    #[test]
    fn hash_prefix_only() {
        let hex = "5b089ca0000000000000000000000000000000ca";
        assert_eq!(Query::compile("5b08").unwrap().hash_prefix(hex), Some(4));
        assert_eq!(Query::compile("5B08").unwrap().hash_prefix(hex), Some(4));
        assert_eq!(Query::compile("ca").unwrap().hash_prefix(hex), None);
        assert_eq!(Query::compile("x*").unwrap().hash_prefix(hex), None);
    }

    #[test]
    fn invalid_regex_message() {
        let e = Query::compile("(").unwrap_err();
        assert!(!e.contains('\n'));
        assert!(e.contains("unclosed group"), "{e}");
    }
}
