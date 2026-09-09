mod app;
mod config;
mod executor;
mod ui;
mod zapret;

use app::{App, InputMode, Tab, ViewItem};
use clap::{Parser, Subcommand};
use config::AppConfig;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, stdout};
use std::panic;
use std::time::Duration;
use zapret::ZapretManager;

#[derive(Parser)]
#[command(name = "dark-cli")]
#[command(about = "Универсальный CLI/TUI лаунчер и диспетчер задач в стиле OpenCode", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Запустить действие напрямую по его ID без открытия TUI
    Run {
        /// Идентификатор действия (например, zapret-status, zapret-restart)
        id: String,
    },
    /// Вывести список всех настроенных команд в виде таблицы
    List,
    /// Сгенерировать config.toml по умолчанию
    Init,
    /// Управление сервисом и обновлениями Flowseal Zapret
    Zapret {
        #[command(subcommand)]
        action: ZapretCommands,
    },
}

#[derive(Subcommand)]
enum ZapretCommands {
    /// Проверить статус службы, процессов и актуальность версии
    Status,
    /// Запустить службу Zapret
    Start,
    /// Остановить службу Zapret и процессы winws
    Stop,
    /// Перезапустить службу Zapret
    Restart,
    /// Обновить Zapret до последней версии с GitHub (сохраняя пользовательские списки)
    Update {
        /// Принудительно обновить, даже если версия уже совпадает
        #[arg(short, long)]
        force: bool,
    },
    /// Открыть папку с программой Zapret в Проводнике
    Open,
    /// Запустить оригинальный service.bat от имени администратора
    Manager,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let (config, config_path) = AppConfig::load_or_create()
        .map_err(|e| format!("Ошибка конфигурации: {}", e))?;

