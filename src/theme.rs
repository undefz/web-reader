use ratatui::style::{Color, Modifier, Style};

pub const BG: Color = Color::Rgb(12, 12, 20);
pub const FG: Color = Color::Rgb(204, 204, 220);
pub const ACCENT: Color = Color::Rgb(255, 183, 77);
pub const SELECTED_BG: Color = Color::Rgb(40, 40, 70);
pub const DIM: Color = Color::Rgb(100, 100, 140);
pub const BORDER: Color = Color::Rgb(60, 60, 100);
pub const STATUS_BG: Color = Color::Rgb(20, 20, 35);

pub fn base() -> Style {
    Style::default().fg(FG).bg(BG)
}

pub fn selected() -> Style {
    Style::default().fg(FG).bg(SELECTED_BG)
}

pub fn channel_name() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub fn channel_name_selected() -> Style {
    Style::default()
        .fg(ACCENT)
        .bg(SELECTED_BG)
        .add_modifier(Modifier::BOLD)
}

pub fn dim() -> Style {
    Style::default().fg(DIM).bg(BG)
}

pub fn status_bar() -> Style {
    Style::default().fg(DIM).bg(STATUS_BG)
}
