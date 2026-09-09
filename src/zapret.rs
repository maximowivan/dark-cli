use crate::config::ZapretConfig;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

pub struct ZapretManager;

impl ZapretManager {
    /// Получить подробный статус работы Zapret
    pub fn get_status(config: &ZapretConfig) -> String {
        let mut out = String::new();
        out.push_str("====================================================\n");
        out.push_str("          СТАТУС FLOWSEAL ZAPRET                    \n");
        out.push_str("====================================================\n\n");

        let resolved_path = config.get_resolved_path();
        match resolved_path {
            Some(ref path) => {
                out.push_str("📌 Статус установки:  🟢 Установлен на этом компьютере\n");
                out.push_str(&format!("📁 Папка программы:   {}\n", path.display()));
            }
            None => {
                out.push_str("📌 Статус установки:  🔴 НЕ УСТАНОВЛЕН НА ЭТОМ ПК\n");
                out.push_str("📁 Папка программы:   [Не найдена]\n");
            }
        }

        // 1. Проверка службы Windows
        let service_status = Self::check_service_status(&config.service_name);
        out.push_str(&format!("⚙  Служба Windows:    {}\n", service_status));

        // 2. Проверка процесса winws.exe
        let process_status = Self::check_winws_process();
        out.push_str(&format!("⚡ Процесс winws:     {}\n", process_status));

        // 3. Активная стратегия из реестра или папки
        let mut strategy = Self::get_active_strategy(&config.service_name);
        if strategy == "Не определена в реестре" {
            if let Some(ref dir) = resolved_path {
                if let Some(strat_path) = Self::find_strategy_bat(dir) {
                    if let Some(name) = strat_path.file_name().and_then(|n| n.to_str()) {
                        strategy = format!("{} (для прямого запуска)", name);
                    }
                }
            }
        }
        out.push_str(&format!("🎯 Активная стратегия: {}\n", strategy));

        // 4. Локальная версия
        let local_version = resolved_path
            .as_ref()
            .and_then(|p| Self::get_local_version(p))
            .unwrap_or_else(|| "Не установлена".to_string());
        out.push_str(&format!("📌 Локальная версия:  {}\n", local_version));

        if resolved_path.is_none() {
            out.push_str("\n💡 Zapret пока не установлен на этом компьютере.\n");
            out.push_str("   Для установки запустите команду 'dark-cli zapret install' или\n");
            out.push_str("   нажмите [u] в меню TUI. Программа предложит два варианта:\n");
            out.push_str("     [1] Установить в папку по умолчанию (C:\\zapret)\n");
            out.push_str("     [2] Выбрать свою папку\n");
        } else if service_status.contains("Не установлена") {
            out.push_str("\n💡 Служба Windows не зарегистрирована. Zapret можно запускать напрямую (клавиша '1')\n");
            out.push_str("   или зарегистрировать как постоянную службу через Service.bat (клавиша 'm').\n");
        }

        // 5. Проверка последнего релиза на GitHub
        out.push_str("\n🔍 Проверка последней версии на GitHub...\n");
        match Self::get_latest_github_release(&config.github_repo) {
            Ok((tag, url)) => {
                out.push_str(&format!("🌐 Последний релиз:   v{}\n", tag));
                out.push_str(&format!("🔗 Ссылка:            {}\n", url));

                if local_version.trim_start_matches('v') == tag.trim_start_matches('v') {
                    out.push_str("✨ У вас установлена самая актуальная версия!\n");
                } else if local_version != "Не определена" {
                    out.push_str(&format!(
                        "⚡ Доступно обновление: {} -> {}! Запустите 'Обновить Zapret' (клавиша 'u').\n",
                        local_version, tag
                    ));
                }
            }
            Err(e) => {
                out.push_str(&format!("⚠ Не удалось проверить GitHub: {}\n", e));
            }
        }

        out.push_str("\n====================================================\n");
        out
    }

