//! Pattern scanning over the client executable.
//!
//! Every pattern must match exactly once. A zero-match means the build is unsupported or the
//! file is already patched; a multi-match means the signature is ambiguous and picking one
//! arbitrarily would corrupt the client. Both are hard errors, never warnings.

use anyhow::{bail, Result};

/// All offsets at which `pattern` occurs in `haystack`.
pub fn find_all(haystack: &[u8], pattern: &[u8]) -> Vec<usize> {
    if pattern.is_empty() || pattern.len() > haystack.len() {
        return Vec::new();
    }

    haystack
        .windows(pattern.len())
        .enumerate()
        .filter(|(_, window)| *window == pattern)
        .map(|(offset, _)| offset)
        .collect()
}

/// The single offset at which `pattern` occurs, or an error naming the problem.
pub fn find_unique(haystack: &[u8], pattern: &[u8], name: &str) -> Result<usize> {
    find_unique_outside(haystack, pattern, name, None)
}

/// Like [`find_unique`], but ignoring any match that starts within `exclude`.
///
/// Used for the portal string, which appears once as a standalone template *and* many times inside
/// the embedded cert bundle (as `us.actual.battle.net` etc.). The bundle region is replaced
/// wholesale by its own patch, so excluding it leaves the single standalone occurrence.
pub fn find_unique_outside(
    haystack: &[u8],
    pattern: &[u8],
    name: &str,
    exclude: Option<std::ops::Range<usize>>,
) -> Result<usize> {
    let matches: Vec<usize> = find_all(haystack, pattern)
        .into_iter()
        .filter(|&off| exclude.as_ref().is_none_or(|r| !r.contains(&off)))
        .collect();

    match matches.len() {
        1 => Ok(matches[0]),
        0 => bail!(
            "pattern '{name}' not found — this client build is unsupported, or the file has \
             already been patched"
        ),
        n => bail!(
            "pattern '{name}' matched {n} times at {:x?}; refusing to guess which one to patch",
            matches
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_single_match() {
        let data = b"aaaaNEEDLEbbbb";
        assert_eq!(find_unique(data, b"NEEDLE", "needle").unwrap(), 4);
    }

    #[test]
    fn finds_match_at_the_very_end() {
        let data = b"aaaaNEEDLE";
        assert_eq!(find_unique(data, b"NEEDLE", "needle").unwrap(), 4);
    }

    #[test]
    fn rejects_a_missing_pattern() {
        let err = find_unique(b"aaaa", b"NEEDLE", "needle").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn rejects_an_ambiguous_pattern() {
        let err = find_unique(b"NEEDLExxNEEDLE", b"NEEDLE", "needle").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("matched 2 times"), "unexpected message: {msg}");
    }

    #[test]
    fn pattern_longer_than_haystack_is_not_found() {
        assert!(find_all(b"ab", b"abcdef").is_empty());
    }

    #[test]
    fn empty_pattern_never_matches() {
        assert!(find_all(b"abc", b"").is_empty());
    }
}
