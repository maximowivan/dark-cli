mod app;
mod autorun;
mod config;
mod executor;
mod self_update;
mod system_tray;
mod tg_proxy;
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
use tg_proxy::TgProxyManager;
use zapret::ZapretManager;

#[derive(Parser)]
#[command(name = "dark-cli")]
#[command(about = "Универсальный CLI/TUI лаунчер и диспетчер задач в стиле OpenCode", long_about = None)]
struct Cli {
    /// Запустить Dark-CLI свернутым в системный трей
    #[arg(long)]
    tray: bool,

    /// Отключить автоматическую проверку обновлений при запуске
    #[arg(long, global = true)]
    no_update: bool,

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
    /// Проверить и обновить Dark-CLI до последней версии с GitHub
    SelfUpdate {
        /// Принудительно обновить даже при совпадении версии
        #[arg(short, long)]
        force: bool,
    },
    /// Управление сервисом и обновлениями Flowseal Zapret
    Zapret {
        #[command(subcommand)]
        action: ZapretCommands,
    },
    /// Управление локальным MTProto WebSocket прокси для Telegram (Flowseal tg-ws-proxy)
    TgProxy {
        #[command(subcommand)]
        action: TgProxyCommands,
    },
    /// Управление автозагрузкой Windows и системным треем
    System {
        #[command(subcommand)]
        action: SystemCommands,
    },
}

#[derive(Subcommand)]
enum SystemCommands {
    /// Проверить статус автозагрузки Dark-CLI, TG WS Proxy и службы Zapret
    Status,
    /// Свернуть окно Dark-CLI в системный трей возле часов
    TrayHide,
    /// Восстановить окно Dark-CLI из системного трея
    TrayShow,
    /// Включить автозапуск Dark-CLI при старте Windows (свернутым в трей)
    AutorunEnable,
    /// Отключить автозапуск Dark-CLI
    AutorunDisable,
    /// Включить автозапуск Flowseal TG WS Proxy в Windows
    TgAutorunEnable,
    /// Отключить автозапуск Flowseal TG WS Proxy
    TgAutorunDisable,
}

#[derive(Subcommand)]
enum ZapretCommands {
    /// Проверить статус службы, процессов и актуальность версии
    Status,
    /// Установить Zapret с GitHub (с выбором папки)
    Install {
        /// Путь для установки (по умолчанию C:\zapret)
        #[arg(short, long)]
        path: Option<String>,
    },
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
        /// Путь к папке Zapret для обновления/установки
        #[arg(short, long)]
        path: Option<String>,
    },
    /// Открыть папку с программой Zapret в Проводнике
    Open,
    /// Установить службу Windows zapret со стратегией (по умолчанию ALT11)
    ServiceInstall {
        /// Название стратегии (например "general (ALT11)")
        #[arg(short, long)]
        strategy: Option<String>,
    },
    /// Удалить службу Windows zapret из системы
    ServiceRemove,
    /// Установить и настроить Zapret под ключ (автоустановка в C:\zapret, тест всех стратегий, запуск службы и проверка)
    EasySetup,
    /// Запустить оригинальный service.bat от имени администратора
    Manager,
}

