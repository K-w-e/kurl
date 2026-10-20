use super::theme::Theme;
use crate::app::App;
use crate::events::Pane;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

pub fn compute_tab_widths(app: &App) -> (Vec<(u16, u16)>, u16, u16) {
    let mut positions = Vec::new();
    let mut current_x = 0u16;

    for session in &app.open_tabs {
        let tab_width = (session.request.method.as_str().len()
            + session.title.len()
            + 6) as u16;
        let start_x = current_x;
        let end_x = start_x + tab_width;
        positions.push((start_x, end_x));
        current_x = end_x + 1;
    }

    let plus_start = current_x;
    let plus_end = plus_start + 5;
    (positions, plus_start, plus_end)
}

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focused_pane == Pane::WorkspaceTabs;
    let border_color = if is_focused {
        Theme::BORDER_ACTIVE
    } else {
        Theme::BORDER_INACTIVE
    };

    let inner_width = area.width.saturating_sub(2);
    let (_, _, plus_end) = compute_tab_widths(app);

    let mut title_spans = vec![
        Span::styled(" Open Requests ", Style::default().add_modifier(Modifier::BOLD)),
    ];

    if app.workspace_tabs_scroll > 0 {
        title_spans.push(Span::styled(
            "◀ ",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));
    }

    if plus_end > app.workspace_tabs_scroll + inner_width {
        title_spans.push(Span::styled(
            "▶ ",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));
    }

    title_spans.push(Span::styled(
        format!(
            "({}/{} tabs) - Ctrl+T: new, Ctrl+W: close ",
            app.active_tab_index + 1,
            app.open_tabs.len()
        ),
        Style::default().fg(Color::DarkGray),
    ));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Line::from(title_spans));

    let mut spans = Vec::new();

    for (idx, session) in app.open_tabs.iter().enumerate() {
        let is_active = idx == app.active_tab_index;
        let method_style = Theme::method_style(&session.request.method);

        let bg_color = if is_active {
            Color::Rgb(40, 48, 64)
        } else {
            Color::Rgb(24, 26, 32)
        };

        let title_style = if is_active {
            Style::default()
                .fg(Color::White)
                .bg(bg_color)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::TEXT_MUTED).bg(bg_color)
        };

        spans.push(Span::styled(" ", Style::default().bg(bg_color)));
        spans.push(Span::styled(session.request.method.as_str(), method_style.bg(bg_color)));
        spans.push(Span::styled(" ", Style::default().bg(bg_color)));
        spans.push(Span::styled(&session.title, title_style));
        spans.push(Span::styled(" ✕ ", Style::default().fg(Color::LightRed).bg(bg_color)));
        spans.push(Span::raw(" "));
    }

    spans.push(Span::styled(
        " [+] ",
        Style::default()
            .bg(Color::Rgb(30, 60, 30))
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    ));

    let p = Paragraph::new(Line::from(spans))
        .block(block)
        .scroll((0, app.workspace_tabs_scroll));
    frame.render_widget(p, area);
}
