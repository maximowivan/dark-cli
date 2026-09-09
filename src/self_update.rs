use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

pub const DEFAULT_REPO: &str = "maximowivan/dark-cli";

#[derive(Debug, Clone)]
pub struct ReleaseAssetInfo {
    pub tag_name: String,
    pub version: String,
    pub download_url: String,
    pub asset_name: String,
    #[allow(dead_code)]
    pub release_notes: String,
}

pub struct SelfUpdateManager;

impl SelfUpdateManager {
    /// Очистка старых резервных файлов .old и незавершенных .new после предыдущих обновлений
    pub fn cleanup_old_binary() {
        if let Ok(current_exe) = std::env::current_exe() {
            if let Some(parent) = current_exe.parent() {
                let old_file = parent.join("dark-cli.exe.old");
                let new_file = parent.join("dark-cli.exe.new");
                let _ = fs::remove_file(old_file);
                let _ = fs::remove_file(new_file);

                // Также проверяем имя самого исполняемого файла с суффиксом .old
                let exe_old = current_exe.with_extension("exe.old");
                let _ = fs::remove_file(exe_old);
            }
        }
    }

    /// Проверка, запущен ли бинарник в dev-окружении (например, cargo run / target/debug)
    pub fn is_dev_environment() -> bool {
        if let Ok(current_exe) = std::env::current_exe() {
            let path_str = current_exe.to_string_lossy().to_lowercase();
            if path_str.contains("target\\debug") || path_str.contains("target/debug") {
                return true;
            }
        }
        false
    }

    /// Парсинг версии (например "v1.2.3" -> (1, 2, 3))
    pub fn parse_version(v: &str) -> (u32, u32, u32) {
        let clean = v.trim().trim_start_matches('v').trim_start_matches('V');
        let parts: Vec<&str> = clean.split(|c| c == '.' || c == '-').collect();

        let major = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
        let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let patch = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

        (major, minor, patch)
    }

    /// Проверка, новее ли удаленная версия текущей локальной
    pub fn is_newer_version(remote: &str, current: &str) -> bool {
        let (r_maj, r_min, r_pat) = Self::parse_version(remote);
        let (c_maj, c_min, c_pat) = Self::parse_version(current);

        (r_maj, r_min, r_pat) > (c_maj, c_min, c_pat)
    }

    /// Проверка наличия свежего релиза на GitHub с быстрым таймаутом (2-3 сек)
    pub fn check_for_update(repo: &str) -> Result<Option<ReleaseAssetInfo>, String> {
        let current_version = env!("CARGO_PKG_VERSION");
        let api_url = format!("https://api.github.com/repos/{}/releases/latest", repo);

        // Попытка 1: Сверхбыстрый curl.exe (встроен в Windows 10/11)
        let json_text = match Self::fetch_url_curl(&api_url, 3) {
            Ok(out) => out,
            Err(_) => {
                // Попытка 2: Fallback через PowerShell Invoke-RestMethod
                Self::fetch_url_powershell(&api_url, 3)?
            }
        };

        if json_text.trim().is_empty() {
            return Ok(None);
        }

        let parsed: serde_json::Value = serde_json::from_str(&json_text)
            .map_err(|e| format!("Не удалось разобрать ответ GitHub: {}", e))?;

        let tag_name = match parsed.get("tag_name").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => {
                // Если на GitHub еще нет релизов (404 Not Found) - считаем версию актуальной
                return Ok(None);
            }
        };

        let clean_remote_version = tag_name.trim_start_matches('v').trim_start_matches('V');
        if !Self::is_newer_version(clean_remote_version, current_version) {
            return Ok(None);
        }

        let release_notes = parsed["body"].as_str().unwrap_or("").to_string();

        // Поиск подходящего исполняемого файла в ассетах
        let mut download_url = None;
        let mut asset_name = None;

        if let Some(assets) = parsed["assets"].as_array() {
            // Приоритет 1: direct dark-cli.exe
            for a in assets {
                let name = a["name"].as_str().unwrap_or("");
                if name.eq_ignore_ascii_case("dark-cli.exe") {
                    download_url = a["browser_download_url"].as_str().map(String::from);
                    asset_name = Some(name.to_string());
                    break;
                }
            }

            // Приоритет 2: любой .exe файл
            if download_url.is_none() {
                for a in assets {
                    let name = a["name"].as_str().unwrap_or("");
                    if name.ends_with(".exe") {
                        download_url = a["browser_download_url"].as_str().map(String::from);
                        asset_name = Some(name.to_string());
                        break;
                    }
                }
            }
        }

        let url = download_url.ok_or_else(|| {
            format!("В релизе {} не найден исполняемый файл dark-cli.exe", tag_name)
        })?;

