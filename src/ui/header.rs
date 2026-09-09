use crate::app::{App, Tab};
use crate::ui::theme::Theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

pub fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(16),  // App Title
            Constraint::Min(20),     // Current directory / Status
            Constraint::Length(45),  // Tabs
        ])
        .split(area);

    // Title
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border());
    let title_p = Paragraph::new(Line::from(vec![
        Span::styled(" ⚡ ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled("DARK-CLI", Theme::title()),
        Span::raw(" "),
    ]))
    .block(title_block);
    f.render_widget(title_p, chunks[0]);

    // Status or CWD
    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border());

    let (status_text, is_err) = app
        .status_message
        .as_ref()
        .map(|(s, err)| (s.as_str(), *err))
        .unwrap_or(("Готов к работе", false));

    let status_style = if is_err {
        Style::default().fg(Theme::ERROR).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Theme::PRIMARY)
    };

    let status_p = Paragraph::new(Line::from(vec![
        Span::styled(" ● ", if is_err { Style::default().fg(Theme::ERROR) } else { Style::default().fg(Theme::SUCCESS) }),
        Span::styled(status_text, status_style),
    ]))
    .block(status_block);
    f.render_widget(status_p, chunks[1]);

    // Navigation Tabs
    let tabs_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border());

    let make_tab_span = |num: &str, name: &str, tab: Tab| {
        let is_active = app.active_tab == tab;
        if is_active {
            Span::styled(
                format!(" [{}{}] ", num, name),
                Style::default()
                    .fg(Color::Black)
                    .bg(Theme::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                format!(" {}{} ", num, name),
                Style::default().fg(Theme::MUTED),
            )
        }
    };

    let tabs_p = Paragraph::new(Line::from(vec![
        make_tab_span("1:", "Действия", Tab::Actions),
        make_tab_span("2:", "Вывод", Tab::Output),
        make_tab_span("3:", "История", Tab::History),
        make_tab_span("4:", "Справка", Tab::Help),
    ]))
    .block(tabs_block);
    f.render_widget(tabs_p, chunks[2]);
}
