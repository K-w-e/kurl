use crate::app::App;
use crate::events::{Action, Pane, SidebarViewMode};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FooterClickTarget {
    pub start_x: u16,
    pub end_x: u16,
    pub action: Action,
}

pub fn build_footer(app: &App, max_width: u16) -> (Line<'static>, Vec<FooterClickTarget>) {
    let mut spans = Vec::new();
    let mut targets = Vec::new();
    let mut current_x = 0u16;

    let focus_label = match app.focused_pane {
        Pane::Sidebar => "Sidebar",
        Pane::WorkspaceTabs => "Tabs",
        Pane::UrlBar => "URL Bar",
        Pane::MethodSelector => "Method",
        Pane::RequestTabs => "Req Tabs",
        Pane::RequestEditor => "Editor",
        Pane::ResponseTabs => "Resp Tabs",
        Pane::ResponseViewer => "Response",
    };

    let focus_str = format!(" FOCUS: {} ", focus_label);
    let focus_w = focus_str.len() as u16;
    targets.push(FooterClickTarget {
        start_x: current_x,
        end_x: current_x + focus_w,
        action: Action::FocusNext,
    });
    spans.push(Span::styled(
        focus_str,
        Style::default()
            .bg(Color::Cyan)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    ));
    current_x += focus_w;

    spans.push(Span::raw(" "));
    current_x += 1;

    let active_env_name = app
        .active_environment()
        .map(|e| e.name.as_str())
        .unwrap_or("none");
    let env_str = format!(" [Env: {}] ", active_env_name);
    let env_w = env_str.len() as u16;
    targets.push(FooterClickTarget {
        start_x: current_x,
        end_x: current_x + env_w,
        action: Action::NextEnvironment,
    });
    spans.push(Span::styled(
        env_str,
        Style::default()
            .bg(Color::Rgb(40, 50, 70))
            .fg(Color::LightCyan)
            .add_modifier(Modifier::BOLD),
    ));
    current_x += env_w;

    spans.push(Span::raw(" "));
    current_x += 1;

    if let Some(msg) = &app.status_flash_message {
        let msg_str = format!(" {} ", msg);
        let msg_w = msg_str.len() as u16;
        spans.push(Span::styled(
            msg_str,
            Style::default()
                .bg(Color::Green)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ));
        current_x += msg_w;
        spans.push(Span::raw(" "));
        current_x += 1;
    }

    let quit_button = (
        " [Quit] ",
        Style::default()
            .bg(Color::Rgb(70, 20, 20))
            .fg(Color::LightRed)
            .add_modifier(Modifier::BOLD),
        Action::Quit,
    );
    let quit_w = quit_button.0.len() as u16;

    let buttons = match app.sidebar_view_mode {
        SidebarViewMode::Collections => vec![
            (
                " [Send] ",
                Style::default()
                    .bg(Color::Green)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
                Action::SendRequest,
            ),
            (
                " [+Req] ",
                Style::default()
                    .bg(Color::Rgb(30, 60, 40))
                    .fg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
                Action::OpenNewRequestModal,
            ),
            (
                " [-Req] ",
                Style::default()
                    .bg(Color::Rgb(60, 30, 30))
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::BOLD),
                Action::OpenDeleteRequestModal,
            ),
            (
                " [+Grp] ",
                Style::default()
                    .bg(Color::Rgb(60, 50, 30))
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                Action::OpenNewGroupModal,
            ),
            (
                " [-Grp] ",
                Style::default()
                    .bg(Color::Rgb(60, 30, 30))
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
                Action::OpenDeleteGroupModal,
            ),
            (
                " [Save] ",
                Style::default()
                    .bg(Color::Rgb(30, 50, 70))
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
                Action::SaveRequest,
            ),
            (
                " [Import] ",
                Style::default()
                    .bg(Color::Rgb(50, 30, 60))
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
                Action::OpenImportModal,
            ),
            (
                " [Export] ",
                Style::default()
                    .bg(Color::Rgb(30, 50, 60))
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
                Action::OpenExportModal,
            ),
            (
                " [cURL] ",
                Style::default()
                    .bg(Color::Rgb(50, 50, 30))
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                Action::CopyAsCurl,
            ),
            (
                " [Body] ",
                Style::default()
                    .bg(Color::Rgb(40, 40, 50))
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
                Action::CopyResponseBody,
            ),
            (
                " [Find] ",
                Style::default()
                    .bg(Color::Rgb(50, 30, 60))
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
                Action::OpenCommandPalette,
            ),
            (
                " [Search] ",
                Style::default()
                .bg(Color::Rgb(50, 50, 30))
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
                Action::OpenSearchModal,
            ),
            (
                " [Hist] ",
                Style::default()
                    .bg(Color::Rgb(30, 40, 60))
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
                Action::ToggleSidebarView,
            ),
        ],
        SidebarViewMode::History => vec![
            (
                " [Send] ",
                Style::default()
                    .bg(Color::Green)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
                Action::SendRequest,
            ),
            (
                " [-Item] ",
                Style::default()
                    .bg(Color::Rgb(60, 30, 30))
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::BOLD),
                Action::DeleteSelectedHistory,
            ),
            (
                " [Clear] ",
                Style::default()
                    .bg(Color::Rgb(70, 20, 20))
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
                Action::OpenClearHistoryModal,
            ),
            (
                " [Save] ",
                Style::default()
                    .bg(Color::Rgb(30, 50, 70))
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
                Action::SaveRequest,
            ),
            (
                " [Import] ",
                Style::default()
                    .bg(Color::Rgb(50, 30, 60))
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
                Action::OpenImportModal,
            ),
            (
                " [Export] ",
                Style::default()
                    .bg(Color::Rgb(30, 50, 60))
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
                Action::OpenExportModal,
            ),
            (
                " [cURL] ",
                Style::default()
                    .bg(Color::Rgb(50, 50, 30))
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                Action::CopyAsCurl,
            ),
            (
                " [Body] ",
                Style::default()
                    .bg(Color::Rgb(40, 40, 50))
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
                Action::CopyResponseBody,
            ),
            (
                " [Find] ",
                Style::default()
                    .bg(Color::Rgb(50, 30, 60))
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
                Action::OpenCommandPalette,
            ),
            (
                " [Search] ",
                Style::default()
                    .bg(Color::Rgb(50, 50, 30))
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                Action::OpenSearchModal,
            ),
            (
                " [Colls] ",
                Style::default()
                    .bg(Color::Rgb(30, 40, 60))
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
                Action::ToggleSidebarView,
            ),
        ],
    };

    for (label, style, action) in buttons {
        let btn_w = label.len() as u16;
        if current_x + btn_w + 1 + quit_w <= max_width {
            targets.push(FooterClickTarget {
                start_x: current_x,
                end_x: current_x + btn_w,
                action,
            });
            spans.push(Span::styled(label, style));
            current_x += btn_w;
            spans.push(Span::raw(" "));
            current_x += 1;
        }
    }

    if current_x + quit_w <= max_width {
        targets.push(FooterClickTarget {
            start_x: current_x,
            end_x: current_x + quit_w,
            action: quit_button.2,
        });
        spans.push(Span::styled(quit_button.0, quit_button.1));
    }

    (Line::from(spans), targets)
}

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let (line, _) = build_footer(app, area.width);
    let footer = Paragraph::new(line);
    frame.render_widget(footer, area);
}
