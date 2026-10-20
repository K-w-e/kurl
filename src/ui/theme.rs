use crate::models::HttpMethod;
use ratatui::style::{Color, Modifier, Style};

pub struct Theme;

impl Theme {
    #[allow(dead_code)]
    pub const BG_DARK: Color = Color::Reset;
    pub const BORDER_INACTIVE: Color = Color::DarkGray;
    pub const BORDER_ACTIVE: Color = Color::Cyan;
    pub const TEXT_PRIMARY: Color = Color::White;
    pub const TEXT_MUTED: Color = Color::Gray;
    pub const HIGHLIGHT_BG: Color = Color::Rgb(40, 44, 52);
    pub const SELECTION_FG: Color = Color::Yellow;

    pub fn method_style(method: &HttpMethod) -> Style {
        match method {
            HttpMethod::Get => Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            HttpMethod::Post => Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            HttpMethod::Put => Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            HttpMethod::Delete => Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
            HttpMethod::Patch => Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
            HttpMethod::Head => Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            HttpMethod::Options => Style::default()
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        }
    }

    pub fn status_style(status_code: u16) -> Style {
        match status_code {
            200..=299 => Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            300..=399 => Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            400..=499 => Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            500..=599 => Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
            _ => Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        }
    }
}
