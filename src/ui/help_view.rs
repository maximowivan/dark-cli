use crate::app::App;
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};
use ratatui::Frame;

pub fn render_help_view(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border())
        .title(Span::styled(" Quick Help & Keybindings ", Theme::title()));

    let key_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let section_style = Style::default().fg(Theme::PRIMARY).add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Color::White);
    let dim_style = Style::default().fg(Theme::MUTED);

    let text = vec![
        Line::from(vec![Span::styled("Навигация и управление:", section_style)]),
        Line::from(vec![
            Span::styled("  [Enter]         ", key_style),
            Span::styled("Запустить выбранное действие", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  [↑ / ↓] / [j/k] ", key_style),
            Span::styled("Перемещение по списку действий", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  [Tab] / [1..4]  ", key_style),
            Span::styled("Переключение вкладок (Actions, Output, History, Help)", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  [/]             ", key_style),
            Span::styled("Открыть нечеткий поиск (Fuzzy Command Palette в стиле OpenCode)", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  [:]             ", key_style),
            Span::styled("Открыть строку ввода slash-команд (/run, /reload, /clear, /exit)", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  [r]             ", key_style),
            Span::styled("Горячая перезагрузка конфигурации из config.toml", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  [Esc]           ", key_style),
            Span::styled("Закрыть поиск / отменить ввод / вернуться в стандартный режим", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  [q]             ", key_style),
            Span::styled("Выход из программы (в обычном режиме)", desc_style),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Slash-команды (в строке ввода):", section_style)]),
        Line::from(vec![
            Span::styled("  /run <id>       ", key_style),
            Span::styled("Запустить действие по его идентификатору (ID)", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  /reload         ", key_style),
            Span::styled("Перечитать config.toml без перезапуска", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  /clear          ", key_style),
            Span::styled("Очистить окно вывода", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  /help           ", key_style),
            Span::styled("Показать эту справку", desc_style),
        ]),
        Line::from(vec![
            Span::styled("  /exit           ", key_style),
            Span::styled("Завершить работу приложения", desc_style),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Конфигурационный файл:", section_style)]),
        Line::from(vec![
            Span::styled("  Путь к конфигу: ", dim_style),
            Span::styled(format!("{}", app.config_path.display()), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("  Вы можете добавлять новые программы, скрипты и ссылки в config.toml.", dim_style),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}
