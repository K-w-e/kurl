use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub fn highlight_json_line(line_idx: usize, line: &str, search_query: Option<&str>) -> Line<'static> {
    let mut spans = Vec::new();

    let prefix_span = Span::styled(
        format!("{:>4} │ ", line_idx + 1),
        Style::default().fg(Color::DarkGray),
    );
    spans.push(prefix_span);

    let trimmed = line.trim();

    if let Some((key_part, val_part)) = split_json_key_value(trimmed) {
        let leading_spaces = line.len() - line.trim_start().len();
        if leading_spaces > 0 {
            spans.push(Span::raw(" ".repeat(leading_spaces)));
        }

        spans.push(Span::styled(
            key_part.to_string(),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(": ", Style::default().fg(Color::DarkGray)));

        let val_spans = highlight_value(val_part);
        spans.extend(val_spans);
    } else {
        spans.extend(highlight_value(line));
    }

    if let Some(query) = search_query {
        if !query.is_empty() {
            return apply_search_highlight(spans, query);
        }
    }

    Line::from(spans)
}

fn split_json_key_value(line: &str) -> Option<(&str, &str)> {
    if line.starts_with('"') {
        if let Some(end_quote) = line[1..].find('"') {
            let key = &line[0..=end_quote + 1];
            let remainder = line[end_quote + 2..].trim_start();
            if remainder.starts_with(':') {
                let val = remainder[1..].trim_start();
                return Some((key, val));
            }
        }
    }
    None
}

fn highlight_value(val_str: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let trimmed = val_str.trim();

    let is_comma_ended = trimmed.ends_with(',');
    let core_val = if is_comma_ended {
        &trimmed[..trimmed.len() - 1]
    } else {
        trimmed
    };

    if core_val.starts_with('"') && core_val.ends_with('"') {
        spans.push(Span::styled(
            core_val.to_string(),
            Style::default().fg(Color::Green),
        ));
    } else if core_val == "true" || core_val == "false" {
        spans.push(Span::styled(
            core_val.to_string(),
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        ));
    } else if core_val == "null" {
        spans.push(Span::styled(
            core_val.to_string(),
            Style::default().fg(Color::Red),
        ));
    } else if core_val.parse::<f64>().is_ok() {
        spans.push(Span::styled(
            core_val.to_string(),
            Style::default().fg(Color::Yellow),
        ));
    } else {
        spans.push(Span::styled(
            val_str.to_string(),
            Style::default().fg(Color::White),
        ));
        return spans;
    }

    if is_comma_ended {
        spans.push(Span::styled(",", Style::default().fg(Color::DarkGray)));
    }

    spans
}

fn apply_search_highlight(spans: Vec<Span<'static>>, query: &str) -> Line<'static> {
    let mut new_spans = Vec::new();
    let query_lower = query.to_lowercase();

    for span in spans {
        let content = span.content.into_owned();
        let content_lower = content.to_lowercase();

        if let Some(pos) = content_lower.find(&query_lower) {
            let before = &content[..pos];
            let matched = &content[pos..pos + query.len()];
            let after = &content[pos + query.len()..];

            if !before.is_empty() {
                new_spans.push(Span::styled(before.to_string(), span.style));
            }
            new_spans.push(Span::styled(
                matched.to_string(),
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ));
            if !after.is_empty() {
                new_spans.push(Span::styled(after.to_string(), span.style));
            }
        } else {
            new_spans.push(Span::styled(content, span.style));
        }
    }

    Line::from(new_spans)
}
