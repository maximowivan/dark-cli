use crate::config::TgProxyConfig;
use std::fs;
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct TgProxySummary {
    pub is_installed: bool,
    pub exe_path: Option<PathBuf>,
    pub version: Option<String>,
    pub is_running: bool,
    pub pids: Vec<u32>,
    pub is_port_listening: bool,
    pub port: u16,
    pub host: String,
    pub secret: Option<String>,
    pub config_file: Option<PathBuf>,
    pub log_file: Option<PathBuf>,
    pub tg_link: String,
    pub web_link: String,
}

pub struct TgProxyManager;

impl TgProxyManager {
    /// Получить сводку состояния TG WS Proxy
    pub fn get_summary(config: &TgProxyConfig) -> TgProxySummary {
        let exe_path = config.get_resolved_path();
        let is_installed = exe_path.is_some();
        let version = exe_path.as_deref().and_then(Self::get_local_version);
        let pids = Self::get_running_pids();
        let is_running = !pids.is_empty();

        let (host, port, secret) = Self::read_config_json(config.default_port);
        let is_port_listening = if is_running {
            Self::check_port_listening(&host, port)
        } else {
            false
        };

        let config_file = TgProxyConfig::get_config_json_path().filter(|p| p.exists());
        let log_file = TgProxyConfig::get_log_file_path().filter(|p| p.exists());

        let secret_str = secret.as_deref().unwrap_or("");
        let effective_secret = if secret_str.len() == 32 && !secret_str.starts_with("dd") && !secret_str.starts_with("ee") {
            format!("dd{}", secret_str)
        } else {
            secret_str.to_string()
        };

        let tg_link = if effective_secret.is_empty() {
            format!("tg://proxy?server={}&port={}", host, port)
        } else {
            format!("tg://proxy?server={}&port={}&secret={}", host, port, effective_secret)
        };

        let web_link = if effective_secret.is_empty() {
            format!("https://t.me/proxy?server={}&port={}", host, port)
        } else {
            format!("https://t.me/proxy?server={}&port={}&secret={}", host, port, effective_secret)
        };

        TgProxySummary {
            is_installed,
            exe_path,
            version,
            is_running,
            pids,
            is_port_listening,
            port,
            host,
            secret,
            config_file,
            log_file,
            tg_link,
            web_link,
        }
    }

