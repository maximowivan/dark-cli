use crate::app::App;
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

pub fn render_history_view(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border())
        .title(Span::styled(
            format!(" Execution History ({}) ", app.history.len()),
            Theme::title(),
        ));

    if app.history.is_empty() {
        let empty_p = Paragraph::new("\n  История запусков пуста.")
            .block(block)
            .style(Style::default().fg(Theme::MUTED));
        f.render_widget(empty_p, area);
        return;
    }

    let items: Vec<ListItem> = app
        .history
        .iter()
        .rev()
        .skip(app.history_scroll as usize)
        .map(|res| {
            let status_icon = if res.success { "✔" } else { "✖" };
            let status_style = if res.success {
                Style::default().fg(Theme::SUCCESS)
            } else {
                Style::default().fg(Theme::ERROR)
            };

            let time_str = res.timestamp.format("%H:%M:%S").to_string();

            let line = Line::from(vec![
                Span::styled(format!(" [{}] ", time_str), Style::default().fg(Theme::MUTED)),
                Span::styled(format!("{} ", status_icon), status_style.add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:<18} ", res.name), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled(format!("({:<12}) ", res.id), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("({:.2?}) ", res.duration), Style::default().fg(Color::Yellow)),
                Span::styled(format!("$ {}", res.command), Style::default().fg(Color::Gray)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}
