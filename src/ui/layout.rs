use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppLayout {
    pub sidebar: Rect,
    pub sidebar_new_buttons: Rect,
    pub sidebar_list: Rect,
    pub workspace_tabs: Rect,
    pub url_bar: Rect,
    pub request_tabs: Rect,
    pub request_editor: Rect,
    pub response_status: Rect,
    pub response_tabs: Rect,
    pub response_content: Rect,
    pub footer: Rect,
    pub sidebar_divider: Rect,
    pub horizontal_divider: Rect,
}

impl AppLayout {
    pub fn compute(area: Rect, sidebar_width: u16, request_split_percent: u16) -> Self {
        let main_vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(10), Constraint::Length(1)])
            .split(area);

        let clamped_sidebar_w = sidebar_width.clamp(16, area.width.saturating_sub(25));
        let workspace_horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(clamped_sidebar_w), Constraint::Min(25)])
            .split(main_vertical[0]);

        let right_main = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(8)])
            .split(workspace_horizontal[1]);

        let clamped_split_pct = request_split_percent.clamp(20, 80);
        let right_vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(clamped_split_pct),
                Constraint::Percentage(100 - clamped_split_pct),
            ])
            .split(right_main[1]);

        let request_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(4),
            ])
            .split(right_vertical[0]);

        let response_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(4),
            ])
            .split(right_vertical[1]);

        let sidebar_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(5)])
            .split(workspace_horizontal[0]);

        let sidebar_divider = Rect {
            x: workspace_horizontal[0].x + workspace_horizontal[0].width.saturating_sub(1),
            y: workspace_horizontal[0].y,
            width: 2,
            height: workspace_horizontal[0].height,
        };

        let horizontal_divider = Rect {
            x: right_vertical[0].x,
            y: right_vertical[0].y + right_vertical[0].height.saturating_sub(1),
            width: right_vertical[0].width,
            height: 2,
        };

        Self {
            sidebar: workspace_horizontal[0],
            sidebar_new_buttons: sidebar_chunks[0],
            sidebar_list: sidebar_chunks[1],
            workspace_tabs: right_main[0],
            url_bar: request_chunks[0],
            request_tabs: request_chunks[1],
            request_editor: request_chunks[2],
            response_status: response_chunks[0],
            response_tabs: response_chunks[1],
            response_content: response_chunks[2],
            footer: main_vertical[1],
            sidebar_divider,
            horizontal_divider,
        }
    }

    pub fn contains(rect: Rect, x: u16, y: u16) -> bool {
        x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
    }
}
