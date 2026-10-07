//! k9s-style `:` command resolution (pure).

use crate::app_state::ResourceKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Empty,
    View(ResourceKind),
    Quit,
    Unknown,
}

/// Accepted words in suggestion order; anything else resolves by unique prefix.
const NAMES: &[(&str, Command)] = &[
    ("containers", Command::View(ResourceKind::Containers)),
    ("c", Command::View(ResourceKind::Containers)),
    ("images", Command::View(ResourceKind::Images)),
    ("i", Command::View(ResourceKind::Images)),
    ("volumes", Command::View(ResourceKind::Volumes)),
    ("v", Command::View(ResourceKind::Volumes)),
    ("networks", Command::View(ResourceKind::Networks)),
    ("n", Command::View(ResourceKind::Networks)),
    ("quit", Command::Quit),
    ("q", Command::Quit),
];

pub fn resolve(input: &str) -> Command {
    let s = input.trim().to_ascii_lowercase();
    if s.is_empty() {
        return Command::Empty;
    }
    if let Some((_, c)) = NAMES.iter().find(|(n, _)| *n == s) {
        return *c;
    }
    let mut hits = NAMES.iter().filter(|(n, _)| n.starts_with(&s));
    match (hits.next(), hits.next()) {
        (Some((_, c)), None) => *c,
        _ => Command::Unknown,
    }
}

/// Full-word completion of the typed prefix.
pub fn suggest(input: &str) -> Option<&'static str> {
    let s = input.to_ascii_lowercase();
    if s.is_empty() {
        return None;
    }
    NAMES
        .iter()
        .map(|(n, _)| *n)
        .find(|n| n.len() > s.len() && n.starts_with(&s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_prefixes_unknown() {
        let v = Command::View;
        assert_eq!(resolve("c"), v(ResourceKind::Containers));
        assert_eq!(resolve("IMAGES"), v(ResourceKind::Images));
        assert_eq!(resolve("vol"), v(ResourceKind::Volumes));
        assert_eq!(resolve(" net "), v(ResourceKind::Networks));
        assert_eq!(resolve("q"), Command::Quit);
        assert_eq!(resolve("qu"), Command::Quit);
        assert_eq!(resolve(""), Command::Empty);
        assert_eq!(resolve("x"), Command::Unknown);
        assert_eq!(resolve("co"), v(ResourceKind::Containers));
    }

    #[test]
    fn suggestions() {
        assert_eq!(suggest("im"), Some("images"));
        assert_eq!(suggest("images"), None);
        assert_eq!(suggest(""), None);
        assert_eq!(suggest("zz"), None);
    }

    #[test]
    fn edge_inputs_never_panic_and_resolve_sanely() {
        let v = Command::View;
        assert_eq!(resolve("   "), Command::Empty);
        assert_eq!(resolve("  images  "), v(ResourceKind::Images));
        assert_eq!(resolve("\timages\n"), v(ResourceKind::Images));
        assert_eq!(resolve("Q"), Command::Quit);
        for junk in ["🦀", "ünï", "图像", "i m", "imagesx", "::", "1", "-1"]
        {
            assert_eq!(resolve(junk), Command::Unknown, "{junk}");
        }
        // Non-ASCII uppercase is left alone (ASCII-only folding), no panic.
        assert_eq!(resolve("İMAGES"), Command::Unknown);
        assert_eq!(suggest("🦀"), None);
        assert_eq!(suggest("İ"), None);
        assert_eq!(suggest("IM"), Some("images"));
        assert_eq!(suggest(" im"), None, "leading space: no ghost");
    }
}
