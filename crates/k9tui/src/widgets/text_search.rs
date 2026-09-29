use ratatui::text::{Line, Span};

use crate::theme;

/// Lines matching `query` (case-insensitive substring), with the match
/// highlighted. Empty query returns everything unfiltered.
#[must_use]
pub fn filter_and_highlight<'a>(
    lines: &[Line<'a>],
    query: &str,
) -> Vec<Line<'a>> {
    if query.is_empty() {
        return lines.to_vec();
    }
    let needle = query.to_lowercase();
    lines
        .iter()
        .filter(|line| {
            line.spans
                .iter()
                .any(|s| s.content.to_lowercase().contains(&needle))
        })
        .map(|line| highlight_line(line, &needle))
        .collect()
}

fn highlight_line<'a>(line: &Line<'a>, needle: &str) -> Line<'a> {
    let mut spans = Vec::new();
    for span in &line.spans {
        let text = span.content.as_ref();
        let lower = text.to_lowercase();
        let mut rest = text;
        let mut lower_rest = lower.as_str();
        let mut offset = 0;
        while let Some(pos) = lower_rest.find(needle) {
            let byte_pos = offset + pos;
            let before = &text[offset..byte_pos];
            if !before.is_empty() {
                spans.push(Span::styled(before.to_string(), span.style));
            }
            let matched = &text[byte_pos..byte_pos + needle.len()];
            spans.push(Span::styled(matched.to_string(), theme::accent()));
            offset = byte_pos + needle.len();
            rest = &text[offset..];
            lower_rest = &lower[offset..];
        }
        if !rest.is_empty() {
            spans.push(Span::styled(rest.to_string(), span.style));
        }
    }
    Line::from(spans)
}
