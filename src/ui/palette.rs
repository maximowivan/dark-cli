use crate::app::App;
use crate::ui::theme::Theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

pub fn render_palette(f: &mut Frame, app: &App, area: Rect) {
    let modal_area = centered_rect(65, 60, area);

    // Clear the background so underlying widgets don't bleed through
    f.render_widget(Clear, modal_area);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::PRIMARY).add_modifier(Modifier::BOLD))
        .title(Span::styled(" ⚡ Палитра команд (Быстрый поиск) ", Theme::title()));

    let inner_area = outer_block.inner(modal_area);
    f.render_widget(outer_block, modal_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input box
            Constraint::Min(3),    // Results list
        ])
        .split(inner_area);

    // Search input box
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));

    let input_p = Paragraph::new(Line::from(vec![
        Span::styled(" > ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(&app.input_buffer, Style::default().fg(Color::White)),
        Span::styled("█", Style::default().fg(Color::Yellow)),
    ]))
    .block(input_block);
    f.render_widget(input_p, chunks[0]);

    // Filtered results
    let filtered = app.filtered_actions();
    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .take(15) // Show top 15 results
        .map(|(i, (_, action))| {
            let is_selected = i == app.palette_selected_idx;
            let prefix = if is_selected { " ▶ " } else { "   " };

            let line = Line::from(vec![
                Span::styled(prefix, if is_selected { Style::default().fg(Theme::PRIMARY) } else { Style::default() }),
                Span::styled(format!("{:<10} ", format!("[{}]", action.category)), Theme::category_badge(&action.category)),
                Span::styled(
                    format!("{:<20} ", action.name),
                    if is_selected { Theme::selected_item() } else { Style::default().fg(Color::White) },
                ),
                Span::styled(format!("$ {}", action.command), Style::default().fg(Theme::MUTED)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let results_title = format!(" Найдено совпадений ({}) ", filtered.len());
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border())
        .title(Span::styled(results_title, Style::default().fg(Theme::MUTED)));

    let list = List::new(items).block(list_block);
    f.render_widget(list, chunks[1]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