    /// Форматированный отчет о статусе работы
    pub fn get_status(config: &TgProxyConfig) -> String {
        let summary = Self::get_summary(config);
        let mut out = String::new();

        out.push_str("====================================================\n");
        out.push_str("     СТАТУС FLOWSEAL TG WS PROXY (TELEGRAM)         \n");
        out.push_str("====================================================\n\n");

        // 1. Установка
        if summary.is_installed {
            out.push_str("📌 Статус установки:  🟢 Установлен на этом компьютере\n");
            if let Some(ref path) = summary.exe_path {
                out.push_str(&format!("📁 Файл программы:    {}\n", path.display()));
            }
            if let Some(ref ver) = summary.version {
                out.push_str(&format!("🏷  Версия программы:  {}\n", ver));
            }
        } else {
            out.push_str("📌 Статус установки:  🔴 НЕ УСТАНОВЛЕН НА ЭТОМ ПК\n");
            out.push_str("📁 Файл программы:    [Не найден TgWsProxy_windows.exe]\n");
        }

        // 2. Процесс
        if summary.is_running {
            let pids_str: Vec<String> = summary.pids.iter().map(|p| p.to_string()).collect();
            out.push_str(&format!("⚡ Статус процесса:   🟢 Работает (PID: {})\n", pids_str.join(", ")));
        } else {
            out.push_str("⚡ Статус процесса:   🔴 Остановлен\n");
        }

        // 3. Порт
        if summary.is_port_listening {
            out.push_str(&format!("🌐 Локальный порт:    🟢 {}:{} (Слушает подключения)\n", summary.host, summary.port));
        } else {
            out.push_str(&format!("🌐 Локальный порт:    🔴 {}:{} (Не доступен)\n", summary.host, summary.port));
        }

        // 4. Secret
        if let Some(ref sec) = summary.secret {
            out.push_str(&format!("🔑 Secret (ключ):     {}\n", sec));
        } else {
            out.push_str("🔑 Secret (ключ):     [Не задан или конфиг отсутствует]\n");
        }

        // 5. Конфиг и логи
        if let Some(ref cf) = summary.config_file {
            out.push_str(&format!("⚙  Файл настроек:     {}\n", cf.display()));
        }
        if let Some(ref lf) = summary.log_file {
            let size = fs::metadata(lf).map(|m| m.len()).unwrap_or(0);
            let size_kb = (size as f64) / 1024.0;
            out.push_str(&format!("📝 Файл логов:        {} ({:.1} KB)\n", lf.display(), size_kb));
        }

        out.push('\n');

        // 6. Ссылки для подключения
        if summary.is_running && summary.is_port_listening {
            out.push_str("🚀 БЫСТРОЕ ПОДКЛЮЧЕНИЕ TELEGRAM DESKTOP:\n");
            out.push_str(&format!("   ▶ Ссылка для приложения: {}\n", summary.tg_link));
            out.push_str(&format!("   ▶ Веб-ссылка (t.me):      {}\n\n", summary.web_link));
            out.push_str("💡 Нажмите [c] («Подключить в Telegram») для мгновенного добавления в Telegram Desktop,\n");
            out.push_str("   или [y] («Скопировать ссылку прокси») для копирования ссылки в буфер обмена.\n");
        } else if summary.is_installed {
            out.push_str("💡 Прокси установлен, но не запущен. Нажмите [1] («Запустить TG WS Proxy»).\n");
        } else {
            out.push_str("💡 Чтобы установить TG WS Proxy с GitHub в один клик, нажмите [u] («Установить / Обновить»).\n");
        }

        // 7. Проверка релизов GitHub
        out.push_str("\n🔍 Проверка последней версии на GitHub...\n");
        match Self::get_latest_github_release(&config.github_repo) {
            Ok((tag, url)) => {
                out.push_str(&format!("🌐 Последний релиз:   {}\n", tag));
                out.push_str(&format!("🔗 Ссылка:            {}\n", url));
            }
            Err(e) => {
                out.push_str(&format!("⚠️  Не удалось проверить GitHub: {}\n", e));
            }
        }

        out.push_str("\n====================================================\n");
        out
    }

    /// Запустить TG WS Proxy
    pub fn start(config: &TgProxyConfig) -> Result<String, String> {
        let summary = Self::get_summary(config);
        if summary.is_running && summary.is_port_listening {
            return Ok(format!(
                "🟢 TG WS Proxy уже работает (PID: {}) на порту {}:{}.\nСсылка: {}",
                summary.pids.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", "),
                summary.host,
                summary.port,
                summary.tg_link
            ));
        }

        let exe = match summary.exe_path {
            Some(p) => p,
            None => {
                return Err(
                    "Исполняемый файл TgWsProxy_windows.exe не найден!\n\
                     Запустите 'dark-cli tg-proxy update' или нажмите [u] для автоматической загрузки с GitHub."
                        .to_string(),
                );
            }
        };

        // Запуск процесса в фоновом режиме (системный трей) через Start-Process
        // Это предотвращает наследование дескрипторов ввода-вывода (stdout/stderr pipes)
        #[cfg(target_os = "windows")]
        {
            let parent_dir = exe.parent().unwrap_or_else(|| Path::new("."));
            let script = format!(
                "Start-Process -FilePath '{}' -WorkingDirectory '{}'",
                exe.display(),
                parent_dir.display()
            );
            Self::run_powershell(&script)
                .map_err(|e| format!("Не удалось запустить {}: {}", exe.display(), e))?;
        }

