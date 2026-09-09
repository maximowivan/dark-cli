use crate::app::App;
use crate::ui::theme::Theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

pub fn render_actions_view(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    let filtered = app.filtered_actions();
    let categories = app.categories();

    // Split left pane into: Category Bar (Top) + Actions List (Bottom)
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .split(chunks[0]);

    // 1. Category Bar
    let mut cat_spans = vec![Span::raw(" ")];
    for (i, cat) in categories.iter().enumerate() {
        let count = if i == 0 {
            app.config.actions.len()
        } else {
            app.config.actions.iter().filter(|a| a.category.eq_ignore_ascii_case(cat)).count()
        };
        let is_selected = i == app.selected_category_idx;
        let icon = match cat.to_lowercase().as_str() {
            "zapret" => "⚡ ",
            "разработка" | "dev" => "💻 ",
            "система" | "system" => "🛠 ",
            "git" => "🌿 ",
            "веб" | "web" => "🌐 ",
            _ => "📁 ",
        };
        let label = format!(" {}{}({}) ", if i == 0 { "✦ " } else { icon }, cat, count);
        if is_selected {
            cat_spans.push(Span::styled(
                label,
                Style::default().fg(Color::Black).bg(Theme::PRIMARY).add_modifier(Modifier::BOLD),
            ));
        } else {
            cat_spans.push(Span::styled(
                label,
                Style::default().fg(Theme::MUTED),
            ));
        }
        cat_spans.push(Span::raw(" "));
    }

    let cat_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border())
        .title(Span::styled(" Группы / Категории [← / →] ", Theme::title()));
    let cat_p = Paragraph::new(Line::from(cat_spans)).block(cat_block);
    f.render_widget(cat_p, left_chunks[0]);

    // 2. Actions List
    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, (_, action))| {
            let is_selected = i == app.selected_action_idx;

            let prefix = if is_selected { " ▶ " } else { "   " };
            let cat_style = Theme::category_badge(&action.category);

            let shortcut_span = if let Some(ref sc) = action.shortcut {
                Span::styled(format!("[{}] ", sc), Style::default().fg(Color::Yellow))
            } else {
                Span::raw("")
            };

            let line = Line::from(vec![
                Span::styled(prefix, if is_selected { Style::default().fg(Theme::PRIMARY) } else { Style::default() }),
                Span::styled(format!("{:<11} ", format!("[{}]", action.category)), cat_style),
                Span::styled(format!("{:<18} ", action.name), if is_selected { Theme::selected_item() } else { Style::default().fg(Color::White) }),
                shortcut_span,
                Span::styled(format!("({})", action.id), Style::default().fg(Theme::MUTED)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let active_cat = categories.get(app.selected_category_idx).map(|s| s.as_str()).unwrap_or("Все");
    let title_text = format!(" Действия: {} ({}) ", active_cat, filtered.len());
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border_active())
        .title(Span::styled(title_text, Theme::title()));

    let list_widget = List::new(items).block(list_block);
    f.render_widget(list_widget, left_chunks[1]);

    // Detail Panel (Right)
    let detail_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border())
        .title(Span::styled(" Детали действия ", Theme::title()));

    if let Some((_, selected_action)) = filtered.get(app.selected_action_idx) {
        let sc_text = selected_action.shortcut.as_deref().unwrap_or("Нет");
        let cwd_text = selected_action.cwd.as_deref().unwrap_or("Текущая папка");

        let lines = vec![
            Line::from(vec![
                Span::styled("Название:    ", Style::default().fg(Theme::MUTED)),
                Span::styled(&selected_action.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Код (ID):    ", Style::default().fg(Theme::MUTED)),
                Span::styled(&selected_action.id, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("Категория:   ", Style::default().fg(Theme::MUTED)),
                Span::styled(&selected_action.category, Theme::category_badge(&selected_action.category)),
            ]),
            Line::from(vec![
                Span::styled("Клавиша:     ", Style::default().fg(Theme::MUTED)),
                Span::styled(sc_text, Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::styled("Рабочая папка: ", Style::default().fg(Theme::MUTED)),
                Span::styled(cwd_text, Style::default().fg(Color::Gray)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Команда:", Style::default().fg(Theme::PRIMARY).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(format!("  $ {}", selected_action.command), Style::default().fg(Color::Green)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Описание:", Style::default().fg(Theme::PRIMARY).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(format!("  {}", selected_action.description), Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("─── Быстрый запуск ───", Style::default().fg(Theme::MUTED)),
            ]),
            Line::from(vec![
                Span::styled("Нажмите ", Style::default().fg(Theme::MUTED)),
                Span::styled("[Enter]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(" для запуска этого действия", Style::default().fg(Theme::MUTED)),
            ]),
        ];

        let detail_p = Paragraph::new(lines)
            .block(detail_block)
            .wrap(Wrap { trim: false });
        f.render_widget(detail_p, chunks[1]);
    } else {
        let empty_p = Paragraph::new("Нет доступных действий по текущему фильтру.")
            .block(detail_block)
            .style(Style::default().fg(Theme::MUTED));
        f.render_widget(empty_p, chunks[1]);
    }
}
