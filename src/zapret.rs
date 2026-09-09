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
                out.push_str(&format!("📁 Папка программы:   {}\n", path.display()));
            }
            None => {
                out.push_str("📁 Папка программы:   [НЕ НАЙДЕНА] (укажите в config.toml)\n");
            }
        }

        // 1. Проверка службы Windows
        let service_status = Self::check_service_status(&config.service_name);
        out.push_str(&format!("⚙  Служба Windows:    {}\n", service_status));

        // 2. Проверка процесса winws.exe
        let process_status = Self::check_winws_process();
        out.push_str(&format!("⚡ Процесс winws:     {}\n", process_status));

        // 3. Активная стратегия из реестра
        let strategy = Self::get_active_strategy(&config.service_name);
        out.push_str(&format!("🎯 Активная стратегия: {}\n", strategy));

        // 4. Локальная версия
        let local_version = resolved_path
            .as_ref()
            .and_then(|p| Self::get_local_version(p))
            .unwrap_or_else(|| "Не определена".to_string());
        out.push_str(&format!("📌 Установлена версия: {}\n", local_version));

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

    /// Запустить службу Zapret
    pub fn start(config: &ZapretConfig) -> Result<String, String> {
        let service = &config.service_name;
        let script = format!(
            "$s = Get-Service -Name '{}' -ErrorAction SilentlyContinue; \
             if ($s.Status -eq 'Running') {{ Write-Output 'Служба {} уже запущена.' }} \
             else {{ Start-Process cmd -ArgumentList '/c net start {}' -Verb RunAs -Wait; \
                     $s2 = Get-Service -Name '{}' -ErrorAction SilentlyContinue; \
                     if ($s2.Status -eq 'Running') {{ Write-Output 'Служба {} успешно запущена!' }} \
                     else {{ Write-Output 'Не удалось запустить службу {}. Проверьте права администратора.' }} }}",
            service, service, service, service, service, service
        );

        Self::run_powershell_script(&script)
    }

    /// Остановить службу Zapret и завершить процесс winws.exe
    pub fn stop(config: &ZapretConfig) -> Result<String, String> {
        let service = &config.service_name;
        let script = format!(
            "Start-Process cmd -ArgumentList '/c net stop {} & taskkill /F /IM winws.exe & net stop WinDivert' -Verb RunAs -Wait; \
             $proc = Get-Process winws -ErrorAction SilentlyContinue; \
             if ($proc) {{ taskkill /F /IM winws.exe | Out-Null }}; \
             Write-Output 'Служба {} и процесс winws.exe остановлены.'",
            service, service
        );

        Self::run_powershell_script(&script)
    }

    /// Перезапустить службу Zapret
    pub fn restart(config: &ZapretConfig) -> Result<String, String> {
        let mut report = String::new();
        report.push_str("1. Остановка текущей службы и процессов...\n");
        let stop_res = Self::stop(config)?;
        report.push_str(&stop_res);
        report.push('\n');

        sleep(Duration::from_millis(1500));

        report.push_str("\n2. Запуск службы Zapret...\n");
        let start_res = Self::start(config)?;
        report.push_str(&start_res);

        Ok(report)
    }

    /// Обновить папку Zapret до последнего релиза на GitHub
    pub fn update(config: &ZapretConfig, force: bool) -> Result<String, String> {
        let zapret_dir = config
            .get_resolved_path()
            .ok_or_else(|| "Папка Zapret не найдена! Укажите корректный путь в config.toml.".to_string())?;

        let mut log = String::new();
        log.push_str(&format!("📁 Целевая папка Zapret: {}\n", zapret_dir.display()));

        // 1. Получение информации о последнем релизе
        log.push_str("🔍 Запрос последнего релиза на GitHub...\n");
        let (latest_version, zip_url) = Self::get_latest_github_release_asset(&config.github_repo)?;
        log.push_str(&format!("📦 Последняя версия: {}\n", latest_version));
        log.push_str(&format!("🔗 URL архива: {}\n\n", zip_url));

        let local_version = Self::get_local_version(&zapret_dir).unwrap_or_default();
        if !force && !local_version.is_empty() && local_version.trim_start_matches('v') == latest_version.trim_start_matches('v') {
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
            .ok_or_else(|| "Папка Zapret не найдена!".to_string())?;

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
            .ok_or_else(|| "Папка Zapret не найдена!".to_string())?;

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
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
            .map_err(|e| format!("Не удалось запустить PowerShell: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() && stdout.trim().is_empty() {
            Err(stderr)
        } else {
            Ok(format!("{}{}", stdout, if stderr.is_empty() { "" } else { "\n" }).trim().to_string())
        }
    }
}
