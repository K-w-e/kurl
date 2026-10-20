use super::theme::Theme;
use crate::app::session::RequestTabSession;
use crate::app::App;
use crate::events::{Pane, ResponseSubtab};
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, Tabs};
use ratatui::Frame;

pub fn render(frame: &mut Frame, app: &App, status_area: Rect, tabs_area: Rect, content_area: Rect) {
    if let Some(session) = app.current_session() {
        render_status_bar(frame, app, session, status_area);
        render_tabs(frame, app, session, tabs_area);
        render_content(frame, app, session, content_area);
    }
}

fn render_status_bar(frame: &mut Frame, app: &App, session: &RequestTabSession, area: Rect) {
    let is_focused = app.focused_pane == Pane::ResponseTabs;
    let border_color = if is_focused {
        Theme::BORDER_ACTIVE
    } else {
        Theme::BORDER_INACTIVE
    };

    let title = Line::from(vec![
        Span::styled(" [4] ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "Response",
            Style::default()
                .fg(Theme::TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let mut spans = Vec::new();

    if session.is_loading {
        let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let frame_char = spinner_frames[app.spinner_frame % spinner_frames.len()];
        spans.push(Span::styled(
            format!(" {} Executing request... ", frame_char),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));
    } else if let Some(err) = &session.error_message {
        spans.push(Span::styled(
            " ERROR ",
            Style::default().bg(Color::Red).fg(Color::White).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(err, Style::default().fg(Color::LightRed)));
    } else if let Some(res) = &session.active_response {
        let status_style = Theme::status_style(res.status_code);
        spans.push(Span::styled(
            format!(" {} {} ", res.status_code, res.status_text),
            status_style.bg(Color::Rgb(30, 30, 35)),
        ));
        spans.push(Span::raw("   "));

        spans.push(Span::styled(
            format!("⏱ {} ms", res.duration_ms),
            Style::default().fg(Color::Cyan),
        ));
        spans.push(Span::raw("   "));

        let size_kb = (res.body_size_bytes as f64) / 1024.0;
        spans.push(Span::styled(
            format!("📦 {:.2} KB", size_kb),
            Style::default().fg(Color::White),
        ));
        spans.push(Span::raw("   "));

        if let Some(ct) = &res.content_type {
            spans.push(Span::styled(
                format!("📄 {}", ct),
                Style::default().fg(Theme::TEXT_MUTED),
            ));
        }
    } else {
        spans.push(Span::styled(
            "Ready. Press [Enter] or [Ctrl+R] to send request",
            Style::default().fg(Theme::TEXT_MUTED),
        ));
    }

    let paragraph = Paragraph::new(Line::from(spans)).block(block);
    frame.render_widget(paragraph, area);
}

fn render_tabs(frame: &mut Frame, app: &App, session: &RequestTabSession, area: Rect) {
    let is_focused = app.focused_pane == Pane::ResponseTabs;
    let border_color = if is_focused {
        Theme::BORDER_ACTIVE
    } else {
        Theme::BORDER_INACTIVE
    };

    let headers_count = session
        .active_response
        .as_ref()
        .map(|r| r.headers.len())
        .unwrap_or(0);

    let tab_titles = vec![
        Line::from("Body"),
        Line::from(format!("Headers ({})", headers_count)),
    ];

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .title("View Mode"),
        )
        .select(session.response_subtab.index())
        .style(Style::default().fg(Theme::TEXT_MUTED))
        .highlight_style(
            Style::default()
                .fg(Theme::SELECTION_FG)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
        )
        .divider(Span::styled(" | ", Style::default().fg(Color::DarkGray)));

    frame.render_widget(tabs, area);
}

fn render_content(frame: &mut Frame, app: &App, session: &RequestTabSession, area: Rect) {
    let is_focused = app.focused_pane == Pane::ResponseViewer;
    let border_color = if is_focused {
        Theme::BORDER_ACTIVE
    } else {
        Theme::BORDER_INACTIVE
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    if let Some(res) = &session.active_response {
        match session.response_subtab {
            ResponseSubtab::Body => {
                let formatted = res.formatted_body();
                let lines: Vec<Line> = formatted
                    .lines()
                    .enumerate()
                    .map(|(i, line)| {
                        super::highlighter::highlight_json_line(
                            i,
                            line,
                            session.search_query.as_deref(),
                        )
                    })
                    .collect();

                let mut title_spans = vec![
                    Span::styled("Response Payload", Style::default().add_modifier(Modifier::BOLD)),
                ];

                if let Some(query) = &session.search_query {
                    let total = session.search_matches.len();
                    let current = if total > 0 {
                        session.selected_match_idx + 1
                    } else {
                        0
                    };
                    title_spans.push(Span::styled(
                        format!(" [Search: \"{}\" ({}/{} matches) - n/N: jump, Esc: clear]", query, current, total),
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ));
                } else {
                    title_spans.push(Span::styled(
                        " ('/': search, y: copy body, ↑/↓: scroll)",
                        Style::default().fg(Color::DarkGray),
                    ));
                }

                let paragraph = Paragraph::new(lines)
                    .block(block.title(Line::from(title_spans)))
                    .scroll((session.response_scroll, 0));

                frame.render_widget(paragraph, area);
            }
            ResponseSubtab::Headers => {
                let rows: Vec<Row> = res
                    .headers
                    .iter()
                    .map(|header| {
                        Row::new(vec![
                            Cell::from(header.name.as_str())
                                .style(Style::default().fg(Color::Cyan)),
                            Cell::from(header.value.as_str()),
                        ])
                    })
                    .collect();

                let header = Row::new(vec![
                    Cell::from("Header Name").style(Style::default().add_modifier(Modifier::BOLD)),
                    Cell::from("Header Value").style(Style::default().add_modifier(Modifier::BOLD)),
                ])
                .bottom_margin(1);

                let widths = [Constraint::Percentage(35), Constraint::Percentage(65)];

                let table = Table::new(rows, widths).header(header).block(
                    block.title(Line::from(vec![
                        Span::styled("Response Headers", Style::default().add_modifier(Modifier::BOLD)),
                    ])),
                );

                frame.render_widget(table, area);
            }
        }
    } else if let Some(err) = &session.error_message {
        let error_text = vec![
            Line::from(Span::styled("Connection or HTTP Error:", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(Span::styled(err, Style::default().fg(Color::LightRed))),
            Line::from(""),
            Line::from(Span::styled("Suggestions:", Style::default().fg(Color::Yellow))),
            Line::from("- Verify the URL and protocol scheme (http:// or https://)"),
            Line::from("- Check if target host is reachable and DNS resolves"),
            Line::from("- Confirm that endpoint headers and payload are valid"),
        ];

        let paragraph = Paragraph::new(error_text).block(block.title("Error Diagnostics"));
        frame.render_widget(paragraph, area);
    } else {
        let placeholder = vec![
            Line::from(Span::styled("No response data yet.", Style::default().fg(Color::DarkGray))),
            Line::from(""),
            Line::from(Span::styled("Shortcuts:", Style::default().fg(Theme::TEXT_MUTED))),
            Line::from("  [Enter] in URL bar or [Ctrl+R] to send request"),
            Line::from("  [Ctrl+Tab] Cycle open request tabs"),
            Line::from("  [Ctrl+W] Close current request tab"),
            Line::from("  [Ctrl+T] Open new request tab"),
            Line::from("  [1-4] Quick jump between panels"),
            Line::from("  [m] Cycle HTTP method (GET, POST, etc.)"),
            Line::from("  [q] Quit application"),
        ];
        let paragraph = Paragraph::new(placeholder).block(block.title("Awaiting Request"));
        frame.render_widget(paragraph, area);
    }
}