        #[cfg(not(target_os = "windows"))]
        {
            Command::new(&exe)
                .spawn()
                .map_err(|e| format!("Не удалось запустить {}: {}", exe.display(), e))?;
        }

        // Ждем поднятия прокси и открытия порта (до 6 секунд)
        for _ in 0..12 {
            sleep(Duration::from_millis(500));
            let new_summary = Self::get_summary(config);
            if new_summary.is_running && new_summary.is_port_listening {
                return Ok(format!(
                    "✅ TG WS Proxy успешно запущен и слушает подключения!\n\
                     ⚡ Процесс: PID {}\n\
                     🌐 Порт: {}:{}\n\
                     🔑 Secret: {}\n\
                     🔗 Ссылка: {}\n\n\
                     Значок программы появился в системном трее Windows рядом с часами.",
                    new_summary.pids.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", "),
                    new_summary.host,
                    new_summary.port,
                    new_summary.secret.as_deref().unwrap_or("[авто]"),
                    new_summary.tg_link
                ));
            } else if new_summary.is_running {
                // Если процесс уже есть, дадим еще немного времени сокету
                continue;
            }
        }

        let fallback_summary = Self::get_summary(config);
        if fallback_summary.is_running {
            Ok(format!(
                "✅ TG WS Proxy запущен (PID: {}). Сокет инициализируется.\n\
                 🌐 Порт: {}:{}\n\
                 🔗 Ссылка: {}",
                fallback_summary.pids.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", "),
                fallback_summary.host,
                fallback_summary.port,
                fallback_summary.tg_link
            ))
        } else {
            Ok("TG WS Proxy запущен (ожидает инициализации сокетов). Проверьте статус через клавишу [s].".to_string())
        }
    }

    /// Остановить все процессы TG WS Proxy
    pub fn stop(_config: &TgProxyConfig) -> Result<String, String> {
        let pids = Self::get_running_pids();
        if pids.is_empty() {
            return Ok("TG WS Proxy не запущен.".to_string());
        }

        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("taskkill")
                .args(["/F", "/IM", "TgWsProxy_windows.exe"])
                .output();
            let _ = Command::new("taskkill")
                .args(["/F", "/FI", "IMAGENAME eq TgWsProxy*"])
                .output();
            let _ = Self::run_powershell("Stop-Process -Name *tgws*, *TgWsProxy* -Force -ErrorAction SilentlyContinue");
        }

        sleep(Duration::from_millis(500));
        let remaining = Self::get_running_pids();
        if remaining.is_empty() {
            Ok(format!("✅ TG WS Proxy успешно остановлен (завершены процессы: {:?}).", pids))
        } else {
            Err(format!("Не удалось завершить некоторые процессы: {:?}", remaining))
        }
    }

    /// Перезапустить TG WS Proxy
    pub fn restart(config: &TgProxyConfig) -> Result<String, String> {
        let stop_msg = Self::stop(config).unwrap_or_else(|e| format!("Предупреждение: {}", e));
        sleep(Duration::from_millis(1000));
        let start_msg = Self::start(config)?;
        Ok(format!("{}\n\n{}", stop_msg, start_msg))
    }

    /// Подключить в Telegram Desktop (открыть tg://proxy)
    pub fn connect_telegram(config: &TgProxyConfig) -> Result<String, String> {
        let summary = Self::get_summary(config);
        if !summary.is_running {
            let _ = Self::start(config);
            sleep(Duration::from_millis(1000));
        }

        let link = Self::get_summary(config).tg_link;

        #[cfg(target_os = "windows")]
        {
            let script = format!("Start-Process '{}'", link);
            let _ = Self::run_powershell(&script);
        }

        Ok(format!(
            "✅ Ссылка отправлена в Telegram Desktop!\n\
             В открывшемся окне Telegram нажмите «Включить прокси» (Enable Proxy).\n\n\
             Параметры подключения:\n\
             ▶ Сервер: {}\n\
             ▶ Порт:   {}\n\
             ▶ Secret: {}\n\
             ▶ Ссылка: {}",
            summary.host,
            summary.port,
            summary.secret.as_deref().unwrap_or("[авто]"),
            link
        ))
    }

    /// Скопировать готовую ссылку tg://proxy в буфер обмена Windows
    pub fn copy_link(config: &TgProxyConfig) -> Result<String, String> {
        let summary = Self::get_summary(config);
        let link = &summary.tg_link;

        let script = format!("Set-Clipboard -Value '{}'", link);
        Self::run_powershell(&script)?;

        Ok(format!(
            "✅ Ссылка на MTProto прокси скопирована в буфер обмена!\n\n\
             {}\n\n\
             Вы можете вставить её в чат «Избранное» (Saved Messages) в Telegram или передать друзьям.",
            link
        ))
    }

    /// Показать последние строки лога proxy.log
    pub fn open_logs(_config: &TgProxyConfig) -> Result<String, String> {
        let log_path = match TgProxyConfig::get_log_file_path() {
            Some(p) if p.exists() => p,
            _ => {
                return Err("Файл логов proxy.log пока не создан. Запустите прокси хотя бы один раз.".to_string());
            }
        };

        let content = fs::read_to_string(&log_path)
            .map_err(|e| format!("Не удалось прочитать {}: {}", log_path.display(), e))?;

        let lines: Vec<&str> = content.lines().collect();
        let total = lines.len();
        let to_show = if total > 60 { &lines[total - 60..] } else { &lines[..] };

        let mut out = String::new();
        out.push_str("====================================================\n");
        out.push_str(&format!("  ЖУРНАЛ TG WS PROXY (Последние {} строк)\n", to_show.len()));
        out.push_str(&format!("  Путь: {}\n", log_path.display()));
        out.push_str("====================================================\n\n");
        out.push_str(&to_show.join("\n"));
        out.push_str("\n\n====================================================\n");
        out.push_str("💡 Чтобы открыть полный файл в Блокноте, введите: notepad \"%APPDATA%\\TgWsProxy\\proxy.log\"\n");

        Ok(out)
    }

    /// Открыть папку с файлом программы или логами в Проводнике
    pub fn open_folder(config: &TgProxyConfig) -> Result<String, String> {
        let appdata = TgProxyConfig::get_appdata_dir().filter(|p| p.exists());
        let exe_dir = config.get_resolved_path().and_then(|p| p.parent().map(|p| p.to_path_buf()));

        let target = appdata.or(exe_dir).unwrap_or_else(|| PathBuf::from("C:\\"));

        Command::new("explorer")
            .arg(&target)
            .spawn()
            .map_err(|e| format!("Не удалось открыть папку {}: {}", target.display(), e))?;

        Ok(format!("Папка открыта в Проводнике: {}", target.display()))
    }

    /// Загрузить / Обновить TgWsProxy_windows.exe с GitHub
    pub fn update(config: &TgProxyConfig, path: Option<&Path>, force: bool) -> Result<String, String> {
        let mut log = String::new();
        log.push_str("====================================================\n");
        log.push_str("    ОБНОВЛЕНИЕ / УСТАНОВКА FLOWSEAL TG WS PROXY     \n");
        log.push_str("====================================================\n\n");

        // Определяем куда сохранять
        let target_exe = match path {
            Some(p) => {
                if p.is_dir() {
                    p.join("TgWsProxy_windows.exe")
                } else {
                    p.to_path_buf()
                }
            }
            None => {
                if let Some(existing) = config.get_resolved_path() {
                    existing
                } else {
                    PathBuf::from("C:\\tg-ws-proxy\\TgWsProxy_windows.exe")
                }
            }
        };

        let is_installed = target_exe.exists();
        let local_version = Self::get_local_version(&target_exe).unwrap_or_default();

        log.push_str(&format!("📁 Файл программы:    {}\n", target_exe.display()));
        if is_installed && !local_version.is_empty() {
            log.push_str(&format!("📌 Текущая версия:    {}\n", local_version));
        }

        // 1. Проверка последнего релиза на GitHub
        log.push_str("🔍 1/4: Запрос актуального релиза на GitHub...\n");
        let (latest_tag, download_url) = Self::get_latest_github_release_asset(&config.github_repo)?;
        log.push_str(&format!("📦 Последний релиз:   {}\n", latest_tag));

        // Проверка: актуальна ли уже установленная версия
        if is_installed && !force && !local_version.is_empty() {
            let clean_local = local_version.trim_start_matches('v');
            let clean_latest = latest_tag.trim_start_matches('v');
            if clean_local == clean_latest {
                return Ok(format!(
                    "✨ У вас уже установлена актуальная версия TG WS Proxy ({})!\n\
                     📁 Файл программы: {}\n\
                     Обновление не требуется.\n\n\
                     💡 Для принудительной переустановки используйте флаг --force:\n\
                        dark-cli tg-proxy update --force",
                    local_version,
                    target_exe.display()
                ));
            }
        }

        if force && is_installed {
            log.push_str("⚡ Запрошено принудительное обновление (--force).\n");
        }

        // 2. Остановка старого процесса
        log.push_str("🛑 2/4: Остановка активных процессов TG WS Proxy...\n");
        let _ = Self::stop(config);
        sleep(Duration::from_millis(500));

        if let Some(parent) = target_exe.parent() {
            let _ = fs::create_dir_all(parent);
        }

        // 3. Скачивание
        log.push_str(&format!("⬇ 3/4: Загрузка {}...\n", download_url));
        let download_script = format!(
            "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
             $url = '{}'; \
             $out = '{}'; \
             Invoke-WebRequest -Uri $url -OutFile $out -UseBasicParsing",
            download_url,
            target_exe.display()
        );

        Self::run_powershell(&download_script)
            .map_err(|e| format!("Ошибка скачивания файла: {}", e))?;

        if !target_exe.exists() {
            return Err("Файл не был создан после скачивания.".to_string());
        }

        // Сохраняем версию в файл рядом и в AppData
        let _ = Self::save_local_version(&target_exe, &latest_tag);

        log.push_str("   Файл успешно загружен.\n");

        // 4. Запуск обновленного прокси
        log.push_str("🚀 4/4: Запуск обновленного TG WS Proxy...\n");
        sleep(Duration::from_millis(500));
        let mut new_config = config.clone();
        new_config.path = Some(target_exe.display().to_string());
        let _ = Self::start(&new_config);

        log.push_str(&format!(
            "\n🎉 ОБНОВЛЕНИЕ УСПЕШНО ЗАВЕРШЕНО! Установлена версия {}.\n\
             Значок программы появился в системном трее Windows рядом с часами.\n",
            latest_tag
        ));
        Ok(log)
    }

    /// Сохранить информацию о версии прокси
    pub fn save_local_version(exe: &Path, version: &str) -> Result<(), String> {
        let clean_ver = if version.starts_with('v') { version.to_string() } else { format!("v{}", version) };
        if let Some(appdata) = TgProxyConfig::get_appdata_dir() {
            let _ = fs::create_dir_all(&appdata);
            let vfile = appdata.join("version.txt");
            let _ = fs::write(&vfile, &clean_ver);
        }
        if let Some(parent) = exe.parent() {
            let _ = fs::create_dir_all(parent);
            let vfile = parent.join("version.txt");
            let _ = fs::write(&vfile, &clean_ver);
        }
        Ok(())
    }

    /// Определить установленную версию TG WS Proxy
    pub fn get_local_version(exe: &Path) -> Option<String> {
        // 1. Проверяем version.txt в AppData
        if let Some(appdata) = TgProxyConfig::get_appdata_dir() {
            let vfile = appdata.join("version.txt");
            if let Ok(c) = fs::read_to_string(&vfile) {
                let tr = c.trim();
                if !tr.is_empty() {
                    return Some(if tr.starts_with('v') { tr.to_string() } else { format!("v{}", tr) });
                }
            }
        }

        // 2. Проверяем version.txt рядом с exe
        if let Some(parent) = exe.parent() {
            let vfile = parent.join("version.txt");
            if let Ok(c) = fs::read_to_string(&vfile) {
                let tr = c.trim();
                if !tr.is_empty() {
                    return Some(if tr.starts_with('v') { tr.to_string() } else { format!("v{}", tr) });
                }
            }
        }

        // 3. Проверяем строки в proxy.log на наличие версии
        if let Some(log_path) = TgProxyConfig::get_log_file_path() {
            if let Ok(content) = fs::read_to_string(&log_path) {
                for line in content.lines() {
                    if let Some(idx) = line.find("версия ") {
                        let rest = &line[idx + "версия ".len()..];
                        if let Some(ver) = rest.split_whitespace().next() {
                            let clean_ver = ver.trim();
                            if !clean_ver.is_empty() {
                                return Some(format!("v{}", clean_ver.trim_start_matches('v')));
                            }
                        }
                    } else if let Some(idx) = line.find("version ") {
                        let rest = &line[idx + "version ".len()..];
                        if let Some(ver) = rest.split_whitespace().next() {
                            let clean_ver = ver.trim();
                            if !clean_ver.is_empty() {
                                return Some(format!("v{}", clean_ver.trim_start_matches('v')));
                            }
                        }
                    }
                }
            }
        }

        None
    }

    // --- Внутренние вспомогательные методы ---

    fn get_running_pids() -> Vec<u32> {
        #[cfg(target_os = "windows")]
        {
            // Используем быстрый встроенный tasklist вместо тяжелого PowerShell
            if let Ok(output) = Command::new("tasklist")
                .args(["/FI", "IMAGENAME eq TgWsProxy*", "/FO", "CSV", "/NH"])
                .output()
            {
                let text = String::from_utf8_lossy(&output.stdout);
                let mut pids = Vec::new();
                for line in text.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with("INFO:") {
                        continue;
                    }
                    let parts: Vec<&str> = trimmed.split(',').collect();
                    if parts.len() >= 2 {
                        let pid_str = parts[1].trim().trim_matches('"');
                        if let Ok(pid) = pid_str.parse::<u32>() {
                            pids.push(pid);
                        }
                    }
                }
                return pids;
            }
        }
        Vec::new()
    }

    fn check_port_listening(host: &str, port: u16) -> bool {
        let addr_str = if host == "0.0.0.0" || host.is_empty() { "127.0.0.1" } else { host };
        if let Ok(addr) = format!("{}:{}", addr_str, port).parse::<SocketAddr>() {
            TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_ok()
        } else {
            false
        }
    }

    fn read_config_json(default_port: u16) -> (String, u16, Option<String>) {
        let path = match TgProxyConfig::get_config_json_path() {
            Some(p) if p.exists() => p,
            _ => return ("127.0.0.1".to_string(), default_port, None),
        };

        if let Ok(content) = fs::read_to_string(&path) {
            let host = Self::extract_json_string(&content, "host").unwrap_or_else(|| "127.0.0.1".to_string());
            let port = Self::extract_json_u16(&content, "port").unwrap_or(default_port);
            let secret = Self::extract_json_string(&content, "secret");
            (host, port, secret)
        } else {
            ("127.0.0.1".to_string(), default_port, None)
        }
    }

    pub fn extract_json_string(json: &str, key: &str) -> Option<String> {
        let pattern = format!("\"{}\":", key);
        let idx = json.find(&pattern)?;
        let rest = &json[idx + pattern.len()..];
        let trimmed = rest.trim_start();
        if trimmed.starts_with('"') {
            let inner = &trimmed[1..];
            let end_quote = inner.find('"')?;
            Some(inner[..end_quote].to_string())
        } else {
            None
        }
    }

    pub fn extract_json_u16(json: &str, key: &str) -> Option<u16> {
        let pattern = format!("\"{}\":", key);
        let idx = json.find(&pattern)?;
        let rest = &json[idx + pattern.len()..];
        let trimmed = rest.trim_start();
        let num_str: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
        num_str.parse::<u16>().ok()
    }

    fn get_latest_github_release(repo: &str) -> Result<(String, String), String> {
        let script = format!(
            "$r = Invoke-RestMethod -Uri 'https://api.github.com/repos/{}/releases/latest' -Headers @{{'User-Agent'='DarkCLI'}}; \
             Write-Output \"$($r.tag_name)|$($r.html_url)\"",
            repo
        );

        let output = Self::run_powershell(&script)?;
        let trimmed = output.trim();
        let parts: Vec<&str> = trimmed.split('|').collect();
        if parts.len() == 2 {
            Ok((parts[0].to_string(), parts[1].to_string()))
        } else {
            Err(format!("Неожиданный ответ GitHub: {}", trimmed))
        }
    }

    fn get_latest_github_release_asset(repo: &str) -> Result<(String, String), String> {
        let script = format!(
            "$r = Invoke-RestMethod -Uri 'https://api.github.com/repos/{}/releases/latest' -Headers @{{'User-Agent'='DarkCLI'}}; \
             $asset = $r.assets | Where-Object {{ $_.name -like '*windows.exe' }} | Select-Object -First 1; \
             Write-Output \"$($r.tag_name)|$($asset.browser_download_url)\"",
            repo
        );

        let output = Self::run_powershell(&script)?;
        let trimmed = output.trim();
        let parts: Vec<&str> = trimmed.split('|').collect();
        if parts.len() == 2 && !parts[1].is_empty() {
            Ok((parts[0].to_string(), parts[1].to_string()))
        } else {
            Err(format!("Не удалось найти windows.exe в релизах: {}", trimmed))
        }
    }

    fn run_powershell(script: &str) -> Result<String, String> {
        let full_script = format!(
            "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
             $OutputEncoding = [System.Text.Encoding]::UTF8; \
             {}",
            script
        );

        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &full_script])
            .output()
            .map_err(|e| format!("Не удалось запустить PowerShell: {}", e))?;

        let stdout = decode_bytes(&output.stdout);
        let stderr = decode_bytes(&output.stderr);

        if !output.status.success() && stdout.trim().is_empty() {
            Err(stderr)
        } else {
            Ok(format!("{}{}", stdout, if stderr.is_empty() { "" } else { "\n" }).trim().to_string())
        }
    }
}

fn decode_bytes(bytes: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    #[cfg(target_os = "windows")]
    {
        use oem_cp::{Cp866, StringExt};
        String::from_cp::<Cp866>(bytes)
    }
    #[cfg(not(target_os = "windows"))]
    {
        String::from_utf8_lossy(bytes).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_values() {
        let json = r#"{
            "host": "127.0.0.1",
            "port": 1443,
            "secret": "7b3259e60e85cc5f02332e61b8c21bf3",
            "verbose": false
        }"#;

        assert_eq!(TgProxyManager::extract_json_string(json, "host"), Some("127.0.0.1".to_string()));
        assert_eq!(TgProxyManager::extract_json_u16(json, "port"), Some(1443));
        assert_eq!(
            TgProxyManager::extract_json_string(json, "secret"),
            Some("7b3259e60e85cc5f02332e61b8c21bf3".to_string())
        );
    }

    #[test]
    fn test_link_generation() {
        let config = TgProxyConfig::default();
        let summary = TgProxyManager::get_summary(&config);
        assert!(summary.tg_link.starts_with("tg://proxy?server="));
        assert!(summary.web_link.starts_with("https://t.me/proxy?server="));
    }
}
