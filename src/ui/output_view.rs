use crate::app::App;
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};
use ratatui::Frame;

pub fn render_output_view(f: &mut Frame, app: &App, area: Rect) {
    if let Some(ref action) = app.executing_action {
        let title_spans = Line::from(vec![
            Span::styled(" Вывод: ", Theme::title()),
            Span::styled(&action.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::raw(" | "),
            Span::styled(format!("${} ", action.command), Style::default().fg(Color::Cyan)),
            Span::raw("| Статус: "),
            Span::styled("⏳ ВЫПОЛНЯЕТСЯ...", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Yellow))
            .title(title_spans);

        let lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  ⏳ Команда запущена и выполняется: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(&action.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  💻 Команда:   ", Style::default().fg(Theme::MUTED)),
                Span::styled(format!("${}", action.command), Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("  📁 Категория: ", Style::default().fg(Theme::MUTED)),
                Span::styled(&action.category, Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  Пожалуйста, подождите завершения процесса...",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(Span::styled(
                "  Полный лог выполнения появится на этом экране сразу после окончания работы команды.",
                Style::default().fg(Theme::MUTED),
            )),
        ];

        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
        return;
    }

    if let Some(ref res) = app.last_result {
        let code_str = match res.exit_code {
            Some(0) => "УСПЕШНО (0)".to_string(),
            Some(code) => format!("ОШИБКА ({})", code),
            None => "СБОЙ".to_string(),
        };

        let status_style = if res.success {
            Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::ERROR).add_modifier(Modifier::BOLD)
        };

        let title_spans = Line::from(vec![
            Span::styled(" Вывод: ", Theme::title()),
            Span::styled(&res.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::raw(" | "),
            Span::styled(format!("${} ", res.command), Style::default().fg(Color::Cyan)),
            Span::raw("| Статус: "),
            Span::styled(code_str, status_style),
            Span::raw(" | Время: "),
            Span::styled(format!("{:.2?}", res.duration), Style::default().fg(Color::Yellow)),
        ]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if res.success { Theme::border_active() } else { Style::default().fg(Theme::ERROR) })
            .title(title_spans);

        let lines: Vec<Line> = res
            .output
            .lines()
            .map(|l| Line::from(Span::styled(l, Style::default().fg(Color::White))))
            .collect();

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((app.output_scroll, 0));

        f.render_widget(paragraph, area);
    } else {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Theme::border())
            .title(Span::styled(" Консоль вывода ", Theme::title()));

        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Ещё ни одна команда не была запущена в текущей сессии.",
                Style::default().fg(Theme::MUTED),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Перейдите во вкладку ", Style::default().fg(Theme::MUTED)),
                Span::styled("[1] Действия", Style::default().fg(Theme::PRIMARY).add_modifier(Modifier::BOLD)),
                Span::styled(" и нажмите ", Style::default().fg(Theme::MUTED)),
                Span::styled("[Enter]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(" для запуска любого действия.", Style::default().fg(Theme::MUTED)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Или нажмите ", Style::default().fg(Theme::MUTED)),
                Span::styled("[/]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(" для быстрого поиска и запуска команды на лету.", Style::default().fg(Theme::MUTED)),
            ]),
        ];

        let paragraph = Paragraph::new(text).block(block);
        f.render_widget(paragraph, area);
    }
}
