use crate::app::{App, InputMode};
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

pub fn render_input_bar(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(match app.input_mode {
            InputMode::Normal => Theme::border(),
            InputMode::CommandInput => Theme::border_active(),
            InputMode::PaletteSearch => Style::default().fg(Color::Yellow),
        });

    let make_hint = |key: &str, desc: &str| -> Vec<Span> {
        vec![
            Span::styled(format!(" [{}] ", key), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{}  ", desc), Style::default().fg(Theme::MUTED)),
        ]
    };

    let p = match app.input_mode {
        InputMode::Normal => {
            let mut spans = vec![Span::raw(" ")];
            spans.extend(make_hint("Enter/→", "Открыть/Запуск"));
            spans.extend(make_hint("Esc/←", "Назад"));
            spans.extend(make_hint("↑↓", "Выбор"));
            spans.extend(make_hint("/", "Поиск"));
            spans.extend(make_hint("Tab", "Вкладки"));
            spans.extend(make_hint("r", "Конфиг"));
            spans.extend(make_hint("q", "Выход"));
            Paragraph::new(Line::from(spans)).block(block)
        }
        InputMode::CommandInput => {
            let spans = vec![
                Span::styled(" : ", Style::default().fg(Theme::PRIMARY).add_modifier(Modifier::BOLD)),
                Span::styled(&app.input_buffer, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled("█", Style::default().fg(Theme::PRIMARY)), // Block cursor
            ];
            Paragraph::new(Line::from(spans)).block(block)
        }
        InputMode::PaletteSearch => {
            let mut spans = vec![
                Span::styled(" 🔍 Поиск: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(&app.input_buffer, Style::default().fg(Color::White)),
                Span::styled("█ ", Style::default().fg(Color::Yellow)),
            ];
            spans.extend(make_hint("Esc", "Отмена"));
            spans.extend(make_hint("Enter", "Запустить"));
            Paragraph::new(Line::from(spans)).block(block)
        }
    };

    f.render_widget(p, area);
}
