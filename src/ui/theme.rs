use ratatui::style::{Color, Modifier, Style};

#[allow(dead_code)]
pub struct Theme;

#[allow(dead_code)]
impl Theme {
    pub const PRIMARY: Color = Color::Cyan;
    pub const SECONDARY: Color = Color::Magenta;
    pub const SUCCESS: Color = Color::Green;
    pub const WARNING: Color = Color::Yellow;
    pub const ERROR: Color = Color::Red;
    pub const TEXT: Color = Color::White;
    pub const MUTED: Color = Color::DarkGray;
    pub const HIGHLIGHT_BG: Color = Color::Rgb(30, 40, 60);

    pub fn title() -> Style {
        Style::default()
            .fg(Self::PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    pub fn header_badge() -> Style {
        Style::default()
            .fg(Color::Black)
            .bg(Self::PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    pub fn border() -> Style {
        Style::default().fg(Color::Rgb(60, 70, 90))
    }

    pub fn border_active() -> Style {
        Style::default()
            .fg(Self::PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected_item() -> Style {
        Style::default()
            .fg(Color::White)
            .bg(Color::Rgb(25, 45, 75))
            .add_modifier(Modifier::BOLD)
    }

    pub fn category_badge(category: &str) -> Style {
        match category.to_lowercase().as_str() {
            "development" | "dev" | "разработка" => Style::default().fg(Color::LightCyan),
            "git" => Style::default().fg(Color::LightYellow),
            "system" | "sys" | "система" => Style::default().fg(Color::LightMagenta),
            "web" | "веб" => Style::default().fg(Color::LightBlue),
            _ => Style::default().fg(Color::LightGreen),
        }
    }
}
