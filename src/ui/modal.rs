use crate::app::{App, Modal};
use crate::ui::theme::Theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn render(frame: &mut Frame, app: &App) {
    match &app.active_modal {
        Modal::None => {}
        Modal::NewRequest {
            name_input,
            selected_method,
            group_input,
            url_input,
            active_field,
        } => {
            let area = centered_rect(64, 18, frame.area());
            frame.render_widget(Clear, area);

            let block = Block::default()
                .title(Line::from(vec![
                    Span::styled(" + ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        "Create New Request",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Theme::BORDER_ACTIVE));

            let inner = block.inner(area);
            frame.render_widget(block, area);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(2),
                ])
                .split(inner);

            let field_style = |idx: usize| {
                if *active_field == idx {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                }
            };

            let name_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(field_style(0))
                .title(" Request Name ");
            let name_p = Paragraph::new(name_input.as_str()).block(name_block);
            frame.render_widget(name_p, rows[0]);

            let method_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(field_style(1))
                .title(" HTTP Method (←/→ or 'm' to change) ");
            let method_style = Theme::method_style(selected_method);
            let method_p = Paragraph::new(Line::from(vec![
                Span::raw(" [ "),
                Span::styled(selected_method.as_str(), method_style),
                Span::raw(" ] "),
            ]))
            .block(method_block);
            frame.render_widget(method_p, rows[1]);

            let url_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(field_style(2))
                .title(" Target URL ");
            let url_p = Paragraph::new(url_input.as_str()).block(url_block);
            frame.render_widget(url_p, rows[2]);

            let group_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(field_style(3))
                .title(" Collection Group / Subfolder (optional) ");
            let group_p = Paragraph::new(group_input.as_str()).block(group_block);
            frame.render_widget(group_p, rows[3]);

            let buttons = Paragraph::new(Line::from(vec![
                Span::styled(
                    " [ Enter: Create & Open ] ",
                    Style::default()
                        .bg(Color::Green)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("   "),
                Span::styled(
                    " [ Esc: Cancel ] ",
                    Style::default()
                        .bg(Color::DarkGray)
                        .fg(Color::White),
                ),
                Span::raw("   "),
                Span::styled(" [Tab] Next field ", Style::default().fg(Color::Gray)),
            ]));
            frame.render_widget(buttons, rows[4]);
        }
        Modal::NewGroup { name_input } => {
            let area = centered_rect(50, 10, frame.area());
            frame.render_widget(Clear, area);

            let block = Block::default()
                .title(Line::from(vec![
                    Span::styled(" 📁 ", Style::default()),
                    Span::styled(
                        "Create New Collection Group",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Theme::BORDER_ACTIVE));

            let inner = block.inner(area);
            frame.render_widget(block, area);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Length(2)])
                .split(inner);

            let name_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Group / Folder Name ");
            let name_p = Paragraph::new(name_input.as_str()).block(name_block);
            frame.render_widget(name_p, rows[0]);

            let buttons = Paragraph::new(Line::from(vec![
                Span::styled(
                    " [ Enter: Create ] ",
                    Style::default()
                        .bg(Color::Green)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("   "),
                Span::styled(
                    " [ Esc: Cancel ] ",
                    Style::default()
                        .bg(Color::DarkGray)
                        .fg(Color::White),
                ),
            ]));
            frame.render_widget(buttons, rows[1]);
        }
        Modal::EditTableRow {
            row_index,
            key_input,
            value_input,
            active_field,
            target_subtab,
        } => {
            let area = centered_rect(60, 12, frame.area());
            frame.render_widget(Clear, area);

            let action_title = if row_index.is_some() {
                format!("Edit Row in {:?}", target_subtab)
            } else {
                format!("Add Row to {:?}", target_subtab)
            };

            let block = Block::default()
                .title(Line::from(vec![
                    Span::styled(" ✎ ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        action_title,
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Theme::BORDER_ACTIVE));

            let inner = block.inner(area);
            frame.render_widget(block, area);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(2),
                ])
                .split(inner);

            let field_style = |idx: usize| {
                if *active_field == idx {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                }
            };

            let key_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(field_style(0))
                .title(" Key / Name ");
            let key_p = Paragraph::new(key_input.as_str()).block(key_block);
            frame.render_widget(key_p, rows[0]);

            let val_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(field_style(1))
                .title(" Value ");
            let val_p = Paragraph::new(value_input.as_str()).block(val_block);
            frame.render_widget(val_p, rows[1]);

            let buttons = Paragraph::new(Line::from(vec![
                Span::styled(
                    " [ Enter: Save ] ",
                    Style::default()
                        .bg(Color::Green)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("   "),
                Span::styled(
                    " [ Esc: Cancel ] ",
                    Style::default().bg(Color::DarkGray).fg(Color::White),
                ),
                Span::raw("   "),
                Span::styled(" [Tab] Switch field ", Style::default().fg(Color::Gray)),
            ]));
            frame.render_widget(buttons, rows[2]);
        }
        Modal::ImportUniversal { input } => {
            let area = centered_rect(75, 14, frame.area());
            frame.render_widget(Clear, area);

            let block = Block::default()
                .title(Line::from(vec![
                    Span::styled(" 📥 ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        "Universal Importer (Postman, Bruno, cURL, YAML, File/Dir)",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Theme::BORDER_ACTIVE));

            let inner = block.inner(area);
            frame.render_widget(block, area);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(6), Constraint::Length(2)])
                .split(inner);

            let input_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Paste cURL, Postman JSON, Bruno .bru, YAML or enter file/folder path on disk ");
            let input_p = Paragraph::new(input.as_str()).block(input_block);
            frame.render_widget(input_p, rows[0]);

            let buttons = Paragraph::new(Line::from(vec![
                Span::styled(
                    " [ Enter: Detect & Import ] ",
                    Style::default()
                        .bg(Color::Cyan)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("   "),
                Span::styled(
                    " [ Esc: Cancel ] ",
                    Style::default().bg(Color::DarkGray).fg(Color::White),
                ),
            ]));
            frame.render_widget(buttons, rows[1]);
        }
        Modal::Export {
            format_idx,
            path_input,
            active_field,
        } => {
            let area = centered_rect(72, 13, frame.area());
            frame.render_widget(Clear, area);

            let block = Block::default()
                .title(Line::from(vec![
                    Span::styled(" 📤 ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        "Export Request / Collection / Environment",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Theme::BORDER_ACTIVE));

            let inner = block.inner(area);
            frame.render_widget(block, area);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(2),
                ])
                .split(inner);

            let field_style = |idx: usize| {
                if *active_field == idx {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                }
            };

            let formats = [
                "1: Postman Collection v2.1 (JSON)",
                "2: Bruno (.bru)",
                "3: Postman Environment (JSON)",
                "4: cURL Command",
                "5: Posting YAML",
            ];

            let format_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(field_style(0))
                .title(" Format (←/→ or 1-5 to select) ");

            let current_fmt = formats.get(*format_idx).unwrap_or(&formats[0]);
            let format_p = Paragraph::new(Line::from(vec![
                Span::raw(" [ "),
                Span::styled(
                    *current_fmt,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" ] "),
            ]))
            .block(format_block);
            frame.render_widget(format_p, rows[0]);

            let path_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(field_style(1))
                .title(" Destination Path (leave empty to copy to clipboard) ");
            let path_p = Paragraph::new(path_input.as_str()).block(path_block);
            frame.render_widget(path_p, rows[1]);

            let buttons = Paragraph::new(Line::from(vec![
                Span::styled(
                    " [ Enter: Export / Copy ] ",
                    Style::default()
                        .bg(Color::Green)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("   "),
                Span::styled(
                    " [ Esc: Cancel ] ",
                    Style::default().bg(Color::DarkGray).fg(Color::White),
                ),
                Span::raw("   "),
                Span::styled(" [Tab] Switch field ", Style::default().fg(Color::Gray)),
            ]));
            frame.render_widget(buttons, rows[2]);
        }
        Modal::SearchResponse { query } => {
            let area = centered_rect(60, 7, frame.area());
            frame.render_widget(Clear, area);

            let block = Block::default()
                .title(Line::from(vec![
                    Span::styled(" 🔍 ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        "Search Response Body",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Color::Yellow));

            let inner = block.inner(area);
            frame.render_widget(block, area);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Length(2)])
                .split(inner);

            let input_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Search Query ");
            let input_p = Paragraph::new(format!("/ {}█", query)).block(input_block);
            frame.render_widget(input_p, rows[0]);

            let buttons = Paragraph::new(Line::from(vec![
                Span::styled(
                    " [ Enter: Jump & Cycle ] ",
                    Style::default()
                        .bg(Color::Green)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("   "),
                Span::styled(
                    " [ Esc: Close ] ",
                    Style::default().bg(Color::DarkGray).fg(Color::White),
                ),
                Span::raw("   "),
                Span::styled(" (n / N: jump matches in viewer) ", Style::default().fg(Color::Cyan)),
            ]));
            frame.render_widget(buttons, rows[1]);
        }
        Modal::CommandPalette {
            query,
            selected_idx,
        } => {
            let area = centered_rect(75, 18, frame.area());
            frame.render_widget(Clear, area);

            let block = Block::default()
                .title(Line::from(vec![
                    Span::styled(" 🔎 ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        "Command Palette",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        " (Fuzzy jump to request)",
                        Style::default().fg(Color::DarkGray),
                    ),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Theme::BORDER_ACTIVE));

            let inner = block.inner(area);
            frame.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(6),
                    Constraint::Length(2),
                ])
                .split(inner);

            let input_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Type to filter requests by name, method, or URL... ");
            let input_p = Paragraph::new(format!("> {}█", query)).block(input_block);
            frame.render_widget(input_p, chunks[0]);

            let q_lower = query.trim().to_lowercase();
            let matches: Vec<&crate::models::CollectionItem> = app
                .collections
                .iter()
                .filter(|item| {
                    if q_lower.is_empty() {
                        return true;
                    }
                    item.name.to_lowercase().contains(&q_lower)
                        || item.request.method.as_str().to_lowercase().contains(&q_lower)
                        || item.request.url.to_lowercase().contains(&q_lower)
                        || item
                            .group
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&q_lower)
                })
                .collect();

            let visible_rows = chunks[1].height as usize;
            let start_idx = if *selected_idx >= visible_rows && visible_rows > 0 {
                *selected_idx - visible_rows + 1
            } else {
                0
            };

            let list_items: Vec<Line> = if matches.is_empty() {
                vec![Line::from(Span::styled(
                    "  No matching requests found",
                    Style::default().fg(Color::DarkGray),
                ))]
            } else {
                matches
                    .iter()
                    .enumerate()
                    .skip(start_idx)
                    .take(visible_rows)
                    .map(|(idx, item)| {
                        let is_sel = idx == *selected_idx;
                        let cursor = if is_sel { "▶ " } else { "  " };
                        let method_style = Theme::method_style(&item.request.method);
                        let group_str = item
                            .group
                            .as_ref()
                            .map(|g| format!("[{}] ", g))
                            .unwrap_or_default();

                        let spans = vec![
                            Span::styled(
                                cursor,
                                if is_sel {
                                    Style::default()
                                        .fg(Color::Yellow)
                                        .add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default()
                                },
                            ),
                            Span::styled(
                                format!("{:>6} ", item.request.method.as_str()),
                                method_style.add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                item.name.clone(),
                                if is_sel {
                                    Style::default()
                                        .fg(Color::White)
                                        .add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default().fg(Color::White)
                                },
                            ),
                            Span::raw(" "),
                            Span::styled(group_str, Style::default().fg(Color::Cyan)),
                            Span::styled(
                                item.request.url.clone(),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ];

                        if is_sel {
                            Line::from(spans).style(Style::default().bg(Color::Rgb(30, 45, 60)))
                        } else {
                            Line::from(spans)
                        }
                    })
                    .collect()
            };

            let list_p = Paragraph::new(list_items);
            frame.render_widget(list_p, chunks[1]);

            let footer = Paragraph::new(Line::from(vec![
                Span::styled(
                    " [ Enter: Open Request ] ",
                    Style::default()
                        .bg(Color::Green)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("   "),
                Span::styled(
                    " [ ↑/↓: Navigate ] ",
                    Style::default().fg(Color::Gray),
                ),
                Span::raw("   "),
                Span::styled(
                    " [ Esc: Cancel ] ",
                    Style::default().bg(Color::DarkGray).fg(Color::White),
                ),
            ]));
            frame.render_widget(footer, chunks[2]);
        }
        Modal::ConfirmDeleteRequest { name, path } => {
            let area = centered_rect(60, 9, frame.area());
            frame.render_widget(Clear, area);

            let block = Block::default()
                .title(Line::from(vec![
                    Span::styled(" ⚠️ ", Style::default().fg(Color::Red)),
                    Span::styled(
                        "Delete Saved Request",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Color::Red));

            let inner = block.inner(area);
            frame.render_widget(block, area);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(4), Constraint::Length(2)])
                .split(inner);

            let info_lines = vec![
                Line::from(Span::styled(
                    "Permanently delete this request file from disk?",
                    Style::default().fg(Color::Yellow),
                )),
                Line::from(vec![
                    Span::styled("Request: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        name.as_str(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("File: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(path.to_string_lossy(), Style::default().fg(Color::DarkGray)),
                ]),
            ];
            frame.render_widget(Paragraph::new(info_lines), rows[0]);

            let buttons = Paragraph::new(Line::from(vec![
                Span::styled(
                    " [ Enter: Delete Permanently ] ",
                    Style::default()
                        .bg(Color::Red)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("   "),
                Span::styled(
                    " [ Esc: Cancel ] ",
                    Style::default().bg(Color::DarkGray).fg(Color::White),
                ),
            ]));
            frame.render_widget(buttons, rows[1]);
        }
        Modal::ConfirmDeleteGroup {
            group_name,
            path,
            req_count,
        } => {
            let area = centered_rect(65, 10, frame.area());
            frame.render_widget(Clear, area);

            let block = Block::default()
                .title(Line::from(vec![
                    Span::styled(" ⚠️ ", Style::default().fg(Color::Red)),
                    Span::styled(
                        "Delete Collection Group",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Color::Red));

            let inner = block.inner(area);
            frame.render_widget(block, area);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(5), Constraint::Length(2)])
                .split(inner);

            let info_lines = vec![
                Line::from(Span::styled(
                    "Permanently delete this group folder and ALL its request files?",
                    Style::default().fg(Color::Yellow),
                )),
                Line::from(vec![
                    Span::styled("Group: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        group_name.as_str(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" ({} requests)", req_count),
                        Style::default().fg(Color::LightRed),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Path: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(path.to_string_lossy(), Style::default().fg(Color::DarkGray)),
                ]),
            ];
            frame.render_widget(Paragraph::new(info_lines), rows[0]);

            let buttons = Paragraph::new(Line::from(vec![
                Span::styled(
                    " [ Enter: Delete Group & Files ] ",
                    Style::default()
                        .bg(Color::Red)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("   "),
                Span::styled(
                    " [ Esc: Cancel ] ",
                    Style::default().bg(Color::DarkGray).fg(Color::White),
                ),
            ]));
            frame.render_widget(buttons, rows[1]);
        }
        Modal::SelectGroupToDelete { selected_idx } => {
            let area = centered_rect(55, 12, frame.area());
            frame.render_widget(Clear, area);

            let block = Block::default()
                .title(Line::from(vec![
                    Span::styled(" 🗑 ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        "Select Group to Delete",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Color::Yellow));

            let inner = block.inner(area);
            frame.render_widget(block, area);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(5), Constraint::Length(2)])
                .split(inner);

            let items: Vec<Line> = if app.groups.is_empty() {
                vec![Line::from(Span::styled(
                    "No collection groups exist",
                    Style::default().fg(Color::DarkGray),
                ))]
            } else {
                app.groups
                    .iter()
                    .enumerate()
                    .map(|(idx, g)| {
                        let is_sel = idx == *selected_idx;
                        let prefix = if is_sel { "▶ " } else { "  " };
                        let count = app
                            .collections
                            .iter()
                            .filter(|c| c.group.as_deref() == Some(g.as_str()))
                            .count();
                        let line = Line::from(vec![
                            Span::styled(
                                prefix,
                                if is_sel {
                                    Style::default()
                                        .fg(Color::Yellow)
                                        .add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default()
                                },
                            ),
                            Span::styled(
                                g.clone(),
                                if is_sel {
                                    Style::default()
                                        .fg(Color::White)
                                        .add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default().fg(Color::White)
                                },
                            ),
                            Span::styled(
                                format!(" ({} requests)", count),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]);
                        if is_sel {
                            line.style(Style::default().bg(Color::Rgb(40, 30, 20)))
                        } else {
                            line
                        }
                    })
                    .collect()
            };
            frame.render_widget(Paragraph::new(items), rows[0]);

            let buttons = Paragraph::new(Line::from(vec![
                Span::styled(
                    " [ Enter: Confirm Selection ] ",
                    Style::default()
                        .bg(Color::Yellow)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("   "),
                Span::styled(
                    " [ Esc: Cancel ] ",
                    Style::default().bg(Color::DarkGray).fg(Color::White),
                ),
            ]));
            frame.render_widget(buttons, rows[1]);
        }
        Modal::ConfirmClearHistory => {
            let area = centered_rect(55, 8, frame.area());
            frame.render_widget(Clear, area);

            let block = Block::default()
                .title(Line::from(vec![
                    Span::styled(" ⚠️ ", Style::default().fg(Color::Red)),
                    Span::styled(
                        "Clear All Call History",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Color::Red));

            let inner = block.inner(area);
            frame.render_widget(block, area);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Length(2)])
                .split(inner);

            let info_lines = vec![
                Line::from(Span::styled(
                    "Are you sure you want to delete ALL call history records?",
                    Style::default().fg(Color::Yellow),
                )),
                Line::from(vec![
                    Span::styled("Total items to remove: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        app.history.len().to_string(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
            ];
            frame.render_widget(Paragraph::new(info_lines), rows[0]);

            let buttons = Paragraph::new(Line::from(vec![
                Span::styled(
                    " [ Enter: Clear History ] ",
                    Style::default()
                        .bg(Color::Red)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("   "),
                Span::styled(
                    " [ Esc: Cancel ] ",
                    Style::default().bg(Color::DarkGray).fg(Color::White),
                ),
            ]));
            frame.render_widget(buttons, rows[1]);
        }
    }
}

pub fn modal_rect(modal: &Modal, area: Rect) -> Option<Rect> {
    match modal {
        Modal::None => None,
        Modal::NewRequest { .. } => Some(centered_rect(64, 18, area)),
        Modal::NewGroup { .. } => Some(centered_rect(50, 10, area)),
        Modal::EditTableRow { .. } => Some(centered_rect(60, 12, area)),
        Modal::ImportUniversal { .. } => Some(centered_rect(75, 14, area)),
        Modal::Export { .. } => Some(centered_rect(72, 13, area)),
        Modal::SearchResponse { .. } => Some(centered_rect(60, 7, area)),
        Modal::CommandPalette { .. } => Some(centered_rect(75, 18, area)),
        Modal::ConfirmDeleteRequest { .. } => Some(centered_rect(60, 9, area)),
        Modal::ConfirmDeleteGroup { .. } => Some(centered_rect(65, 10, area)),
        Modal::SelectGroupToDelete { .. } => Some(centered_rect(55, 12, area)),
        Modal::ConfirmClearHistory => Some(centered_rect(55, 8, area)),
    }
}

pub fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_width = (r.width * percent_x / 100).min(r.width.saturating_sub(4)).max(30);
    let popup_height = height.min(r.height.saturating_sub(2)).max(6);

    let x = (r.width.saturating_sub(popup_width)) / 2;
    let y = (r.height.saturating_sub(popup_height)) / 2;

    Rect::new(r.x + x, r.y + y, popup_width, popup_height)
}
