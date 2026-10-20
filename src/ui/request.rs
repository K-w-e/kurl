use super::theme::Theme;
use crate::app::session::RequestTabSession;
use crate::app::App;
use crate::events::{Pane, RequestSubtab};
use crate::models::AuthConfig;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, Tabs};
use ratatui::Frame;

pub fn render(frame: &mut Frame, app: &App, url_area: Rect, tabs_area: Rect, editor_area: Rect) {
    if let Some(session) = app.current_session() {
        render_url_bar(frame, app, session, url_area);
        render_tabs(frame, app, session, tabs_area);
        render_tab_content(frame, app, session, editor_area);
    }
}

fn render_url_bar(frame: &mut Frame, app: &App, session: &RequestTabSession, area: Rect) {
    let is_focused = app.focused_pane == Pane::UrlBar;
    let border_color = if is_focused {
        Theme::BORDER_ACTIVE
    } else {
        Theme::BORDER_INACTIVE
    };

    let title = Line::from(vec![
        Span::styled(" [2] ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "Request URL",
            Style::default()
                .fg(Theme::TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" (Enter/Ctrl+R: send) ", Style::default().fg(Color::DarkGray)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let method_style = Theme::method_style(&session.request.method);
    let method_str = format!(" {} ", session.request.method.as_str());

    let mut url_spans = vec![
        Span::styled(method_str, method_style.bg(Color::Rgb(30, 30, 35))),
        Span::raw(" "),
    ];

    if session.request.url.is_empty() {
        url_spans.push(Span::styled(
            "https://api.example.com/v1/resource",
            Style::default().fg(Color::DarkGray),
        ));
    } else {
        url_spans.push(Span::styled(
            &session.request.url,
            Style::default().fg(Color::White),
        ));
    }

    let url_paragraph = Paragraph::new(Line::from(url_spans)).block(block);
    frame.render_widget(url_paragraph, area);

    if is_focused {
        let cursor_x = area.x
            + 1
            + session.request.method.as_str().len() as u16
            + 3
            + session.url_cursor_pos as u16;
        let cursor_y = area.y + 1;
        if cursor_x < area.x + area.width - 1 {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

fn render_tabs(frame: &mut Frame, app: &App, session: &RequestTabSession, area: Rect) {
    let is_focused = app.focused_pane == Pane::RequestTabs;
    let border_color = if is_focused {
        Theme::BORDER_ACTIVE
    } else {
        Theme::BORDER_INACTIVE
    };

    let auth_badge = if session.request.auth.is_none() {
        "Auth"
    } else {
        "Auth *"
    };

    let tab_titles = vec![
        Line::from(format!("Query ({})", session.request.params.len())),
        Line::from(format!("Headers ({})", session.request.headers.len())),
        Line::from(auth_badge),
        Line::from("Body"),
        Line::from(format!("Path ({})", session.request.path_params.len())),
    ];

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .title(Line::from(vec![
                    Span::styled(" [3] ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        "Payload Config",
                        Style::default()
                            .fg(Theme::TEXT_PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" (a: add, e: edit, d: del) ", Style::default().fg(Color::DarkGray)),
                ])),
        )
        .select(session.request_subtab.index())
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

fn render_tab_content(frame: &mut Frame, app: &App, session: &RequestTabSession, area: Rect) {
    let is_focused = app.focused_pane == Pane::RequestEditor;
    let border_color = if is_focused {
        Theme::BORDER_ACTIVE
    } else {
        Theme::BORDER_INACTIVE
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    match session.request_subtab {
        RequestSubtab::Query => {
            render_key_value_table(
                frame,
                &session.request.params,
                session.table_selected_row,
                is_focused,
                "Query Parameters",
                "Param Key",
                "Param Value",
                block,
                area,
            );
        }
        RequestSubtab::Headers => {
            render_key_value_table(
                frame,
                &session.request.headers,
                session.table_selected_row,
                is_focused,
                "Request Headers",
                "Header Name",
                "Header Value",
                block,
                area,
            );
        }
        RequestSubtab::Path => {
            render_key_value_table(
                frame,
                &session.request.path_params,
                session.table_selected_row,
                is_focused,
                "Path Parameters (:name)",
                "Token",
                "Resolved Value",
                block,
                area,
            );
        }
        RequestSubtab::Auth => {
            let auth_title = Line::from(vec![
                Span::styled("Authentication", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(" (m: cycle type, e: edit fields)", Style::default().fg(Color::DarkGray)),
            ]);

            let mut lines = Vec::new();
            lines.push(Line::from(vec![
                Span::styled("Type: ", Style::default().fg(Theme::TEXT_MUTED)),
                Span::styled(
                    format!(" [ {} ] ", session.request.auth.type_name()),
                    Style::default()
                        .bg(Color::Rgb(30, 45, 65))
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  (Press 'm' to cycle type)", Style::default().fg(Color::DarkGray)),
            ]));
            lines.push(Line::from(""));

            match &session.request.auth {
                AuthConfig::None => {
                    lines.push(Line::from(Span::styled(
                        "No authentication configured for this request.",
                        Style::default().fg(Color::DarkGray),
                    )));
                    lines.push(Line::from(Span::styled(
                        "Press 'm' to select Bearer Token, Basic Auth, or API Key.",
                        Style::default().fg(Theme::TEXT_MUTED),
                    )));
                }
                AuthConfig::Bearer { token } => {
                    let display_token = if token.is_empty() {
                        "<none - press 'e' to set token>"
                    } else {
                        token.as_str()
                    };
                    lines.push(Line::from(vec![
                        Span::styled("Token: ", Style::default().fg(Color::Yellow)),
                        Span::styled(display_token, Style::default().fg(Color::White)),
                    ]));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Will inject header: Authorization: Bearer <token>",
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                AuthConfig::Basic { username, password } => {
                    let display_pass = "*".repeat(password.len());
                    lines.push(Line::from(vec![
                        Span::styled("Username: ", Style::default().fg(Color::Yellow)),
                        Span::styled(username.as_str(), Style::default().fg(Color::White)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Password: ", Style::default().fg(Color::Yellow)),
                        Span::styled(display_pass, Style::default().fg(Color::White)),
                    ]));
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Will inject header: Authorization: Basic <base64>",
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                AuthConfig::ApiKey {
                    key,
                    value,
                    in_header,
                } => {
                    let loc = if *in_header { "Header" } else { "Query Parameter" };
                    lines.push(Line::from(vec![
                        Span::styled("Key Name: ", Style::default().fg(Color::Yellow)),
                        Span::styled(key.as_str(), Style::default().fg(Color::White)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Value:    ", Style::default().fg(Color::Yellow)),
                        Span::styled(value.as_str(), Style::default().fg(Color::White)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Location: ", Style::default().fg(Color::Yellow)),
                        Span::styled(loc, Style::default().fg(Color::Cyan)),
                    ]));
                }
            }

            let p = Paragraph::new(lines).block(block.title(auth_title));
            frame.render_widget(p, area);
        }
        RequestSubtab::Body => {
            let body_content = session
                .request
                .body
                .as_ref()
                .and_then(|b| b.content.as_deref())
                .unwrap_or("");

            let text = if body_content.is_empty() {
                Span::styled(
                    "(No payload defined - type text to edit body)",
                    Style::default().fg(Color::DarkGray),
                )
            } else {
                Span::styled(body_content, Style::default().fg(Color::White))
            };

            let paragraph = Paragraph::new(text).block(
                block.title(Line::from(vec![
                    Span::styled("Body Content (Raw/JSON)", Style::default().add_modifier(Modifier::BOLD)),
                ])),
            );

            frame.render_widget(paragraph, area);
        }
    }
}

fn render_key_value_table(
    frame: &mut Frame,
    items: &[crate::models::KeyValuePair],
    selected_row: usize,
    is_focused: bool,
    title: &'static str,
    key_col: &'static str,
    val_col: &'static str,
    block: Block,
    area: Rect,
) {
    let rows: Vec<Row> = items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let is_selected = idx == selected_row;
            let status_checkbox = if item.enabled { "[x]" } else { "[ ]" };
            let status_style = if item.enabled {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let row_style = if is_selected && is_focused {
                Style::default().bg(Theme::HIGHLIGHT_BG)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(Span::styled(status_checkbox, status_style)),
                Cell::from(item.name.as_str()),
                Cell::from(item.value.as_str()),
            ])
            .style(row_style)
        })
        .collect();

    let header = Row::new(vec![
        Cell::from("Active").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from(key_col).style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from(val_col).style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .bottom_margin(1);

    let widths = [
        Constraint::Length(8),
        Constraint::Percentage(40),
        Constraint::Percentage(50),
    ];

    let full_title = Line::from(vec![
        Span::styled(title, Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(" [a: Add, e: Edit, d: Del, Space: Toggle]", Style::default().fg(Color::DarkGray)),
    ]);

    let table = Table::new(rows, widths).header(header).block(block.title(full_title));
    frame.render_widget(table, area);
}