    // Handle CLI subcommands
    match cli.command {
        Some(Commands::Run { id }) => {
            if let Some(action) = config.actions.iter().find(|a| a.id.eq_ignore_ascii_case(&id)) {
                println!("▶ Запуск [{}] ({})...", action.name, action.command);
                let result = executor::CommandExecutor::execute(action, config.settings.default_cwd.as_deref());
                println!("{}", result.output);
                println!("\n⏱ Время: {:.2?} | Код: {:?}", result.duration, result.exit_code);
                if !result.success {
                    std::process::exit(result.exit_code.unwrap_or(1));
                }
            } else {
                eprintln!("Ошибка: действие с ID '{}' не найдено в конфигурации.", id);
                eprintln!("Используйте 'dark-cli list' для просмотра доступных команд.");
                std::process::exit(1);
            }
            return Ok(());
        }
        Some(Commands::List) => {
            println!("╔════════════════════════════════════════════════════════════════════════════════╗");
            println!("║                       СПИСОК НАСТРОЕННЫХ КОМАНД (DARK-CLI)                     ║");
            println!("╠══════════════════╦══════════════╦═════╦══════════════════════════════════════════╣");
            println!("║ ID               ║ КАТЕГОРИЯ    ║ КЛВ ║ НАЗВАНИЕ / ОПИСАНИЕ                      ║");
            println!("╠══════════════════╬══════════════╬═════╬══════════════════════════════════════════╣");
            for action in &config.actions {
                let sc = action.shortcut.as_deref().unwrap_or("-");
                println!(
                    "║ {:<16} ║ {:<12} ║ {:<3} ║ {:<40} ║",
                    action.id, action.category, sc, action.name
                );
            }
            println!("╚══════════════════╩══════════════╩═════╩══════════════════════════════════════════╝");
            println!("Файл конфигурации: {}", config_path.display());
            return Ok(());
        }
        Some(Commands::Init) => {
            println!("Конфигурационный файл актуален: {}", config_path.display());
            return Ok(());
        }
        Some(Commands::Zapret { action }) => {
            let zapret_cfg = config.zapret.unwrap_or_default();
            match action {
                ZapretCommands::Status => {
                    println!("{}", ZapretManager::get_status(&zapret_cfg));
                }
                ZapretCommands::Start => {
                    match ZapretManager::start(&zapret_cfg) {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("Ошибка запуска: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                ZapretCommands::Stop => {
                    match ZapretManager::stop(&zapret_cfg) {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("Ошибка остановки: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                ZapretCommands::Restart => {
                    match ZapretManager::restart(&zapret_cfg) {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("Ошибка перезапуска: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                ZapretCommands::Update { force } => {
                    match ZapretManager::update(&zapret_cfg, force) {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("Ошибка обновления: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                ZapretCommands::Open => {
                    match ZapretManager::open_folder(&zapret_cfg) {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("{}", e);
                            std::process::exit(1);
                        }
                    }
                }
                ZapretCommands::Manager => {
                    match ZapretManager::run_manager(&zapret_cfg) {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("{}", e);
                            std::process::exit(1);
                        }
                    }
                }
            }
            return Ok(());
        }
        None => {
            // Launch Interactive OpenCode TUI
            run_tui(config, config_path)?;
        }
    }

    Ok(())
}

fn run_tui(config: AppConfig, config_path: std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Setup panic hook to cleanly restore terminal
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(config, config_path);

    let res = main_loop(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Ошибка работы интерфейса: {:?}", err);
    }

    Ok(())
}

fn main_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;

        if app.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                // Ignore key release events on Windows
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Global shortcut: Ctrl+C quits anytime
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    app.should_quit = true;
                    break;
                }

                match app.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') => {
                            app.should_quit = true;
                            break;
                        }
                        KeyCode::Char('/') => {
                            app.input_mode = InputMode::PaletteSearch;
                            app.input_buffer.clear();
                            app.palette_selected_idx = 0;
                        }
                        KeyCode::Char(':') => {
                            app.input_mode = InputMode::CommandInput;
                            app.input_buffer = "/".to_string();
                        }
                        KeyCode::Char('r') => {
                            app.reload_config();
                        }
                        KeyCode::Char('?') => {
                            app.active_tab = Tab::Help;
                        }
                        KeyCode::Char('1') => app.active_tab = Tab::Actions,
                        KeyCode::Char('2') => app.active_tab = Tab::Output,
                        KeyCode::Char('3') => app.active_tab = Tab::History,
                        KeyCode::Char('4') => app.active_tab = Tab::Help,
                        KeyCode::Tab => app.next_tab(),
                        KeyCode::BackTab => app.prev_tab(),
                        KeyCode::Right | KeyCode::Char('l') => {
                            if app.active_tab == Tab::Actions {
                                let items = app.current_items();
                                if let Some(ViewItem::Folder { .. }) = items.get(app.selected_action_idx) {
                                    app.enter_selected();
                                }
                            }
                        }
                        KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace | KeyCode::Esc => {
                            if app.active_tab == Tab::Actions {
                                app.go_back();
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if app.active_tab == Tab::Actions {
                                app.next_action();
                            } else if app.active_tab == Tab::Output {
                                app.output_scroll = app.output_scroll.saturating_add(1);
                            } else if app.active_tab == Tab::History {
                                app.history_scroll = app.history_scroll.saturating_add(1);
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if app.active_tab == Tab::Actions {
                                app.prev_action();
                            } else if app.active_tab == Tab::Output {
                                app.output_scroll = app.output_scroll.saturating_sub(1);
                            } else if app.active_tab == Tab::History {
                                app.history_scroll = app.history_scroll.saturating_sub(1);
                            }
                        }
                        KeyCode::PageDown => {
                            if app.active_tab == Tab::Output {
                                app.output_scroll = app.output_scroll.saturating_add(10);
                            }
                        }
                        KeyCode::PageUp => {
                            if app.active_tab == Tab::Output {
                                app.output_scroll = app.output_scroll.saturating_sub(10);
                            }
                        }
                        KeyCode::Enter => {
                            if app.active_tab == Tab::Actions {
                                app.enter_selected();
                            }
                        }
                        KeyCode::Char(c) => {
                            let c_str = c.to_string();
                            let matching_action = if let Some(ref current_cat) = app.current_folder {
                                app.config.actions.iter()
                                    .filter(|a| a.category.eq_ignore_ascii_case(current_cat))
                                    .find(|a| a.shortcut.as_ref().map(|s| s.eq_ignore_ascii_case(&c_str)).unwrap_or(false))
                                    .or_else(|| {
                                        app.config.actions.iter().find(|a| {
                                            a.shortcut.as_ref().map(|s| s.eq_ignore_ascii_case(&c_str)).unwrap_or(false)
                                        })
                                    })
                                    .cloned()
                            } else {
                                app.config.actions.iter().find(|a| {
                                    a.shortcut.as_ref().map(|s| s.eq_ignore_ascii_case(&c_str)).unwrap_or(false)
                                }).cloned()
                            };

                            if let Some(action) = matching_action {
                                app.execute_action(&action);
                            }
                        }
                        _ => {}
                    },
                    InputMode::PaletteSearch => match key.code {
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                            app.input_buffer.clear();
                        }
                        KeyCode::Enter => {
                            app.execute_selected_action();
                        }
                        KeyCode::Down => {
                            app.next_palette_item();
                        }
                        KeyCode::Up => {
                            app.prev_palette_item();
                        }
                        KeyCode::Backspace => {
                            app.input_buffer.pop();
                            app.palette_selected_idx = 0;
                        }
                        KeyCode::Char(c) => {
                            app.input_buffer.push(c);
                            app.palette_selected_idx = 0;
                        }
                        _ => {}
                    },
                    InputMode::CommandInput => match key.code {
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                            app.input_buffer.clear();
                        }
                        KeyCode::Enter => {
                            let cmd = app.input_buffer.clone();
                            app.input_mode = InputMode::Normal;
                            app.input_buffer.clear();
                            app.handle_slash_command(&cmd);
                        }
                        KeyCode::Backspace => {
                            app.input_buffer.pop();
                            if app.input_buffer.is_empty() {
                                app.input_mode = InputMode::Normal;
                            }
                        }
                        KeyCode::Char(c) => {
                            app.input_buffer.push(c);
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    Ok(())
}