        Ok(Some(ReleaseAssetInfo {
            tag_name: tag_name.clone(),
            version: clean_remote_version.to_string(),
            download_url: url,
            asset_name: asset_name.unwrap_or_else(|| "dark-cli.exe".to_string()),
            release_notes,
        }))
    }

    /// Загрузка через curl.exe с таймаутом
    fn fetch_url_curl(url: &str, timeout_sec: u32) -> Result<String, String> {
        let output = Command::new("curl.exe")
            .args([
                "-sSL",
                "--connect-timeout",
                &timeout_sec.to_string(),
                "-m",
                &(timeout_sec * 2).to_string(),
                "-H",
                "User-Agent: DarkCLI",
                "-H",
                "Accept: application/vnd.github+json",
                url,
            ])
            .output()
            .map_err(|e| format!("Не удалось запустить curl: {}", e))?;

        if !output.status.success() {
            return Err(format!("curl завершился с ошибкой: {:?}", output.status));
        }

        String::from_utf8(output.stdout)
            .map_err(|e| format!("Некорректный UTF-8 в выводе curl: {}", e))
    }

    /// Загрузка через PowerShell с таймаутом
    fn fetch_url_powershell(url: &str, timeout_sec: u32) -> Result<String, String> {
        let ps_cmd = format!(
            "$ProgressPreference = 'SilentlyContinue'; \
             [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
             try {{ \
                 (Invoke-WebRequest -Uri '{}' -Headers @{{'User-Agent'='DarkCLI'; 'Accept'='application/vnd.github+json'}} -TimeoutSec {} -UseBasicParsing).Content \
             }} catch {{ \
                 exit 1 \
             }}",
            url, timeout_sec
        );

        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_cmd])
            .output()
            .map_err(|e| format!("Не удалось запустить powershell: {}", e))?;

        if !output.status.success() {
            return Err("Ошибка запроса через PowerShell".to_string());
        }

        String::from_utf8(output.stdout)
            .map_err(|e| format!("Некорректный UTF-8 в выводе PowerShell: {}", e))
    }

    /// Загрузка файла по URL
    fn download_file(url: &str, destination: &Path) -> Result<(), String> {
        let dest_str = destination.display().to_string();

        // Попытка 1: curl.exe с поддержкой редиректов (-L)
        let curl_res = Command::new("curl.exe")
            .args([
                "-sSL",
                "--connect-timeout",
                "10",
                "-m",
                "120",
                "-o",
                &dest_str,
                url,
            ])
            .status();

        if let Ok(st) = curl_res {
            if st.success() && destination.exists() {
                if let Ok(meta) = fs::metadata(destination) {
                    if meta.len() > 100 * 1024 {
                        return Ok(());
                    }
                }
            }
        }

        // Попытка 2: PowerShell Invoke-WebRequest
        let ps_cmd = format!(
            "$ProgressPreference = 'SilentlyContinue'; \
             [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
             Invoke-WebRequest -Uri '{}' -OutFile '{}' -TimeoutSec 120",
            url, dest_str
        );

        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_cmd])
            .output()
            .map_err(|e| format!("Ошибка загрузки через PowerShell: {}", e))?;

        if !output.status.success() {
            return Err(format!("Загрузка не удалась: {}", String::from_utf8_lossy(&output.stderr)));
        }

        Ok(())
    }

    /// Проверка корректности PE исполняемого файла Windows (сигнатура 'MZ' и размер)
    pub fn validate_windows_executable(path: &Path) -> Result<(), String> {
        let meta = fs::metadata(path)
            .map_err(|e| format!("Файл не найден: {}", e))?;

        if meta.len() < 500 * 1024 {
            return Err(format!("Размер файла слишком мал ({} байт), возможно загрузка оборвалась", meta.len()));
        }

        let bytes = fs::read(path)
            .map_err(|e| format!("Не удалось прочитать файл: {}", e))?;

        if bytes.len() < 2 || bytes[0] != b'M' || bytes[1] != b'Z' {
            return Err("Файл не является корректным исполняемым файлом Windows (отсутствует сигнатура MZ)".to_string());
        }

        Ok(())
    }

    /// Применение обновления с безопасной ротацией на Windows
    pub fn apply_update(asset: &ReleaseAssetInfo) -> Result<(), String> {
        let current_exe = std::env::current_exe()
            .map_err(|e| format!("Не удалось определить путь к текущему процессу: {}", e))?;
        let exe_dir = current_exe.parent()
            .ok_or_else(|| "Не удалось определить каталог программы".to_string())?;

        let temp_exe = exe_dir.join("dark-cli.exe.new");
        let old_exe = exe_dir.join("dark-cli.exe.old");

        // 1. Очистка старых временных файлов
        let _ = fs::remove_file(&temp_exe);
        let _ = fs::remove_file(&old_exe);

        // 2. Загрузка во временный файл
        Self::download_file(&asset.download_url, &temp_exe)?;

        // 3. Проверка валидности исполняемого файла
        if let Err(e) = Self::validate_windows_executable(&temp_exe) {
            let _ = fs::remove_file(&temp_exe);
            return Err(format!("Проверка загруженного файла не прошла: {}", e));
        }

        // 4. Ротация: переименовываем работающий EXE в .old
        fs::rename(&current_exe, &old_exe)
            .map_err(|e| format!("Не удалось переименовать текущий файл: {}", e))?;

        // 5. Перемещаем скачанный файл на место текущего
        if let Err(e) = fs::rename(&temp_exe, &current_exe) {
            // В случае ошибки возвращаем старый файл обратно
            let _ = fs::rename(&old_exe, &current_exe);
            return Err(format!("Не удалось установить новый файл (откат выполнен): {}", e));
        }

        Ok(())
    }

    /// Перезапуск обновленного процесса
    pub fn restart_process() -> Result<(), String> {
        let current_exe = std::env::current_exe()
            .map_err(|e| format!("Не удалось получить путь exe: {}", e))?;

        let args: Vec<String> = std::env::args().skip(1).collect();

        Command::new(&current_exe)
            .args(&args)
            .spawn()
            .map_err(|e| format!("Не удалось запустить обновленный процесс: {}", e))?;

        std::process::exit(0);
    }

    /// Автоматическая проверка и обновление при старте программы
    /// Возвращает true, если программа была обновлена и перезапущена (текущий процесс завершается)
    pub fn auto_update_on_startup(repo: &str) -> bool {
        // Пропускаем автообновление в dev-режиме сборки (target/debug), чтобы не затирать бинарник сборщика
        if Self::is_dev_environment() {
            return false;
        }

        // Очищаем хвосты от прошлых обновлений
        Self::cleanup_old_binary();

        // Проверяем наличие обновлений (таймаут 2.5 сек)
        match Self::check_for_update(repo) {
            Ok(Some(asset)) => {
                println!("\n==========================================================");
                println!(" 🚀 Найдено обновление Dark-CLI: v{} -> v{}!", env!("CARGO_PKG_VERSION"), asset.version);
                println!(" ⬇ Загрузка релиза {} с GitHub...", asset.tag_name);
                println!("==========================================================\n");

                match Self::apply_update(&asset) {
                    Ok(_) => {
                        println!("✨ Обновление успешно установлено! Перезапуск Dark-CLI...");
                        std::thread::sleep(Duration::from_millis(500));
                        if let Err(e) = Self::restart_process() {
                            eprintln!("Не удалось автоматически перезапустить: {}. Запустите программу вручную.", e);
                        }
                        return true;
                    }
                    Err(e) => {
                        eprintln!("⚠ Не удалось применить автообновление: {}", e);
                        eprintln!("Запуск текущей версии Dark-CLI v{}...\n", env!("CARGO_PKG_VERSION"));
                    }
                }
            }
            Ok(None) => {
                // Актуальная версия, продолжаем нормальный запуск
            }
            Err(_) => {
                // Ошибка сети / таймаут / GitHub недоступен - продолжаем без задержки
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        assert_eq!(SelfUpdateManager::parse_version("1.0.0"), (1, 0, 0));
        assert_eq!(SelfUpdateManager::parse_version("v1.2.3"), (1, 2, 3));
        assert_eq!(SelfUpdateManager::parse_version("V2.10.4-rc1"), (2, 10, 4));
        assert_eq!(SelfUpdateManager::parse_version("invalid"), (0, 0, 0));
    }

    #[test]
    fn test_is_newer_version() {
        assert!(SelfUpdateManager::is_newer_version("1.0.1", "1.0.0"));
        assert!(SelfUpdateManager::is_newer_version("1.1.0", "1.0.9"));
        assert!(SelfUpdateManager::is_newer_version("2.0.0", "1.99.99"));
        assert!(SelfUpdateManager::is_newer_version("v1.0.1", "1.0.0"));

        assert!(!SelfUpdateManager::is_newer_version("1.0.0", "1.0.0"));
        assert!(!SelfUpdateManager::is_newer_version("1.0.0", "1.0.1"));
        assert!(!SelfUpdateManager::is_newer_version("0.9.9", "1.0.0"));
    }

    #[test]
    fn test_validate_executable_invalid() {
        let temp_file = std::env::temp_dir().join("test_invalid_bin.exe");
        let _ = fs::write(&temp_file, b"NOT_MZ_HEADER_JUST_TEXT");
        assert!(SelfUpdateManager::validate_windows_executable(&temp_file).is_err());
        let _ = fs::remove_file(temp_file);
    }
}
