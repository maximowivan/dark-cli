use crate::app::{App, InstallDialogState};
use crate::ui::theme::Theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn render_install_modal(f: &mut Frame, app: &App, area: Rect) {
    let dialog_state = match app.install_dialog {
        Some(ref s) => s,
        None => return,
    };

    let modal_area = centered_rect(70, 48, area);

    // Clear the background
    f.render_widget(Clear, modal_area);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::PRIMARY).add_modifier(Modifier::BOLD))
        .title(Span::styled(" ⚡ Установка Flowseal Zapret ", Theme::title()));

    let inner_area = outer_block.inner(modal_area);
    f.render_widget(outer_block, modal_area);

    match dialog_state {
        InstallDialogState::ChooseOption { selected } => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2), // Prompt text
                    Constraint::Length(5), // Options
                    Constraint::Length(1), // Spacer
                    Constraint::Length(2), // Help text
                ])
                .split(inner_area);

            let prompt_lines = vec![
                Line::from(vec![
                    Span::styled("Выберите, куда вы хотите установить программу Zapret:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled("Будет скачан и распакован последний релиз с GitHub.", Style::default().fg(Theme::MUTED)),
                ]),
            ];
            f.render_widget(Paragraph::new(prompt_lines), chunks[0]);

            let opt1_selected = *selected == 0;
            let opt2_selected = *selected == 1;

            let opt1_line = Line::from(vec![
                Span::styled(if opt1_selected { " ▶ [1] " } else { "   [1] " }, if opt1_selected { Style::default().fg(Theme::PRIMARY) } else { Style::default().fg(Theme::MUTED) }),
                Span::styled("В папку по умолчанию (C:\\zapret)", if opt1_selected { Theme::selected_item() } else { Style::default().fg(Color::White) }),
                Span::styled("  (Рекомендуется)", Style::default().fg(Theme::MUTED)),
            ]);

            let opt2_line = Line::from(vec![
                Span::styled(if opt2_selected { " ▶ [2] " } else { "   [2] " }, if opt2_selected { Style::default().fg(Theme::PRIMARY) } else { Style::default().fg(Theme::MUTED) }),
                Span::styled("Выбрать другую папку (указать свой путь)...", if opt2_selected { Theme::selected_item() } else { Style::default().fg(Color::White) }),
            ]);

            f.render_widget(Paragraph::new(vec![opt1_line, Line::from(""), opt2_line]), chunks[1]);

            let help_line = Line::from(vec![
                Span::styled("  [↑↓ / 1 / 2]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(" Выбор   ", Style::default().fg(Theme::MUTED)),
                Span::styled("[Enter]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(" Подтвердить   ", Style::default().fg(Theme::MUTED)),
                Span::styled("[Esc]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(" Отмена", Style::default().fg(Theme::MUTED)),
            ]);
            f.render_widget(Paragraph::new(help_line), chunks[3]);
        }
        InstallDialogState::EnterPath { input } => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2), // Prompt text
                    Constraint::Length(3), // Input box
                    Constraint::Length(1), // Spacer
                    Constraint::Length(2), // Help text
                ])
                .split(inner_area);

            let prompt = Paragraph::new("Введите полный путь к папке для установки Zapret (например, D:\\tools\\zapret):")
                .style(Style::default().fg(Color::White));
            f.render_widget(prompt, chunks[0]);

            let input_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Yellow));

            let input_p = Paragraph::new(Line::from(vec![
                Span::styled(" > ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(input, Style::default().fg(Color::White)),
                Span::styled("█", Style::default().fg(Color::Yellow)),
            ]))
            .block(input_block);
            f.render_widget(input_p, chunks[1]);

            let help_line = Line::from(vec![
                Span::styled("  [Enter]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(" Установить в эту папку   ", Style::default().fg(Theme::MUTED)),
                Span::styled("[Esc]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(" Назад к выбору", Style::default().fg(Theme::MUTED)),
            ]);
            f.render_widget(Paragraph::new(help_line), chunks[3]);
        }
    }
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
