pub mod footer;
pub mod highlighter;
pub mod layout;
pub mod modal;
pub mod request;
pub mod response;
pub mod sidebar;
pub mod tabs;
pub mod theme;

pub use layout::AppLayout;

use crate::app::App;
use ratatui::Frame;

pub fn draw(frame: &mut Frame, app: &App) {
    let layout = AppLayout::compute(frame.area(), app.sidebar_width, app.request_split_percent);

    sidebar::render(frame, app, layout.sidebar_new_buttons, layout.sidebar_list);
    tabs::render(frame, app, layout.workspace_tabs);
    request::render(
        frame,
        app,
        layout.url_bar,
        layout.request_tabs,
        layout.request_editor,
    );
    response::render(
        frame,
        app,
        layout.response_status,
        layout.response_tabs,
        layout.response_content,
    );
    footer::render(frame, app, layout.footer);

    modal::render(frame, app);
}
