use super::theme::Theme;
use crate::app::App;
use crate::events::{Pane, SidebarViewMode};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarEntry {
    GroupHeader {
        name: String,
        is_root: bool,
    },
    RequestItem {
        collection_idx: usize,
    },
    EmptyGroupNotice,
}

pub fn build_sidebar_entries(
    collections: &[crate::models::CollectionItem],
    groups: &[String],
) -> Vec<SidebarEntry> {
    let mut entries = Vec::new();

    let root_requests: Vec<usize> = collections
        .iter()
        .enumerate()
        .filter(|(_, it)| it.group.is_none())
        .map(|(idx, _)| idx)
        .collect();

    if !root_requests.is_empty() {
        entries.push(SidebarEntry::GroupHeader {
            name: "Root / Default".to_string(),
            is_root: true,
        });
        for idx in root_requests {
            entries.push(SidebarEntry::RequestItem { collection_idx: idx });
        }
    }

    let mut rendered_groups = Vec::new();

    for group_name in groups {
        rendered_groups.push(group_name.clone());
        entries.push(SidebarEntry::GroupHeader {
            name: group_name.clone(),
            is_root: false,
        });

        let group_requests: Vec<usize> = collections
            .iter()
            .enumerate()
            .filter(|(_, it)| it.group.as_deref() == Some(group_name.as_str()))
            .map(|(idx, _)| idx)
            .collect();

        if group_requests.is_empty() {
            entries.push(SidebarEntry::EmptyGroupNotice);
        } else {
            for idx in group_requests {
                entries.push(SidebarEntry::RequestItem { collection_idx: idx });
            }
        }
    }

    for (idx, item) in collections.iter().enumerate() {
        if let Some(grp) = &item.group {
            if !rendered_groups.contains(grp) {
                rendered_groups.push(grp.clone());
                entries.push(SidebarEntry::GroupHeader {
                    name: grp.clone(),
                    is_root: false,
                });
                entries.push(SidebarEntry::RequestItem { collection_idx: idx });
            }
        }
    }

    entries
}

pub fn render(frame: &mut Frame, app: &App, buttons_area: Rect, list_area: Rect) {
    let is_focused = app.focused_pane == Pane::Sidebar;
    let border_color = if is_focused {
        Theme::BORDER_ACTIVE
    } else {
        Theme::BORDER_INACTIVE
    };

    let buttons_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Line::from(vec![
            Span::styled(" [1] ", Style::default().fg(Color::DarkGray)),
            Span::styled("Sidebar", Style::default().add_modifier(Modifier::BOLD)),
        ]));

    let col_bg = if app.sidebar_view_mode == SidebarViewMode::Collections {
        Color::Rgb(30, 60, 90)
    } else {
        Color::Rgb(25, 25, 30)
    };

    let hist_bg = if app.sidebar_view_mode == SidebarViewMode::History {
        Color::Rgb(30, 60, 90)
    } else {
        Color::Rgb(25, 25, 30)
    };

    let buttons_line = match app.sidebar_view_mode {
        SidebarViewMode::Collections => Line::from(vec![
            Span::styled(
                " Collections ",
                Style::default().bg(col_bg).fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!(" History ({}) ", app.history.len()),
                Style::default().bg(hist_bg).fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                "[+Req]",
                Style::default().bg(Color::Rgb(30, 60, 30)).fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                "[+Grp]",
                Style::default().bg(Color::Rgb(60, 50, 20)).fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                "[-Grp]",
                Style::default().bg(Color::Rgb(60, 25, 25)).fg(Color::LightRed),
            ),
        ]),
        SidebarViewMode::History => Line::from(vec![
            Span::styled(
                " Collections ",
                Style::default().bg(col_bg).fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!(" History ({}) ", app.history.len()),
                Style::default().bg(hist_bg).fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                "[Del]",
                Style::default().bg(Color::Rgb(60, 25, 25)).fg(Color::LightRed).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                "[Clear]",
                Style::default().bg(Color::Rgb(70, 20, 20)).fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
        ]),
    };

    let buttons_p = Paragraph::new(buttons_line).block(buttons_block);
    frame.render_widget(buttons_p, buttons_area);

    match app.sidebar_view_mode {
        SidebarViewMode::Collections => {
            render_collections_list(frame, app, list_area, border_color, is_focused);
        }
        SidebarViewMode::History => {
            render_history_list(frame, app, list_area, border_color, is_focused);
        }
    }
}

