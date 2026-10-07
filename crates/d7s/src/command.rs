//! k9s-style `:` command resolution. Pure: no app state, easy to test.

/// A view/action a bare command word maps to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    Connections,
    Schemas,
    Tables,
    Columns,
    Sql,
    Log,
    Activity,
    Help,
    Quit,
}

/// Every accepted command word, in suggestion order.
const NAMES: &[(&str, Verb)] = &[
    ("connections", Verb::Connections),
    ("conn", Verb::Connections),
    ("schemas", Verb::Schemas),
    ("tables", Verb::Tables),
    ("columns", Verb::Columns),
    ("col", Verb::Columns),
    ("cols", Verb::Columns),
    ("sql", Verb::Sql),
    ("editor", Verb::Sql),
    ("log", Verb::Log),
    ("querylog", Verb::Log),
    ("activity", Verb::Activity),
    ("help", Verb::Help),
    ("quit", Verb::Quit),
    ("q", Verb::Quit),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Blank input: just close the bar.
    Empty,
    Verb(Verb),
    /// Purely numeric: jump to that 1-based row.
    Row(u64),
    /// Best table match first, then the other candidates of the same tier.
    Table(Vec<String>),
    /// Prefix matches several verbs (canonical names listed).
    Ambiguous(Vec<&'static str>),
    Unknown(String),
}

/// All names matching the query in the best non-empty tier: exact, then
/// prefix, then substring (case-insensitive); list order within a tier.
pub fn match_table_candidates<'a>(
    names: &[&'a str],
    query: &str,
) -> Vec<&'a str> {
    let needle = query.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    let tiers: [&dyn Fn(&str) -> bool; 3] =
        [&|n| n == needle, &|n| n.starts_with(&needle), &|n| {
            n.contains(&needle)
        }];
    for tier in tiers {
        let hits: Vec<&str> = names
            .iter()
            .copied()
            .filter(|n| tier(&n.to_ascii_lowercase()))
            .collect();
        if !hits.is_empty() {
            return hits;
        }
    }
    Vec::new()
}

/// Best single table match (exact > prefix > substring).
pub fn match_table_name<'a>(names: &[&'a str], query: &str) -> Option<&'a str> {
    match_table_candidates(names, query).first().copied()
}

/// Resolve typed command text. `tables` are the current schema's table names.
pub fn resolve(input: &str, tables: &[&str]) -> Command {
    let raw = input.trim();
    let lower = raw.to_ascii_lowercase();
    if lower.is_empty() {
        return Command::Empty;
    }
    if lower.bytes().all(|b| b.is_ascii_digit()) {
        return lower
            .parse()
            .map_or_else(|_| Command::Unknown(raw.into()), Command::Row);
    }
    if let Some((_, verb)) = NAMES.iter().find(|(n, _)| *n == lower) {
        return Command::Verb(*verb);
    }
    let mut prefixed: Vec<(&'static str, Verb)> = Vec::new();
    for (name, verb) in NAMES {
        if name.starts_with(&lower) && !prefixed.iter().any(|(_, v)| v == verb)
        {
            prefixed.push((name, *verb));
        }
    }
    if let [(_, verb)] = prefixed.as_slice() {
        return Command::Verb(*verb);
    }
    if match_table_name(tables, raw).is_some() {
        let hits = match_table_candidates(tables, raw);
        return Command::Table(hits.into_iter().map(String::from).collect());
    }
    if prefixed.is_empty() {
        Command::Unknown(raw.into())
    } else {
        Command::Ambiguous(prefixed.into_iter().map(|(n, _)| n).collect())
    }
}

/// Full-text completion for the typed prefix (verbs first, then tables).
pub fn suggest(input: &str, tables: &[&str]) -> Option<String> {
    let lower = input.trim_start().to_ascii_lowercase();
    if lower.is_empty() || lower != input.to_ascii_lowercase() {
        // Only complete plain prefixes (no leading space games).
        return None;
    }
    NAMES
        .iter()
        .map(|(n, _)| *n)
        .chain(tables.iter().copied())
        .find(|n| {
            n.len() > lower.len() && n.to_ascii_lowercase().starts_with(&lower)
        })
        .map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: [&str; 4] = ["accounts", "order_items", "orders", "users"];

    #[test]
    fn table_match_tiers() {
        let find = |q| match_table_name(&T, q);
        assert_eq!(find("orders"), Some("orders"));
        assert_eq!(find("ord"), Some("order_items"), "first prefix hit");
        assert_eq!(find("tems"), Some("order_items"));
        assert_eq!(find("USERS"), Some("users"));
        assert_eq!(find("nope"), None);
        assert_eq!(find("usr"), None, "no hardcoded aliases");
        assert_eq!(
            match_table_candidates(&T, "ord"),
            ["order_items", "orders"]
        );
    }

    #[test]
    fn verbs_and_aliases() {
        for (q, v) in [
            ("tables", Verb::Tables),
            ("conn", Verb::Connections),
            ("EDITOR", Verb::Sql),
            ("sql", Verb::Sql),
            ("querylog", Verb::Log),
            ("q", Verb::Quit),
            ("quit", Verb::Quit),
            ("cols", Verb::Columns),
        ] {
            assert_eq!(resolve(q, &[]), Command::Verb(v), "{q}");
        }
    }

    #[test]
    fn unique_prefix_resolves_ambiguous_does_not() {
        assert_eq!(resolve("sch", &[]), Command::Verb(Verb::Schemas));
        assert_eq!(resolve("act", &[]), Command::Verb(Verb::Activity));
        assert_eq!(resolve("l", &[]), Command::Verb(Verb::Log));
        assert_eq!(
            resolve("c", &[]),
            Command::Ambiguous(vec!["connections", "columns"])
        );
    }

    #[test]
    fn tables_row_blank_unknown() {
        assert_eq!(resolve("  ", &T), Command::Empty);
        assert_eq!(resolve("123", &T), Command::Row(123));
        assert_eq!(
            resolve("ord", &T),
            Command::Table(vec!["order_items".into(), "orders".into()])
        );
        assert_eq!(resolve("zzz", &T), Command::Unknown("zzz".into()));
        assert_eq!(
            resolve("99999999999999999999999", &T),
            Command::Unknown("99999999999999999999999".into())
        );
        // A table prefix beats an ambiguous verb prefix.
        assert_eq!(
            resolve("c", &["cats"]),
            Command::Table(vec!["cats".into()])
        );
    }

    #[test]
    fn suggestions() {
        assert_eq!(suggest("", &T), None);
        assert_eq!(suggest("tab", &T).as_deref(), Some("tables"));
        assert_eq!(suggest("us", &T).as_deref(), Some("users"));
        assert_eq!(suggest("tables", &T), None, "already complete");
        assert_eq!(suggest("zz", &T), None);
    }
}
