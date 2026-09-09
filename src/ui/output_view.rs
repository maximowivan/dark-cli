use crate::app::App;
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};
use ratatui::Frame;

pub fn render_output_view(f: &mut Frame, app: &App, area: Rect) {
    if let Some(ref res) = app.last_result {
        let code_str = match res.exit_code {
            Some(0) => "SUCCESS (0)".to_string(),
            Some(code) => format!("FAILED ({})", code),
            None => "ERROR".to_string(),
        };

        let status_style = if res.success {
            Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Theme::ERROR).add_modifier(Modifier::BOLD)
        };

        let title_spans = Line::from(vec![
            Span::styled(" Output: ", Theme::title()),
            Span::styled(&res.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::raw(" | "),
            Span::styled(format!("${} ", res.command), Style::default().fg(Color::Cyan)),
            Span::raw("| Status: "),
            Span::styled(code_str, status_style),
            Span::raw(" | Time: "),
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
            .title(Span::styled(" Output Console ", Theme::title()));

        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Ещё ни одна команда не была запущена в текущей сессии.",
                Style::default().fg(Theme::MUTED),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Перейдите во вкладку ", Style::default().fg(Theme::MUTED)),
                Span::styled("[1] Actions", Style::default().fg(Theme::PRIMARY).add_modifier(Modifier::BOLD)),
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