#[derive(Subcommand)]
enum TgProxyCommands {
    /// Проверить статус прокси, процесса, порта 1443 и ссылки для подключения
    Status,
    /// Запустить TG WS Proxy в фоновом режиме (с иконкой в трее)
    Start,
    /// Остановить процессы TG WS Proxy
    Stop,
    /// Перезапустить TG WS Proxy
    Restart,
    /// Открыть окно добавления прокси прямо в Telegram Desktop (tg://proxy)
    Connect,
    /// Скопировать готовую ссылку tg://proxy в буфер обмена Windows
    CopyLink,
    /// Просмотреть последние строки журнала proxy.log
    Logs,
    /// Открыть папку с программой или логами в Проводнике
    Open,
    /// Загрузить или обновить TgWsProxy_windows.exe с GitHub Flowseal
    Update {
        /// Принудительно обновить
        #[arg(short, long)]
        force: bool,
        /// Путь к папке или файлу для сохранения
        #[arg(short, long)]
        path: Option<String>,
    },
    /// Установить TG WS Proxy с GitHub
    Install {
        /// Путь для установки (по умолчанию C:\tg-ws-proxy)
        #[arg(short, long)]
        path: Option<String>,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let (config, config_path) = AppConfig::load_or_create()
        .map_err(|e| format!("Ошибка конфигурации: {}", e))?;

    // Очистка старых .old файлов после автообновления
    self_update::SelfUpdateManager::cleanup_old_binary();

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
        Some(Commands::SelfUpdate { force }) => {
            let repo = self_update::DEFAULT_REPO;
            let cur_ver = env!("CARGO_PKG_VERSION");
            println!("🔍 Проверка обновлений Dark-CLI на GitHub ({})...", repo);
            println!("🏷 Текущая версия: v{}", cur_ver);

            match self_update::SelfUpdateManager::check_for_update(repo) {
                Ok(Some(asset)) => {
                    println!("\n🚀 Доступна новая версия: {}!", asset.tag_name);
                    println!("⬇ Загрузка {}...", asset.asset_name);
                    match self_update::SelfUpdateManager::apply_update(&asset) {
                        Ok(_) => {
                            println!("✨ Dark-CLI успешно обновлен до {}!", asset.tag_name);
                            println!("🚀 Перезапуск обновленной версии...");
                            let _ = self_update::SelfUpdateManager::restart_process();
                        }
                        Err(e) => {
                            eprintln!("❌ Ошибка применения обновления: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Ok(None) => {
                    if force {
                        println!("⚡ Принудительная переустановка текущей версии...");
                    } else {
                        println!("✨ У вас уже установлена актуальная версия Dark-CLI (v{})!", cur_ver);
                    }
                }
                Err(e) => {
                    eprintln!("⚠ Не удалось проверить обновления: {}", e);
                    std::process::exit(1);
                }
            }
            return Ok(());
        }
        Some(Commands::Zapret { action }) => {
            let zapret_cfg = config.zapret.clone().unwrap_or_default();
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
                ZapretCommands::Install { path } => {
                    let chosen_path = match path {
                        Some(p) => std::path::PathBuf::from(p),
                        None => match ZapretManager::prompt_install_path("C:\\zapret") {
                            Ok(p) => p,
                            Err(e) => {
                                eprintln!("{}", e);
                                std::process::exit(1);
                            }
                        },
                    };
                    let mut updated_config = config.clone();
                    updated_config.set_zapret_path(chosen_path.display().to_string());
                    let _ = updated_config.save(&config_path);

                    let effective_zapret_cfg = updated_config.zapret.unwrap_or_default();
                    match ZapretManager::update(&effective_zapret_cfg, Some(&chosen_path), true) {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("Ошибка установки: {}", e);
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
                ZapretCommands::Update { force, path } => {
                    let target_path = if let Some(p) = path {
                        let pb = std::path::PathBuf::from(p);
                        let mut updated_config = config.clone();
                        updated_config.set_zapret_path(pb.display().to_string());
                        let _ = updated_config.save(&config_path);
                        Some(pb)
                    } else if zapret_cfg.get_resolved_path().is_none() {
                        let pb = match ZapretManager::prompt_install_path("C:\\zapret") {
                            Ok(p) => p,
                            Err(e) => {
                                eprintln!("{}", e);
                                std::process::exit(1);
                            }
                        };
                        let mut updated_config = config.clone();
                        updated_config.set_zapret_path(pb.display().to_string());
                        let _ = updated_config.save(&config_path);
                        Some(pb)
                    } else {
                        None
                    };

                    match ZapretManager::update(&zapret_cfg, target_path.as_deref(), force) {
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
                ZapretCommands::ServiceInstall { strategy } => {
                    let strat_to_use = match strategy {
                        Some(s) => Some(s),
                        None => {
                            if let Some(path) = zapret_cfg.get_resolved_path() {
                                let list = ZapretManager::list_available_strategies(&path);
                                if !list.is_empty() {
                                    println!("\n====================================================");
                                    println!("       ВЫБОР СТРАТЕГИИ ДЛЯ СЛУЖБЫ ZAPRET            ");
                                    println!("====================================================");
                                    println!("Доступные стратегии в папке:");
                                    for (i, st) in list.iter().enumerate() {
                                        println!("  [{}] {}", i + 1, st);
                                    }
                                    println!("\nНажмите Enter для использования 'general (ALT11)' по умолчанию");
                                    print!("или введите номер стратегии: ");
                                    let _ = std::io::Write::flush(&mut std::io::stdout());
                                    let mut input = String::new();
                                    let _ = std::io::stdin().read_line(&mut input);
                                    let trimmed = input.trim();
                                    if trimmed.is_empty() {
                                        Some("general (ALT11)".to_string())
                                    } else if let Ok(num) = trimmed.parse::<usize>() {
                                        if num >= 1 && num <= list.len() {
                                            Some(list[num - 1].clone())
                                        } else {
                                            Some(trimmed.to_string())
                                        }
                                    } else {
                                        Some(trimmed.to_string())
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
                    };

                    match ZapretManager::install_service(&zapret_cfg, strat_to_use.as_deref()) {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("Ошибка установки службы: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                ZapretCommands::ServiceRemove => {
                    match ZapretManager::remove_service(&zapret_cfg) {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("Ошибка удаления службы: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                ZapretCommands::EasySetup => {
                    match ZapretManager::easy_setup(&config, &config_path) {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("Ошибка автоматической настройки: {}", e);
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
        Some(Commands::TgProxy { action }) => {
            let tg_cfg = config.tg_proxy.clone().unwrap_or_default();
            match action {
                TgProxyCommands::Status => {
                    println!("{}", TgProxyManager::get_status(&tg_cfg));
                }
                TgProxyCommands::Start => {
                    match TgProxyManager::start(&tg_cfg) {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("Ошибка запуска: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                TgProxyCommands::Stop => {
                    match TgProxyManager::stop(&tg_cfg) {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("Ошибка остановки: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                TgProxyCommands::Restart => {
                    match TgProxyManager::restart(&tg_cfg) {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("Ошибка перезапуска: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                TgProxyCommands::Connect => {
                    match TgProxyManager::connect_telegram(&tg_cfg) {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("Ошибка подключения: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                TgProxyCommands::CopyLink => {
                    match TgProxyManager::copy_link(&tg_cfg) {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("Ошибка копирования ссылки: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                TgProxyCommands::Logs => {
                    match TgProxyManager::open_logs(&tg_cfg) {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("Ошибка чтения логов: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                TgProxyCommands::Open => {
                    match TgProxyManager::open_folder(&tg_cfg) {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("Ошибка открытия папки: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                TgProxyCommands::Update { force, path } => {
                    let target = path.as_deref().map(std::path::Path::new);
                    match TgProxyManager::update(&tg_cfg, target, force) {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("Ошибка обновления: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                TgProxyCommands::Install { path } => {
                    let target_str = path.unwrap_or_else(|| "C:\\tg-ws-proxy".to_string());
                    let target_path = std::path::PathBuf::from(&target_str);
                    let mut updated_config = config.clone();
                    updated_config.set_tg_proxy_path(target_path.join("TgWsProxy_windows.exe").display().to_string());
                    let _ = updated_config.save(&config_path);

                    let effective_cfg = updated_config.tg_proxy.unwrap_or_default();
                    match TgProxyManager::update(&effective_cfg, Some(&target_path), true) {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("Ошибка установки: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
            }
            return Ok(());
        }
        Some(Commands::System { action }) => {
            match action {
                SystemCommands::Status => {
                    println!("{}", autorun::AutorunManager::get_status_report());
                }
                SystemCommands::TrayHide => {
                    system_tray::window_control::hide_console();
                    println!("Окно Dark-CLI свернуто в системный трей.");
                }
                SystemCommands::TrayShow => {
                    system_tray::window_control::show_console();
                    println!("Окно Dark-CLI восстановлено из системного трея.");
                }
                SystemCommands::AutorunEnable => {
                    match autorun::AutorunManager::enable_dark_cli() {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("Ошибка: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                SystemCommands::AutorunDisable => {
                    match autorun::AutorunManager::disable_dark_cli() {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("Ошибка: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                SystemCommands::TgAutorunEnable => {
                    let tg_path = config.tg_proxy.as_ref().and_then(|t| t.path.as_deref()).map(std::path::Path::new);
                    match autorun::AutorunManager::enable_tg_proxy(tg_path) {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("Ошибка: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                SystemCommands::TgAutorunDisable => {
                    match autorun::AutorunManager::disable_tg_proxy() {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => {
                            eprintln!("Ошибка: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
            }
            return Ok(());
        }
        None => {
            // Если передан флаг --tray, сворачиваем окно в трей сразу при старте
            if cli.tray {
                system_tray::window_control::hide_console();
            }

            // Автоматическая проверка обновлений при старте (если не отключена флагом --no-update)
            if !cli.no_update {
                if self_update::SelfUpdateManager::auto_update_on_startup(self_update::DEFAULT_REPO) {
                    return Ok(());
                }
            }

            // Launch Interactive OpenCode TUI
            run_tui(config, config_path)?;
        }
    }

    Ok(())
}

fn run_tui(config: AppConfig, config_path: std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Инициализация иконки системного трея Windows
    let _tray = system_tray::TrayManager::init().ok();
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

        if app.last_status_refresh.elapsed() > Duration::from_secs(10) {
            app.refresh_statuses();
        }

        if let Some(action) = app.pending_action.take() {
            app.execute_action(&action);
            terminal.draw(|f| ui::render(f, app))?;
            continue;
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

                if let Some(ref dialog) = app.install_dialog.clone() {
                    match dialog {
                        app::InstallDialogState::ChooseOption { selected } => match key.code {
                            KeyCode::Esc => {
                                app.install_dialog = None;
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.install_dialog = Some(app::InstallDialogState::ChooseOption { selected: 0 });
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                app.install_dialog = Some(app::InstallDialogState::ChooseOption { selected: 1 });
                            }
                            KeyCode::Char('1') => {
                                app.confirm_install_path("C:\\zapret");
                            }
                            KeyCode::Char('2') => {
                                app.install_dialog = Some(app::InstallDialogState::EnterPath {
                                    input: String::from("C:\\zapret"),
                                });
                            }
                            KeyCode::Enter => {
                                if *selected == 0 {
                                    app.confirm_install_path("C:\\zapret");
                                } else {
                                    app.install_dialog = Some(app::InstallDialogState::EnterPath {
                                        input: String::from("C:\\zapret"),
                                    });
                                }
                            }
                            _ => {}
                        },
                        app::InstallDialogState::EnterPath { input } => match key.code {
                            KeyCode::Esc => {
                                app.install_dialog = Some(app::InstallDialogState::ChooseOption { selected: 1 });
                            }
                            KeyCode::Enter => {
                                let path_to_install = input.clone();
                                app.confirm_install_path(&path_to_install);
                            }
                            KeyCode::Backspace => {
                                let mut new_inp = input.clone();
                                new_inp.pop();
                                app.install_dialog = Some(app::InstallDialogState::EnterPath { input: new_inp });
                            }
                            KeyCode::Char(c) => {
                                let mut new_inp = input.clone();
                                new_inp.push(c);
                                app.install_dialog = Some(app::InstallDialogState::EnterPath { input: new_inp });
                            }
                            _ => {}
                        },
                    }
                    continue;
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
                        KeyCode::Char('?') => {
                            app.active_tab = Tab::Help;
                        }
                        KeyCode::F(1) => app.active_tab = Tab::Actions,
                        KeyCode::F(2) => app.active_tab = Tab::Output,
                        KeyCode::F(3) => app.active_tab = Tab::History,
                        KeyCode::F(4) => app.active_tab = Tab::Help,
                        KeyCode::F(5) => app.reload_config(),
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
                        KeyCode::Left | KeyCode::Backspace | KeyCode::Esc => {
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
                            let matching_action = if app.active_tab == Tab::Actions {
                                if let Some(ref current_cat) = app.current_folder {
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
                                }
                            } else {
                                None
                            };

                            if let Some(action) = matching_action {
                                if action.id == "zapret-install" || (action.id == "zapret-update" && !app.is_zapret_installed()) {
                                    app.start_zapret_install_dialog();
                                } else {
                                    app.trigger_action(action);
                                }
                            } else {
                                match c {
                                    '1' => app.active_tab = Tab::Actions,
                                    '2' => app.active_tab = Tab::Output,
                                    '3' => app.active_tab = Tab::History,
                                    '4' => app.active_tab = Tab::Help,
                                    'r' => app.reload_config(),
                                    'h' => {
                                        if app.active_tab == Tab::Actions {
                                            if !app.go_back() {
                                                // В главном меню сворачиваем окно в системный трей
                                                system_tray::window_control::hide_console();
                                            }
                                        }
                                    }
                                    _ => {}
                                }
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
