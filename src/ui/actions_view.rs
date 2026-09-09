use crate::app::{App, ViewItem};
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

    let items = app.current_items();

    // 1. Actions / Folders List (Left Pane)
    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == app.selected_action_idx;
            let prefix = if is_selected { " ▶ " } else { "   " };

            match item {
                ViewItem::Folder { name, count } => {
                    let icon = match name.to_lowercase().as_str() {
                        "zapret" => "📁 ⚡ ",
                        "telegram proxy" | "tg proxy" | "telegram" | "tg-proxy" => "📁 ✈️ ",
                        "автозагрузка и трей" | "автозагрузка" | "трей" | "autorun" => "📁 🖥️ ",
                        "разработка" | "dev" => "📁 💻 ",
                        "система" | "system" => "📁 🛠 ",
                        "git" => "📁 🌿 ",
                        "веб" | "web" => "📁 🌐 ",
                        _ => "📁 ",
                    };

                    let folder_name_style = if is_selected {
                        Theme::selected_item()
                    } else {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    };

                    let mut line_spans = vec![
                        Span::styled(prefix, if is_selected { Style::default().fg(Theme::PRIMARY) } else { Style::default() }),
                        Span::styled(icon, Style::default().fg(Color::Yellow)),
                        Span::styled(format!("{:<15} ", name), folder_name_style),
                    ];

                    if name.eq_ignore_ascii_case("zapret") {
                        if app.is_zapret_installed() {
                            line_spans.push(Span::styled("[🟢 Установлен] ", Style::default().fg(Color::Green)));
                        } else {
                            line_spans.push(Span::styled("[🔴 Не установлен] ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)));
                        }
                    } else if name.eq_ignore_ascii_case("telegram proxy") || name.eq_ignore_ascii_case("tg proxy") {
                        if app.is_tg_proxy_installed() {
                            line_spans.push(Span::styled("[🟢 Установлен] ", Style::default().fg(Color::Green)));
                        } else {
                            line_spans.push(Span::styled("[🔴 Не установлен] ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)));
                        }
                    } else if name.to_lowercase().contains("автозагруз") || name.to_lowercase().contains("трей") {
                        let (dark_auto, tg_auto, _) = app.get_system_autorun_summary();
                        if dark_auto || tg_auto {
                            line_spans.push(Span::styled("[🟢 Автозапуск] ", Style::default().fg(Color::Green)));
                        } else {
                            line_spans.push(Span::styled("[⚪ Обычный] ", Style::default().fg(Theme::MUTED)));
                        }
                    }

                    line_spans.push(Span::styled(format!("({} действий)", count), Style::default().fg(Theme::MUTED)));
                    line_spans.push(Span::styled("  →", Style::default().fg(Theme::PRIMARY)));

                    ListItem::new(Line::from(line_spans))
                }
                ViewItem::Back => {
                    let line = Line::from(vec![
                        Span::styled(prefix, if is_selected { Style::default().fg(Theme::PRIMARY) } else { Style::default() }),
                        Span::styled(
                            "↩  .. [ Назад в главное меню ]",
                            if is_selected {
                                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Theme::MUTED)
                            },
                        ),
                    ]);
                    ListItem::new(line)
                }
                ViewItem::Action(action) => {
                    let cat_style = Theme::category_badge(&action.category);
                    let shortcut_span = if let Some(ref sc) = action.shortcut {
                        Span::styled(format!("[{}] ", sc), Style::default().fg(Color::Yellow))
                    } else {
                        Span::raw("")
                    };

                    let mut spans = vec![
                        Span::styled(prefix, if is_selected { Style::default().fg(Theme::PRIMARY) } else { Style::default() }),
                    ];

                    // If we are in search mode, also display category tag
                    if !app.input_buffer.is_empty() {
                        spans.push(Span::styled(format!("{:<10} ", format!("[{}]", action.category)), cat_style));
                    }

                    // Dynamically name install action if not installed
                    let display_name: &str = if action.id == "zapret-update" && !app.is_zapret_installed() {
                        "Установить Zapret"
                    } else if action.id == "tg-proxy-update" && !app.is_tg_proxy_installed() {
                        "Установить TG WS Proxy"
                    } else {
                        &action.name
                    };

                    spans.push(Span::styled(
                        format!("{:<22} ", display_name),
                        if is_selected { Theme::selected_item() } else { Style::default().fg(Color::White) },
                    ));
                    spans.push(shortcut_span);
                    spans.push(Span::styled(format!("({})", action.id), Style::default().fg(Theme::MUTED)));

                    ListItem::new(Line::from(spans))
                }
            }
        })
        .collect();

    let title_text = if !app.input_buffer.is_empty() {
        format!(" 🔍 Результаты поиска ({}) ", items.len())
    } else if let Some(ref folder) = app.current_folder {
        if folder.eq_ignore_ascii_case("zapret") {
            if let Some(p) = app.get_zapret_path() {
                format!(" 📁 Главное меню / Zapret [🟢 {}] ({}) ", p.display(), items.len().saturating_sub(1))
            } else {
                format!(" 📁 Главное меню / Zapret [🔴 Не установлен] ({}) ", items.len().saturating_sub(1))
            }
        } else {
            format!(" 📁 Главное меню / {} ({}) ", folder, items.len().saturating_sub(1))
        }
    } else {
        format!(" 📁 Главное меню (папок: {}) ", items.len())
    };

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border_active())
        .title(Span::styled(title_text, Theme::title()));

    let list_widget = List::new(list_items).block(list_block);
    f.render_widget(list_widget, chunks[0]);

    // 2. Detail Panel (Right Pane)
    let detail_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border())
        .title(Span::styled(" Детали / Информация ", Theme::title()));

    if let Some(selected_item) = items.get(app.selected_action_idx) {
        match selected_item {
            ViewItem::Folder { name, count } => {
                let mut lines = vec![
                    Line::from(vec![
                        Span::styled("Папка:         ", Style::default().fg(Theme::MUTED)),
                        Span::styled(name, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(vec![
                        Span::styled("Кол-во команд: ", Style::default().fg(Theme::MUTED)),
                        Span::styled(format!("{}", count), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]),
                ];

                if name.eq_ignore_ascii_case("zapret") {
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled("─── СТАТУС УСТАНОВКИ ZAPRET ───", Style::default().fg(Theme::PRIMARY).add_modifier(Modifier::BOLD)),
                    ]));
                    let (is_installed, path, version) = app.get_zapret_summary();
                    if is_installed {
                        lines.push(Line::from(vec![
                            Span::styled("Статус:        ", Style::default().fg(Theme::MUTED)),
                            Span::styled("🟢 Установлен на этом ПК", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                        ]));
                        if let Some(p) = path {
                            lines.push(Line::from(vec![
                                Span::styled("Папка:         ", Style::default().fg(Theme::MUTED)),
                                Span::styled(p.display().to_string(), Style::default().fg(Color::Cyan)),
                            ]));
                        }
                        if let Some(v) = version {
                            lines.push(Line::from(vec![
                                Span::styled("Версия:        ", Style::default().fg(Theme::MUTED)),
                                Span::styled(v, Style::default().fg(Color::White)),
                            ]));
                        }
                    } else {
                        lines.push(Line::from(vec![
                            Span::styled("Статус:        ", Style::default().fg(Theme::MUTED)),
                            Span::styled("🔴 НЕ УСТАНОВЛЕН НА ЭТОМ ПК", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                        ]));
                        lines.push(Line::from(vec![
                            Span::styled("Действие:      ", Style::default().fg(Theme::MUTED)),
                            Span::styled("Нажмите [u] или выберите установку (в C:\\zapret или свою папку)", Style::default().fg(Color::Yellow)),
                        ]));
                    }
                } else if name.eq_ignore_ascii_case("telegram proxy") || name.eq_ignore_ascii_case("tg proxy") {
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled("─── СТАТУС TG WS PROXY ───", Style::default().fg(Theme::PRIMARY).add_modifier(Modifier::BOLD)),
                    ]));
                    let (is_installed, path, version, is_running, port) = app.get_tg_proxy_summary();
                    if is_installed {
                        lines.push(Line::from(vec![
                            Span::styled("Статус:        ", Style::default().fg(Theme::MUTED)),
                            Span::styled("🟢 Установлен на этом ПК", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                        ]));
                        if let Some(p) = path {
                            lines.push(Line::from(vec![
                                Span::styled("Файл:          ", Style::default().fg(Theme::MUTED)),
                                Span::styled(p.display().to_string(), Style::default().fg(Color::Cyan)),
                            ]));
                        }
                        if let Some(v) = version {
                            lines.push(Line::from(vec![
                                Span::styled("Версия:        ", Style::default().fg(Theme::MUTED)),
                                Span::styled(v, Style::default().fg(Color::White)),
                            ]));
                        }
                        let proc_str = if is_running { "🟢 Работает" } else { "🔴 Остановлен" };
                        lines.push(Line::from(vec![
                            Span::styled("Процесс:       ", Style::default().fg(Theme::MUTED)),
                            Span::styled(proc_str, if is_running { Style::default().fg(Color::Green) } else { Style::default().fg(Color::Red) }),
                            Span::styled(format!(" (порт {})", port), Style::default().fg(Theme::MUTED)),
                        ]));
                    } else {
                        lines.push(Line::from(vec![
                            Span::styled("Статус:        ", Style::default().fg(Theme::MUTED)),
                            Span::styled("🔴 НЕ УСТАНОВЛЕН НА ЭТОМ ПК", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                        ]));
                        lines.push(Line::from(vec![
                            Span::styled("Действие:      ", Style::default().fg(Theme::MUTED)),
                            Span::styled("Нажмите [u] для загрузки TgWsProxy_windows.exe с GitHub", Style::default().fg(Color::Yellow)),
                        ]));
                    }
                } else if name.to_lowercase().contains("автозагруз") || name.to_lowercase().contains("трей") {
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled("─── СИСТЕМА И АВТОЗАГРУЗКА ───", Style::default().fg(Theme::PRIMARY).add_modifier(Modifier::BOLD)),
                    ]));
                    let (dark_auto, tg_auto, zap_auto) = app.get_system_autorun_summary();
                    lines.push(Line::from(vec![
                        Span::styled("Dark-CLI в трей: ", Style::default().fg(Theme::MUTED)),
                        Span::styled(
                            if dark_auto { "🟢 Включен (запуск свернутым)" } else { "⚪ Отключен" },
                            if dark_auto { Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::MUTED) },
                        ),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("TG WS Proxy:     ", Style::default().fg(Theme::MUTED)),
                        Span::styled(
                            if tg_auto { "🟢 Включен (старт в трее)" } else { "⚪ Отключен" },
                            if tg_auto { Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::MUTED) },
                        ),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Служба Zapret:   ", Style::default().fg(Theme::MUTED)),
                        Span::styled(zap_auto, Style::default().fg(Color::Cyan)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Трей Windows:    ", Style::default().fg(Theme::MUTED)),
                        Span::styled("🟢 Активен (нажмите [h] чтобы скрыть)", Style::default().fg(Color::Green)),
                    ]));
                }

                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Содержимое папки:", Style::default().fg(Theme::PRIMARY).add_modifier(Modifier::BOLD)),
                ]));

                for action in app.config.actions.iter().filter(|a| a.category.eq_ignore_ascii_case(name)).take(6) {
                    let sc = action.shortcut.as_ref().map(|s| format!(" [{}]", s)).unwrap_or_default();
                    lines.push(Line::from(vec![
                        Span::styled(format!("  • {}", action.name), Style::default().fg(Color::White)),
                        Span::styled(sc, Style::default().fg(Color::Yellow)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled(format!("    {}", action.description), Style::default().fg(Theme::MUTED)),
                    ]));
                }
                if *count > 6 {
                    lines.push(Line::from(vec![
                        Span::styled(format!("    ... и еще {} команд", count - 6), Style::default().fg(Theme::MUTED)),
                    ]));
                }

                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("─── Навигация ───", Style::default().fg(Theme::MUTED)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("Нажмите ", Style::default().fg(Theme::MUTED)),
                    Span::styled("[Enter]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::styled(" или ", Style::default().fg(Theme::MUTED)),
                    Span::styled("[→]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::styled(" чтобы войти в папку", Style::default().fg(Theme::MUTED)),
                ]));

                let detail_p = Paragraph::new(lines).block(detail_block).wrap(Wrap { trim: false });
                f.render_widget(detail_p, chunks[1]);
            }
            ViewItem::Back => {
                let lines = vec![
                    Line::from(vec![
                        Span::styled("Возврат на уровень выше", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Текущая папка: ", Style::default().fg(Theme::MUTED)),
                        Span::styled(app.current_folder.as_deref().unwrap_or(""), Style::default().fg(Color::Yellow)),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("─── Навигация ───", Style::default().fg(Theme::MUTED)),
                    ]),
                    Line::from(vec![
                        Span::styled("Нажмите ", Style::default().fg(Theme::MUTED)),
                        Span::styled("[Enter]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        Span::styled(", ", Style::default().fg(Theme::MUTED)),
                        Span::styled("[Esc]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        Span::styled(" или ", Style::default().fg(Theme::MUTED)),
                        Span::styled("[←]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        Span::styled(" для возврата в корень", Style::default().fg(Theme::MUTED)),
                    ]),
                ];

                let detail_p = Paragraph::new(lines).block(detail_block).wrap(Wrap { trim: false });
                f.render_widget(detail_p, chunks[1]);
            }
            ViewItem::Action(action) => {
                let sc_text = action.shortcut.as_deref().unwrap_or("Нет");
                let cwd_text = action.cwd.as_deref().unwrap_or("Текущая папка");

                let mut lines = vec![
                    Line::from(vec![
                        Span::styled("Название:     ", Style::default().fg(Theme::MUTED)),
                        Span::styled(&action.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(vec![
                        Span::styled("Код (ID):     ", Style::default().fg(Theme::MUTED)),
                        Span::styled(&action.id, Style::default().fg(Color::Cyan)),
                    ]),
                    Line::from(vec![
                        Span::styled("Папка/Группа: ", Style::default().fg(Theme::MUTED)),
                        Span::styled(&action.category, Theme::category_badge(&action.category)),
                    ]),
                    Line::from(vec![
                        Span::styled("Клавиша:      ", Style::default().fg(Theme::MUTED)),
                        Span::styled(sc_text, Style::default().fg(Color::Yellow)),
                    ]),
                    Line::from(vec![
                        Span::styled("Рабочая папка:", Style::default().fg(Theme::MUTED)),
                        Span::styled(format!(" {}", cwd_text), Style::default().fg(Color::Gray)),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Команда:", Style::default().fg(Theme::PRIMARY).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(vec![
                        Span::styled(format!("  $ {}", action.command), Style::default().fg(Color::Green)),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Описание:", Style::default().fg(Theme::PRIMARY).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(vec![
                        Span::styled(format!("  {}", action.description), Style::default().fg(Color::White)),
                    ]),
                ];

                if action.category.eq_ignore_ascii_case("zapret") {
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled("─── СТАТУС ZAPRET ───", Style::default().fg(Theme::PRIMARY).add_modifier(Modifier::BOLD)),
                    ]));
                    if let Some(p) = app.get_zapret_path() {
                        lines.push(Line::from(vec![
                            Span::styled("Установка:    ", Style::default().fg(Theme::MUTED)),
                            Span::styled(format!("🟢 Установлен ({})", p.display()), Style::default().fg(Color::Green)),
                        ]));
                    } else {
                        lines.push(Line::from(vec![
                            Span::styled("Установка:    ", Style::default().fg(Theme::MUTED)),
                            Span::styled("🔴 Не установлен на этом ПК", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                        ]));
                        lines.push(Line::from(vec![
                            Span::styled("Подсказка:    ", Style::default().fg(Theme::MUTED)),
                            Span::styled("При запуске будет предложено выбрать папку установки", Style::default().fg(Color::Yellow)),
                        ]));
                    }
                } else if action.category.eq_ignore_ascii_case("telegram proxy") || action.category.eq_ignore_ascii_case("tg proxy") {
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled("─── СТАТУС TG WS PROXY ───", Style::default().fg(Theme::PRIMARY).add_modifier(Modifier::BOLD)),
                    ]));
                    if let Some(p) = app.get_tg_proxy_path() {
                        let (is_running, port) = app.get_tg_proxy_running_state();
                        let state_str = if is_running { "🟢 Работает" } else { "🔴 Остановлен" };
                        lines.push(Line::from(vec![
                            Span::styled("Программа:    ", Style::default().fg(Theme::MUTED)),
                            Span::styled(format!("🟢 Установлен ({})", p.file_name().unwrap_or_default().to_string_lossy()), Style::default().fg(Color::Green)),
                        ]));
                        lines.push(Line::from(vec![
                            Span::styled("Процесс:      ", Style::default().fg(Theme::MUTED)),
                            Span::styled(state_str, if is_running { Style::default().fg(Color::Green) } else { Style::default().fg(Color::Red) }),
                            Span::styled(format!(" (порт {})", port), Style::default().fg(Theme::MUTED)),
                        ]));
                    } else {
                        lines.push(Line::from(vec![
                            Span::styled("Установка:    ", Style::default().fg(Theme::MUTED)),
                            Span::styled("🔴 Не установлен на этом ПК", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                        ]));
                        lines.push(Line::from(vec![
                            Span::styled("Подсказка:    ", Style::default().fg(Theme::MUTED)),
                            Span::styled("Нажмите [u] для загрузки с GitHub", Style::default().fg(Color::Yellow)),
                        ]));
                    }
                }

                lines.push(Line::from(""));
                lines.extend(vec![
                    Line::from(vec![
                        Span::styled("─── Быстрый запуск ───", Style::default().fg(Theme::MUTED)),
                    ]),
                    Line::from(vec![
                        Span::styled("Нажмите ", Style::default().fg(Theme::MUTED)),
                        Span::styled("[Enter]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        Span::styled(" для запуска этого действия", Style::default().fg(Theme::MUTED)),
                    ]),
                    Line::from(vec![
                        Span::styled("Нажмите ", Style::default().fg(Theme::MUTED)),
                        Span::styled("[Esc] / [←]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        Span::styled(" для возврата в список папок", Style::default().fg(Theme::MUTED)),
                    ]),
                ]);

                let detail_p = Paragraph::new(lines).block(detail_block).wrap(Wrap { trim: false });
                f.render_widget(detail_p, chunks[1]);
            }
        }
    } else {
        let empty_p = Paragraph::new("Список пуст. Нажмите [Esc] для возврата.")
            .block(detail_block)
            .style(Style::default().fg(Theme::MUTED));
        f.render_widget(empty_p, chunks[1]);
    }
}