fn render_collections_list(
    frame: &mut Frame,
    app: &App,
    list_area: Rect,
    border_color: Color,
    is_focused: bool,
) {
    let title = Line::from(vec![
        Span::styled(
            "Collections",
            Style::default()
                .fg(Theme::TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" ({} reqs, {} groups) - 'd': del req, 'D': del grp", app.collections.len(), app.groups.len()),
            Style::default().fg(Theme::TEXT_MUTED),
        ),
    ]);

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let entries = build_sidebar_entries(&app.collections, &app.groups);
    let visible_height = list_area.height.saturating_sub(2) as usize;

    let items: Vec<ListItem> = entries
        .iter()
        .skip(app.sidebar_scroll)
        .take(visible_height)
        .map(|entry| match entry {
            SidebarEntry::GroupHeader { name, is_root } => {
                let color = if *is_root { Color::Cyan } else { Color::Yellow };
                let is_selected_group = app.selected_group.as_deref() == Some(name.as_str());
                let style = if is_selected_group {
                    Style::default().fg(color).bg(Color::Rgb(40, 35, 20)).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(color).add_modifier(Modifier::BOLD)
                };
                ListItem::new(Line::from(vec![
                    Span::styled("📁 ", style),
                    Span::styled(name.as_str(), style),
                ]))
            }
            SidebarEntry::RequestItem { collection_idx } => {
                let item = &app.collections[*collection_idx];
                render_request_item(*collection_idx, item, app, is_focused, "  ")
            }
            SidebarEntry::EmptyGroupNotice => ListItem::new(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    "(empty - press 'n' to add)",
                    Style::default().fg(Color::DarkGray),
                ),
            ])),
        })
        .collect();

    let list = List::new(items).block(list_block);
    frame.render_widget(list, list_area);
}

fn render_history_list(
    frame: &mut Frame,
    app: &App,
    list_area: Rect,
    border_color: Color,
    is_focused: bool,
) {
    let title = Line::from(vec![
        Span::styled(
            "Call History",
            Style::default()
                .fg(Theme::TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" ({}) - 'd': delete, 'C': clear all", app.history.len()),
            Style::default().fg(Theme::TEXT_MUTED),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(title);

    if app.history.is_empty() {
        let p = Paragraph::new(Line::from(vec![
            Span::raw("  "),
            Span::styled("No requests executed yet.", Style::default().fg(Color::DarkGray)),
        ]))
        .block(block);
        frame.render_widget(p, list_area);
        return;
    }

    let visible_height = list_area.height.saturating_sub(2) as usize;

    let items: Vec<ListItem> = app
        .history
        .iter()
        .enumerate()
        .skip(app.sidebar_scroll)
        .take(visible_height)
        .map(|(idx, item)| {
            let is_selected = idx == app.selected_history_index;
            let status_style = Theme::status_style(item.status_code);
            let method_style = Theme::method_style(&item.method);

            let prefix = if is_selected { "▶ " } else { "  " };

            let spans = vec![
                Span::styled(
                    prefix,
                    if is_selected {
                        Style::default().fg(Theme::SELECTION_FG)
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(
                    format!("{:<3} ", item.status_code),
                    status_style,
                ),
                Span::styled(
                    format!("{:<5} ", item.method.as_str()),
                    method_style,
                ),
                Span::styled(
                    format!("{} ", item.timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    &item.url,
                    if is_selected {
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Theme::TEXT_MUTED)
                    },
                ),
            ];

            let style = if is_selected && is_focused {
                Style::default().bg(Theme::HIGHLIGHT_BG)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(spans)).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, list_area);
}

fn render_request_item<'a>(
    idx: usize,
    item: &'a crate::models::CollectionItem,
    app: &App,
    is_focused: bool,
    indent: &'static str,
) -> ListItem<'a> {
    let is_selected = idx == app.selected_collection;
    let method_style = Theme::method_style(&item.request.method);
    let prefix = if is_selected { "▶ " } else { "  " };

    let mut spans = vec![
        Span::raw(indent),
        Span::styled(
            prefix,
            if is_selected {
                Style::default().fg(Theme::SELECTION_FG)
            } else {
                Style::default()
            },
        ),
        Span::styled(
            format!("{:<6} ", item.request.method.as_str()),
            method_style,
        ),
        Span::styled(
            &item.name,
            if is_selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_MUTED)
            },
        ),
    ];

    if is_selected {
        spans.push(Span::styled(" ↵", Style::default().fg(Color::DarkGray)));
    }

    let style = if is_selected && is_focused {
        Style::default().bg(Theme::HIGHLIGHT_BG)
    } else {
        Style::default()
    };

    ListItem::new(Line::from(spans)).style(style)
}