    /// Найти наиболее подходящий .bat файл стратегии в папке Zapret
    pub fn find_strategy_bat(zapret_dir: &Path) -> Option<PathBuf> {
        let preferred = [
            "general (ALT11).bat",
            "general (ALT10).bat",
            "general (ALT).bat",
            "general.bat",
            "general (ALT1).bat",
            "general (ALT2).bat",
            "general (ALT3).bat",
            "general (ALT4).bat",
            "general (ALT5).bat",
            "general (ALT6).bat",
            "general (ALT7).bat",
            "general (ALT8).bat",
            "general (ALT9).bat",
        ];
        for name in &preferred {
            let candidate = zapret_dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }

        // Поиск любого другого general*.bat
        if let Ok(entries) = fs::read_dir(zapret_dir) {
            let mut generals = Vec::new();
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        let name_lower = name.to_lowercase();
                        if name_lower.ends_with(".bat") && name_lower.starts_with("general") {
                            generals.push(path);
                        }
                    }
                }
            }
            generals.sort();
            if let Some(first) = generals.into_iter().next() {
                return Some(first);
            }
        }

        // Поиск любого .bat кроме service.bat
        if let Ok(entries) = fs::read_dir(zapret_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        let name_lower = name.to_lowercase();
                        if name_lower.ends_with(".bat") && !name_lower.starts_with("service") {
                            return Some(path);
                        }
                    }
                }
            }
        }

        None
    }

    /// Запустить Zapret (службу Windows или автономный процесс winws.exe)
    pub fn start(config: &ZapretConfig) -> Result<String, String> {
        let resolved_path = config.get_resolved_path().ok_or_else(|| {
            "Папка Zapret не найдена на этом ПК! Нажмите [u] ('Обновить / Установить Zapret') или [i] для установки.".to_string()
        })?;

        let service = &config.service_name;

        // 1. Проверяем, не запущен ли уже winws.exe
        let check_script = "$proc = Get-Process winws -ErrorAction SilentlyContinue; if ($proc) { $proc.Id -join ', ' }";
        if let Ok(pid_out) = Self::run_powershell_script(check_script) {
            let pids = pid_out.trim();
            if !pids.is_empty() {
                return Ok(format!(
                    "⚡ Zapret (winws.exe) уже запущен и работает в фоновом режиме (PID: {}).",
                    pids
                ));
            }
        }

        // 2. Проверяем наличие установленной службы Windows
        let check_service_script = format!(
            "$s = Get-Service -Name '{}' -ErrorAction SilentlyContinue; \
             if ($s) {{ $s.Status.ToString() }} else {{ 'NOT_INSTALLED' }}",
            service
        );

        let service_state = Self::run_powershell_script(&check_service_script)
            .unwrap_or_else(|_| "NOT_INSTALLED".to_string());
        let service_state = service_state.trim();

        if service_state == "Running" {
            return Ok(format!("Служба Windows '{}' уже запущена.", service));
        } else if service_state == "Stopped" || service_state == "Paused" {
            // Служба установлена, запускаем ее
            let start_service_script = format!(
                "Start-Process cmd -ArgumentList '/c net start {}' -Verb RunAs -Wait; \
                 $s = Get-Service -Name '{}' -ErrorAction SilentlyContinue; \
                 if ($s.Status -eq 'Running') {{ 'OK' }} else {{ 'FAIL' }}",
                service, service
            );
            let res = Self::run_powershell_script(&start_service_script)?;
            if res.trim() == "OK" {
                return Ok(format!("🟢 Служба Windows '{}' успешно запущена!", service));
            } else {
                return Err(format!(
                    "Не удалось запустить службу '{}'. Проверьте системный журнал или откройте Service.bat (клавиша 'm').",
                    service
                ));
            }
        }

        // 3. Служба Windows НЕ установлена -> запускаем автономную стратегию (general*.bat)
        let strategy_bat = Self::find_strategy_bat(&resolved_path).ok_or_else(|| {
            format!(
                "Служба Windows '{}' не установлена, и в папке '{}' не найден подходящий файл стратегии (general*.bat).\n\
                 Откройте Service.bat (клавиша 'm') для настройки или обновите Zapret (клавиша 'u').",
                service, resolved_path.display()
            )
        })?;

        let bat_name = strategy_bat
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "general.bat".to_string());

        let start_bat_script = format!(
            "Start-Process cmd.exe -ArgumentList '/c cd /d \"{}\" && \"{}\"' -Verb RunAs -WindowStyle Minimized; \
             Start-Sleep -Seconds 2; \
             $proc = Get-Process winws -ErrorAction SilentlyContinue; \
             if ($proc) {{ $proc.Id -join ', ' }} else {{ 'FAIL' }}",
            resolved_path.display(),
            strategy_bat.display()
        );

        let res = Self::run_powershell_script(&start_bat_script)?;
        let trimmed = res.trim();

        if trimmed != "FAIL" && !trimmed.is_empty() {
            Ok(format!(
                "🟢 Zapret успешно запущен в фоновом режиме!\n\
                 🎯 Стратегия запуска: {}\n\
                 ⚡ Процесс winws.exe активен (PID: {})\n\n\
                 💡 Примечание: Windows-служба '{}' пока не зарегистрирована в автозагрузке.\n\
                    Если хотите, чтобы Zapret запускался сам при включении ПК, откройте\n\
                    'Service.bat (Менеджер)' (клавиша 'm') и выберите пункт '1. Install Service'.",
                bat_name, trimmed, service
            ))
        } else {
            Err(format!(
                "Не удалось запустить Zapret через '{}'.\n\
                 Возможно, запрос прав администратора (UAC) был отклонен или антивирус заблокировал WinDivert.",
                bat_name
            ))
        }
    }

    /// Остановить службу Zapret и завершить процесс winws.exe
    pub fn stop(config: &ZapretConfig) -> Result<String, String> {
        let service = &config.service_name;
        let script = format!(
            "$s = Get-Service -Name '{service}' -ErrorAction SilentlyContinue; \
             if ($s -and $s.Status -eq 'Running') {{ \
                 Start-Process cmd -ArgumentList '/c net stop {service} & taskkill /F /IM winws.exe & net stop WinDivert' -Verb RunAs -Wait; \
             }} else {{ \
                 Start-Process cmd -ArgumentList '/c taskkill /F /IM winws.exe & net stop WinDivert' -Verb RunAs -Wait; \
             }} \
             $proc = Get-Process winws -ErrorAction SilentlyContinue; \
             if ($proc) {{ taskkill /F /IM winws.exe | Out-Null }}; \
             $procAfter = Get-Process winws -ErrorAction SilentlyContinue; \
             if ($procAfter) {{ \
                 Write-Output 'WARNING_STILL_RUNNING' \
             }} else {{ \
                 Write-Output 'STOPPED_SUCCESS' \
             }}"
        );

        let out = Self::run_powershell_script(&script)?;
        let trimmed = out.trim();
        if trimmed.contains("STOPPED_SUCCESS") {
            Ok("🛑 Zapret успешно остановлен (все процессы winws.exe и службы завершены).".to_string())
        } else if trimmed.contains("WARNING_STILL_RUNNING") {
            Err("Не удалось завершить процесс winws.exe. Проверьте права администратора.".to_string())
        } else {
            Ok(format!("Служба {} и процесс winws.exe остановлены.", service))
        }
    }

    /// Перезапустить службу Zapret
    pub fn restart(config: &ZapretConfig) -> Result<String, String> {
        let mut report = String::new();
        report.push_str("1. Остановка текущей службы и процессов...\n");
        let stop_res = Self::stop(config)?;
        report.push_str(&stop_res);
        report.push('\n');

        sleep(Duration::from_millis(1500));

        report.push_str("\n2. Запуск Zapret...\n");
        let start_res = Self::start(config)?;
        report.push_str(&start_res);

        Ok(report)
    }

    /// Получить краткую сводку об установке (установлен ли, путь, версия)
    pub fn get_install_summary(config: &ZapretConfig) -> (bool, Option<PathBuf>, Option<String>) {
        let path = config.get_resolved_path();
        let is_installed = path.is_some();
        let version = path.as_ref().and_then(|p| Self::get_local_version(p));
        (is_installed, path, version)
    }

    /// Запросить у пользователя выбор места установки (для CLI режима)
    pub fn prompt_install_path(default_path: &str) -> Result<PathBuf, String> {
        use std::io::{self, Write};

        println!("\n====================================================");
        println!("             УСТАНОВКА FLOWSEAL ZAPRET              ");
        println!("====================================================");
        println!("Куда вы хотите установить Zapret?");
        println!("  [1] В папку по умолчанию ({})", default_path);
        println!("  [2] Выбрать другую папку (указать свой путь)");
        print!("\nВыберите вариант [1/2] (по умолчанию 1): ");
        let _ = io::stdout().flush();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| format!("Ошибка ввода: {}", e))?;

        let choice = input.trim();
        if choice == "2" {
            print!("Введите полный путь к папке для установки Zapret: ");
            let _ = io::stdout().flush();

            let mut custom_path = String::new();
            io::stdin()
                .read_line(&mut custom_path)
                .map_err(|e| format!("Ошибка ввода: {}", e))?;

            let trimmed = custom_path.trim().trim_matches('"');
            if trimmed.is_empty() {
                println!("Путь не введен. Будет использована папка по умолчанию: {}", default_path);
                Ok(PathBuf::from(default_path))
            } else {
                let p = PathBuf::from(trimmed);
                println!("Выбрана папка: {}", p.display());
                Ok(p)
            }
        } else {
            println!("Выбрана папка по умолчанию: {}", default_path);
            Ok(PathBuf::from(default_path))
        }
    }

    /// Обновить папку Zapret или установить с нуля
    pub fn update(config: &ZapretConfig, target_path: Option<&Path>, force: bool) -> Result<String, String> {
        let zapret_dir = if let Some(tp) = target_path {
            tp.to_path_buf()
        } else {
            config.get_resolved_path().unwrap_or_else(|| PathBuf::from("C:\\zapret"))
        };

        let is_new_install = !zapret_dir.exists() || !zapret_dir.join("service.bat").exists();

        let mut log = String::new();
        if is_new_install {
            log.push_str(&format!("⚡ Выполняется чистая установка Zapret с GitHub в {}...\n", zapret_dir.display()));
            let _ = fs::create_dir_all(&zapret_dir);
        }

        log.push_str(&format!("📁 Целевая папка Zapret: {}\n", zapret_dir.display()));

        // 1. Получение информации о последнем релизе
        log.push_str("🔍 Запрос последнего релиза на GitHub...\n");
        let (latest_version, zip_url) = Self::get_latest_github_release_asset(&config.github_repo)?;
        log.push_str(&format!("📦 Последняя версия: {}\n", latest_version));
        log.push_str(&format!("🔗 URL архива: {}\n\n", zip_url));

        let local_version = Self::get_local_version(&zapret_dir).unwrap_or_default();
        if !is_new_install && !force && !local_version.is_empty() && local_version.trim_start_matches('v') == latest_version.trim_start_matches('v') {
            return Ok(format!(
                "✨ У вас уже установлена последняя версия ({})!\nОбновление не требуется. Для принудительной переустановки используйте флаг --force.",
                local_version
            ));
        }

        // 2. Остановка службы
        log.push_str("🛑 1/6: Остановка службы и процессов winws...\n");
        let _ = Self::stop(config);
        sleep(Duration::from_millis(1500));

        // 3. Резервное копирование пользовательских файлов списков (*-user.txt)
        log.push_str("💾 2/6: Сохранение пользовательских списков и настроек...\n");
        let backup_dir = zapret_dir.join("_dark_cli_user_backup");
        let _ = fs::create_dir_all(&backup_dir);

        let lists_dir = zapret_dir.join("lists");
        let mut backed_up_files = Vec::new();
        if lists_dir.exists() {
            if let Ok(entries) = fs::read_dir(&lists_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.contains("-user.txt") {
                            let dest = backup_dir.join(name);
                            if fs::copy(&path, &dest).is_ok() {
                                backed_up_files.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }
        log.push_str(&format!("   Сохранено пользовательских файлов: {}\n", backed_up_files.len()));

        // 4. Скачивание архива релиза
        log.push_str("⬇ 3/6: Скачивание релиза с GitHub...\n");
        let temp_dir = std::env::temp_dir();
        let zip_path = temp_dir.join("zapret_latest_release.zip");
        let extract_dir = temp_dir.join("zapret_latest_extracted");

        let download_script = format!(
            "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
             Invoke-WebRequest -Uri '{}' -OutFile '{}' -UseBasicParsing",
            zip_url,
            zip_path.display()
        );
        Self::run_powershell_script(&download_script)
            .map_err(|e| format!("Ошибка скачивания архива: {}", e))?;

        log.push_str("   Архив успешно загружен.\n");

        // 5. Распаковка архива
        log.push_str("📂 4/6: Распаковка архива...\n");
        let extract_script = format!(
            "if (Test-Path '{}') {{ Remove-Item -Recurse -Force '{}' }}; \
             Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
            extract_dir.display(),
            extract_dir.display(),
            zip_path.display(),
            extract_dir.display()
        );
        Self::run_powershell_script(&extract_script)
            .map_err(|e| format!("Ошибка распаковки архива: {}", e))?;

        // Некоторые архивы содержат внутри одну корневую папку zapret-discord-youtube-x.x.x
        let mut source_folder = extract_dir.clone();
        if let Ok(entries) = fs::read_dir(&extract_dir) {
            let items: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
            if items.len() == 1 && items[0].is_dir() {
                source_folder = items[0].clone();
            }
        }

        // 6. Копирование обновленных файлов в папку Zapret
        log.push_str("🔄 5/6: Обновление файлов в целевой папке...\n");
        let copy_script = format!(
            "Copy-Item -Path '{}\\*' -Destination '{}' -Recurse -Force",
            source_folder.display(),
            zapret_dir.display()
        );
        Self::run_powershell_script(&copy_script)
            .map_err(|e| format!("Ошибка копирования файлов: {}", e))?;

        // 7. Восстановление сохраненных пользовательских списков
        if !backed_up_files.is_empty() {
            log.push_str("📝 Восстановление сохраненных списков пользователей...\n");
            for name in &backed_up_files {
                let src = backup_dir.join(name);
                let dst = lists_dir.join(name);
                let _ = fs::copy(&src, &dst);
            }
        }
        let _ = fs::remove_dir_all(&backup_dir);
        let _ = fs::remove_file(&zip_path);
        let _ = fs::remove_dir_all(&extract_dir);

        // 8. Перезапуск службы
        log.push_str("🚀 6/6: Запуск обновленной службы Zapret...\n");
        let _ = Self::start(config);

        log.push_str(&format!(
            "\n🎉 ОБНОВЛЕНИЕ УСПЕШНО ЗАВЕРШЕНО! Установлена версия {}.\n",
            latest_version
        ));
        Ok(log)
    }

    /// Открыть папку Zapret в Проводнике
    pub fn open_folder(config: &ZapretConfig) -> Result<String, String> {
        let path = config
            .get_resolved_path()
            .ok_or_else(|| "Папка Zapret не найдена на этом ПК! Нажмите [u] ('Обновить / Установить Zapret') для автоматической установки.".to_string())?;

        Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Ошибка открытия Проводника: {}", e))?;

        Ok(format!("Открыта папка: {}", path.display()))
    }

    /// Запустить оригинальный service.bat с правами администратора
    pub fn run_manager(config: &ZapretConfig) -> Result<String, String> {
        let path = config
            .get_resolved_path()
            .ok_or_else(|| "Папка Zapret не найдена на этом ПК! Нажмите [u] ('Обновить / Установить Zapret') для автоматической установки.".to_string())?;

        let service_bat = path.join("service.bat");
        if !service_bat.exists() {
            return Err(format!("Файл service.bat не найден по пути {}", service_bat.display()));
        }

        let script = format!(
            "Start-Process cmd.exe -ArgumentList '/c cd /d \"{}\" && \"{}\"' -Verb RunAs",
            path.display(),
            service_bat.display()
        );

        Self::run_powershell_script(&script)?;
        Ok("Запущен оригинальный менеджер service.bat с правами администратора.".to_string())
    }

    // --- Вспомогательные методы ---

    fn check_service_status(service_name: &str) -> String {
        let output = Command::new("sc")
            .args(["query", service_name])
            .output();

        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            if text.contains("RUNNING") {
                return "🟢 Работает (RUNNING)".to_string();
            } else if text.contains("STOPPED") {
                return "🔴 Остановлена (STOPPED)".to_string();
            } else if text.contains("1060") || text.contains("does not exist") {
                return "⚪ Не установлена как служба".to_string();
            }
        }
        "Неизвестно".to_string()
    }

    fn check_winws_process() -> String {
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Get-Process winws -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id",
            ])
            .output();

        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !text.is_empty() {
                return format!("🟢 Активен (PID: {})", text.replace("\r\n", ", "));
            }
        }
        "🔴 Не запущен".to_string()
    }

    fn get_active_strategy(service_name: &str) -> String {
        let key = format!("HKLM\\SYSTEM\\CurrentControlSet\\Services\\{}", service_name);
        let output = Command::new("reg")
            .args(["query", &key, "/v", "zapret-discord-youtube"])
            .output();

        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                if line.contains("zapret-discord-youtube") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        return parts[2..].join(" ");
                    }
                }
            }
        }
        "Не определена в реестре".to_string()
    }

    fn get_local_version(zapret_dir: &Path) -> Option<String> {
        let service_bat = zapret_dir.join("service.bat");
        if service_bat.exists() {
            if let Ok(content) = fs::read_to_string(&service_bat) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("set \"LOCAL_VERSION=") {
                        let val = trimmed
                            .trim_start_matches("set \"LOCAL_VERSION=")
                            .trim_end_matches('"');
                        return Some(val.to_string());
                    }
                }
            }
        }
        None
    }

    fn get_latest_github_release(repo: &str) -> Result<(String, String), String> {
        let script = format!(
            "$r = Invoke-RestMethod -Uri 'https://api.github.com/repos/{}/releases/latest' -Headers @{{'User-Agent'='DarkCLI'}}; \
             Write-Output \"$($r.tag_name)|$($r.html_url)\"",
            repo
        );

        let output = Self::run_powershell_script(&script)?;
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
             $asset = $r.assets | Where-Object {{ $_.name -like '*.zip' }} | Select-Object -First 1; \
             Write-Output \"$($r.tag_name)|$($asset.browser_download_url)\"",
            repo
        );

        let output = Self::run_powershell_script(&script)?;
        let trimmed = output.trim();
        let parts: Vec<&str> = trimmed.split('|').collect();
        if parts.len() == 2 && !parts[1].is_empty() {
            Ok((parts[0].to_string(), parts[1].to_string()))
        } else {
            Err(format!("Не удалось найти zip-архив релиза: {}", trimmed))
        }
    }

    fn run_powershell_script(script: &str) -> Result<String, String> {
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
    use std::fs::File;

    #[test]
    fn test_find_strategy_bat_prefers_alt11() {
        let temp_dir = std::env::temp_dir().join(format!("zapret_strat_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let _ = File::create(temp_dir.join("general (ALT).bat"));
        let _ = File::create(temp_dir.join("general (ALT11).bat"));
        let _ = File::create(temp_dir.join("general.bat"));

        let found = ZapretManager::find_strategy_bat(&temp_dir);
        assert!(found.is_some());
        assert_eq!(
            found.unwrap().file_name().unwrap().to_string_lossy(),
            "general (ALT11).bat"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_find_strategy_bat_fallback() {
        let temp_dir = std::env::temp_dir().join(format!("zapret_fallback_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let _ = File::create(temp_dir.join("general_custom_rule.bat"));
        let _ = File::create(temp_dir.join("service.bat"));

        let found = ZapretManager::find_strategy_bat(&temp_dir);
        assert!(found.is_some());
        assert_eq!(
            found.unwrap().file_name().unwrap().to_string_lossy(),
            "general_custom_rule.bat"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }
}

