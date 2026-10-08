# `src/model/search.rs`

## Purpose
A compiled search query (`/` in the UI): regex with smart case, plus the matching
primitives used for commit hashes, subjects and file lines. Pure, no state.

## Contents

```rust
pub struct Query {
    pattern: String,   // as typed
    re: Regex,         // smart-case applied
    hash_re: Regex,    // `\A(?:<pattern>)`, case-insensitive: hashes match by prefix
}

impl Query {
    /// Err = the regex parse error, as one line (shown in the search prompt).
    /// The caller never passes an empty pattern (empty = "clear the search").
    pub fn compile(pattern: &str) -> Result<Query, String>;
    pub fn pattern(&self) -> &str;
    /// Byte ranges of the non-empty, non-overlapping matches in `hay`.
    pub fn find_iter<'a>(&'a self, hay: &'a str) -> impl Iterator<Item = Range<usize>> + 'a;
    /// At least one non-empty match.
    pub fn is_match(&self, hay: &str) -> bool;
    /// Length of the non-empty match anchored at the start of `hex`, if any.
    pub fn hash_prefix(&self, hex: &str) -> Option<usize>;
}

fn smart_case(pattern: &str) -> bool;   // true ⇒ case-sensitive
```

## Specs
* Syntax: the `regex` crate (no look-around / backreferences). Size limits stay at the
  crate defaults, so a huge pattern is a compile error, not a hang.
* **Smart case**: case-insensitive unless the pattern contains an uppercase letter that
  is not right after a `\` (so `\S`, `\W`, `\D` don't count). `Foo` is case-sensitive;
  `foo` and `\Sfoo` are not.
* **Empty matches are ignored** (`a*`, `^`, `x?`): a hay "matches" only with a match of
  length ≥ 1. This prevents a match on every line.
* **Hashes match by prefix, case-insensitively**: `hash_prefix("5b08")` matches
  `5b089ca…` but `"ca"` does not. Without this, common words that are valid hex (`add`,
  `face`, `bad`) would match random commits.

## Tests
`smart_case_rules`, `empty_matches_ignored`, `hash_prefix_only`, `invalid_regex_message`.
