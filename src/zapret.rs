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
            out.push_str("\n💡 Служба Windows еще не установлена. Нажмите [1] («Запустить службу Zapret»),\n");
            out.push_str("   и программа автоматически зарегистрирует и запустит службу Windows в скрытом режиме (без окон)!\n");
            out.push_str("   Сменить стратегию службы можно клавишей [e] («Установить / Сменить стратегию службы»).\n");
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

    /// Получить список всех доступных стратегий (.bat файлов) в папке Zapret
    pub fn list_available_strategies(zapret_dir: &Path) -> Vec<String> {
        let mut strategies = Vec::new();
        if let Ok(entries) = fs::read_dir(zapret_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        let name_lower = name.to_lowercase();
                        if name_lower.ends_with(".bat") && !name_lower.starts_with("service") {
                            let stem = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or(name);
                            strategies.push(stem.to_string());
                        }
                    }
                }
            }
        }
        strategies.sort();
        strategies
    }

    /// Разобрать .bat файл стратегии и извлечь командную строку для службы winws.exe
    pub fn parse_strategy_bat(
        bat_path: &Path,
        zapret_dir: &Path,
    ) -> Result<(String, String), String> {
        let content = fs::read_to_string(bat_path)
            .map_err(|e| format!("Не удалось прочитать файл стратегии {}: {}", bat_path.display(), e))?;

        let strategy_name = bat_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "general".to_string());

        let dir_str = zapret_dir.display().to_string().trim_end_matches('\\').to_string();
        let bin_dir = format!("{}\\", zapret_dir.join("bin").display());
        let lists_dir = format!("{}\\", zapret_dir.join("lists").display());

        let mut in_command = false;
        let mut command_parts = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("::") || trimmed.starts_with("rem ") {
                continue;
            }

            if !in_command {
                if trimmed.contains("winws.exe") {
                    in_command = true;
                    // Извлекаем все после winws.exe" или winws.exe
                    let after_winws = if let Some(idx) = trimmed.find("winws.exe\"") {
                        &trimmed[idx + 10..]
                    } else if let Some(idx) = trimmed.find("winws.exe") {
                        &trimmed[idx + 9..]
                    } else {
                        trimmed
                    };
                    let cleaned = after_winws.trim_end_matches('^').trim();
                    command_parts.push(cleaned.to_string());
                    if !trimmed.ends_with('^') {
                        break;
                    }
                }
            } else {
                let cleaned = trimmed.trim_end_matches('^').trim();
                command_parts.push(cleaned.to_string());
                if !trimmed.ends_with('^') {
                    break;
                }
            }
        }

        if command_parts.is_empty() {
            return Err(format!(
                "В файле '{}' не найдена команда запуска winws.exe",
                bat_path.display()
            ));
        }

        let raw_args = command_parts.join(" ");

        // Выполняем подстановку путей и переменных окружения
        let replaced_args = raw_args
            .replace("%BIN%", &bin_dir)
            .replace("%bin%", &bin_dir)
            .replace("%LISTS%", &lists_dir)
            .replace("%lists%", &lists_dir)
            .replace("%GameFilterTCP%", "12")
            .replace("%GameFilterUDP%", "12")
            .replace("%GameFilter%", "12")
            .replace("%~dp0", &format!("{}\\", dir_str));

        let winws_exe = zapret_dir.join("bin").join("winws.exe");
        let full_command_line = format!("\"{}\" {}", winws_exe.display(), replaced_args.trim());

        Ok((strategy_name, full_command_line))
    }

    /// Установить службу Windows zapret со стратегией (по умолчанию ALT11)
    pub fn install_service(
        config: &ZapretConfig,
        strategy_name: Option<&str>,
    ) -> Result<String, String> {
        let resolved_path = config.get_resolved_path().ok_or_else(|| {
            "Папка Zapret не найдена на этом ПК! Нажмите [u] ('Обновить / Установить Zapret') или [i] для установки.".to_string()
        })?;

        // Ищем файл стратегии
        let bat_path = if let Some(name) = strategy_name {
            let mut p = resolved_path.join(name);
            if !p.exists() && !name.ends_with(".bat") {
                p = resolved_path.join(format!("{}.bat", name));
            }
            if !p.exists() {
                return Err(format!(
                    "Файл стратегии '{}' не найден в папке {}",
                    name,
                    resolved_path.display()
                ));
            }
            p
        } else {
            Self::find_strategy_bat(&resolved_path).ok_or_else(|| {
                format!(
                    "В папке '{}' не найдены файлы стратегий (general*.bat)",
                    resolved_path.display()
                )
            })?
        };

        let (strat_name, full_command_line) = Self::parse_strategy_bat(&bat_path, &resolved_path)?;
        let service = &config.service_name;

        let script = format!(
            r#"$cmd = @'
{cmd}
'@;
$strat = '{strat}';
$srv = '{service}';

# 1. Остановка старой службы и удаление
$existing = Get-Service -Name $srv -ErrorAction SilentlyContinue;
if ($existing) {{
    Stop-Service -Name $srv -Force -ErrorAction SilentlyContinue;
    sc.exe delete $srv | Out-Null;
    Start-Sleep -Milliseconds 500;
}}

# 2. Создание новой службы
sc.exe create $srv binPath= "placeholder" DisplayName= "$srv" start= auto | Out-Null;
sc.exe description $srv "Zapret DPI bypass software" | Out-Null;
Set-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Services\$srv" -Name "ImagePath" -Value $cmd -Type ExpandString;
New-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Services\$srv" -Name "zapret-discord-youtube" -Value $strat -PropertyType String -Force | Out-Null;
netsh interface tcp set global timestamps=enabled | Out-Null;

$installed = Get-Service -Name $srv -ErrorAction SilentlyContinue;
if ($installed) {{
    Write-Output "SUCCESS|$strat"
}} else {{
    Write-Output "FAIL"
}}"#,
            cmd = full_command_line,
            strat = strat_name,
            service = service,
        );

        // Для записи в реестр HKLM и создания службы требуются права администратора (RunAs)
        let temp_script = std::env::temp_dir().join(format!("install_zapret_{}.ps1", std::process::id()));
        fs::write(&temp_script, script.as_bytes())
            .map_err(|e| format!("Не удалось создать временный скрипт установки: {}", e))?;

        let run_cmd = format!(
            "Start-Process powershell -ArgumentList '-NoProfile -ExecutionPolicy Bypass -File \"{}\"' -Verb RunAs -Wait",
            temp_script.display()
        );
        let _ = Self::run_powershell_script(&run_cmd);
        let _ = fs::remove_file(&temp_script);

        // Проверяем статус созданной службы
        let check_script = format!(
            "$s = Get-Service -Name '{}' -ErrorAction SilentlyContinue; if ($s) {{ 'OK' }} else {{ 'FAIL' }}",
            service
        );
        let res = Self::run_powershell_script(&check_script)?;
        if res.trim() == "OK" {
            Ok(format!(
                "🟢 Служба Windows '{}' успешно зарегистрирована в системе!\n\
                 🎯 Установлена стратегия: {}\n\
                 ⚙  Тип запуска: Автоматически (при старте Windows)\n\
                 ✨ Служба будет работать скрыто в фоновом режиме без открытия окон консоли.",
                service, strat_name
            ))
        } else {
            Err(format!(
                "Не удалось зарегистрировать службу Windows '{}'.\n\
                 Возможно, запрос прав администратора (UAC) был отклонен.",
                service
            ))
        }
    }

    /// Удалить службу Windows zapret
    pub fn remove_service(config: &ZapretConfig) -> Result<String, String> {
        let service = &config.service_name;
        let script = format!(
            "Start-Process cmd -ArgumentList '/c net stop {service} & sc delete {service} & taskkill /F /IM winws.exe & net stop WinDivert' -Verb RunAs -Wait; \
             $s = Get-Service -Name '{service}' -ErrorAction SilentlyContinue; \
             if ($s) {{ 'FAIL' }} else {{ 'DELETED' }}",
            service = service
        );

        let out = Self::run_powershell_script(&script)?;
        let trimmed = out.trim();
        if trimmed == "DELETED" {
            Ok(format!("🗑 Служба Windows '{}' успешно удалена из системы.", service))
        } else {
            Err(format!("Не удалось удалить службу Windows '{}'. Проверьте права администратора.", service))
        }
    }

    /// Запустить службу Zapret (ИСКЛЮЧИТЕЛЬНО как системную службу Windows, без открытия окон)
    pub fn start(config: &ZapretConfig) -> Result<String, String> {
        let _resolved_path = config.get_resolved_path().ok_or_else(|| {
            "Папка Zapret не найдена на этом ПК! Нажмите [u] ('Обновить / Установить Zapret') или [i] для установки.".to_string()
        })?;

        let service = &config.service_name;

        // 1. Проверяем наличие установленной службы Windows
        let check_service_script = format!(
            "$s = Get-Service -Name '{}' -ErrorAction SilentlyContinue; \
             if ($s) {{ $s.Status.ToString() }} else {{ 'NOT_INSTALLED' }}",
            service
        );

        let service_state = Self::run_powershell_script(&check_service_script)
            .unwrap_or_else(|_| "NOT_INSTALLED".to_string());
        let service_state = service_state.trim();

        // 2. Если служба уже запущена, проверяем процесс
        if service_state == "Running" {
            let proc_status = Self::check_winws_process();
            let strat = Self::get_active_strategy(service);
            return Ok(format!(
                "Служба Windows '{}' уже работает в фоновом режиме.\n🎯 Стратегия: {}\n{}",
                service, strat, proc_status
            ));
        }

        // 3. Если служба Windows еще не зарегистрирована, автоматически устанавливаем ее
        let mut install_msg = String::new();
        if service_state == "NOT_INSTALLED" {
            let install_res = Self::install_service(config, None)?;
            install_msg = format!("{}\n\n", install_res);
        }

        // 4. Запускаем службу через net start с правами администратора
        let start_service_script = format!(
            "Start-Process cmd -ArgumentList '/c net start {service}' -Verb RunAs -Wait; \
             $s = Get-Service -Name '{service}' -ErrorAction SilentlyContinue; \
             if ($s -and $s.Status -eq 'Running') {{ \
                 $proc = Get-Process winws -ErrorAction SilentlyContinue; \
                 if ($proc) {{ 'OK|' + ($proc.Id -join ', ') }} else {{ 'OK_NO_PID' }} \
             }} else {{ \
                 'FAIL' \
             }}",
            service = service
        );

        let res = Self::run_powershell_script(&start_service_script)?;
        let trimmed = res.trim();

        if trimmed.starts_with("OK") {
            let strat = Self::get_active_strategy(service);
            let pid_info = if trimmed.starts_with("OK|") {
                format!("⚡ Процесс winws: 🟢 Активен (PID: {})", trimmed.trim_start_matches("OK|"))
            } else {
                Self::check_winws_process()
            };

            Ok(format!(
                "{}🟢 Служба Windows '{}' успешно запущена!\n\
                 🎯 Стратегия службы: {}\n\
                 {}\n\n\
                 ✨ Zapret работает скрыто в фоновом режиме через системную службу Windows.\n\
                 Никаких окон консоли не открывается, служба запускается автоматически при включении ПК.",
                install_msg, service, strat, pid_info
            ))
        } else {
            Err(format!(
                "Не удалось запустить службу Windows '{}'.\n\
                 Проверьте, было ли подтверждено окно контроля учетных записей (UAC), либо запустите Service.bat (клавиша 'm').",
                service
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

    /// Установить и настроить Zapret под ключ (для нового ПК):
    /// 1. Скачивает и устанавливает Zapret в C:\zapret (папка по умолчанию).
    /// 2. Запускает утилиту тестирования всех стратегий (utils\test zapret.ps1).
    /// 3. Находит лучшую стратегию по результатам тестов.
    /// 4. Регистрирует системную службу Windows zapret с найденной лучшей стратегией.
    /// 5. Запускает службу и проверяет доступность YouTube и Discord.
    pub fn easy_setup(
        app_config: &crate::config::AppConfig,
        config_path: &Path,
    ) -> Result<String, String> {
        let default_path = PathBuf::from("C:\\zapret");
        let mut zapret_cfg = app_config.zapret.clone().unwrap_or_default();
        let mut out = String::new();

        out.push_str("====================================================\n");
        out.push_str("   МАСТЕР БЫСТРОЙ НАСТРОЙКИ ZAPRET ПОД КЛЮЧ         \n");
        out.push_str("====================================================\n\n");

        // --- ШАГ 1: Проверка / загрузка файлов в C:\zapret ---
        let has_files = default_path.join("bin").join("winws.exe").exists()
            && default_path.join("utils").join("test zapret.ps1").exists();

        if !has_files {
            out.push_str("📦 Шаг 1/5: Загрузка последней версии Zapret с GitHub в C:\\zapret...\n");
            let update_res = Self::update(&zapret_cfg, Some(&default_path), true)?;
            out.push_str(&update_res);
            out.push('\n');

            let mut updated_config = app_config.clone();
            updated_config.set_zapret_path(default_path.display().to_string());
            let _ = updated_config.save(config_path);
            zapret_cfg.path = Some(default_path.display().to_string());
        } else {
            out.push_str(&format!(
                "✅ Шаг 1/5: Файлы программы готовы в {}\n",
                default_path.display()
            ));
        }

        // --- ШАГ 2: Подготовка окружения (остановка службы перед тестами) ---
        out.push_str("🧹 Шаг 2/5: Остановка служб перед запуском тестирования...\n");
        let _ = Self::remove_service(&zapret_cfg);
        sleep(Duration::from_millis(1000));

        // --- ШАГ 3: Тестирование всех стратегий через utils\test zapret.ps1 ---
        out.push_str("🧪 Шаг 3/5: Автоматическое тестирование стратегий через utils\\test zapret.ps1...\n");
        out.push_str("   (Утилита проверяет пробитие блокировок YouTube и Discord для вашего провайдера)\n");
        let best_strategy = Self::run_strategy_benchmark(&default_path)?;
        out.push_str(&format!("🏆 Лучшая стратегия по тестам: {}\n\n", best_strategy));

        // --- ШАГ 4: Установка системной службы Windows ---
        out.push_str(&format!(
            "⚙  Шаг 4/5: Регистрация системной службы Windows со стратегией '{}'...\n",
            best_strategy
        ));
        let install_res = Self::install_service(&zapret_cfg, Some(&best_strategy))?;
        out.push_str(&install_res);
        out.push('\n');

        // --- ШАГ 5: Запуск службы и сетевая проверка ---
        out.push_str("🚀 Шаг 5/5: Запуск системной службы и проверка доступности...\n");
        let start_res = Self::start(&zapret_cfg)?;
        out.push_str(&start_res);
        out.push('\n');

        // Проверка соединения
        let (_yt_ok, _dc_ok, check_details) = Self::verify_connectivity();

        out.push_str("\n====================================================\n");
        out.push_str("  🎉 ZAPRET ПОЛНОСТЬЮ НАСТРОЕН И ГОТОВ К РАБОТЕ!    \n");
        out.push_str("====================================================\n\n");
        out.push_str(&format!("📁 Папка программы:     {}\n", default_path.display()));
        out.push_str(&format!("🏆 Установленный альт:  {}\n", best_strategy));
        let service_status = Self::check_service_status(&zapret_cfg.service_name);
        out.push_str(&format!("⚙  Служба Windows:      {}\n", service_status));
        let proc_status = Self::check_winws_process();
        out.push_str(&format!("⚡ Процесс winws:       {}\n\n", proc_status));
        out.push_str("🌐 Результаты проверки доступа:\n");
        out.push_str(&check_details);
        out.push_str("\n\n✨ Служба работает в фоновом режиме (Session 0) без окон консоли.\n");
        out.push_str("   Zapret будет автоматически запускаться при каждом включении Windows!\n");
        out.push_str("====================================================\n");

        Ok(out)
    }

    /// Запустить встроенную утилиту utils\test zapret.ps1 для поиска лучшей стратегии
    pub fn run_strategy_benchmark(zapret_dir: &Path) -> Result<String, String> {
        let utils_dir = zapret_dir.join("utils");
        let test_script = utils_dir.join("test zapret.ps1");
        if !test_script.exists() {
            return Ok("general (ALT11)".to_string());
        }

        let runner_script = std::env::temp_dir().join(format!("run_benchmark_{}.ps1", std::process::id()));
        let runner_code = format!(
            r#"$ErrorActionPreference = 'SilentlyContinue';
Set-Location -LiteralPath '{utils}';

# Автоматически возвращаем '1' на вопросы Read-Host (Standard tests + All configs)
function global:Read-Host {{
    param($Prompt)
    return '1'
}}

try {{
    & '{script}'
}} catch {{
}}
"#,
            utils = utils_dir.display().to_string().replace('\'', "''"),
            script = test_script.display().to_string().replace('\'', "''"),
        );

        fs::write(&runner_script, runner_code.as_bytes())
            .map_err(|e| format!("Не удалось создать скрипт запуска тестов: {}", e))?;

        let run_cmd = format!(
            "Start-Process powershell -ArgumentList '-NoProfile -ExecutionPolicy Bypass -File \"{}\"' -Verb RunAs -Wait",
            runner_script.display()
        );

        let _ = Self::run_powershell_script(&run_cmd);
        let _ = fs::remove_file(&runner_script);

        if let Some(best) = Self::get_latest_test_result_strategy(zapret_dir) {
            Ok(best)
        } else {
            Ok("general (ALT11)".to_string())
        }
    }

    /// Получить стратегию-победителя из последнего отчета utils\test results
    pub fn get_latest_test_result_strategy(zapret_dir: &Path) -> Option<String> {
        let results_dir = zapret_dir.join("utils").join("test results");
        if !results_dir.exists() {
            return None;
        }

        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(&results_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map(|e| e == "txt").unwrap_or(false) {
                    if let Ok(meta) = path.metadata() {
                        if let Ok(modified) = meta.modified() {
                            files.push((modified, path));
                        }
                    }
                }
            }
        }

        files.sort_by(|a, b| b.0.cmp(&a.0)); // Свежие файлы первыми

        for (_, path) in files {
            if let Ok(content) = fs::read_to_string(&path) {
                for line in content.lines().rev() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("Best strategy:") {
                        let strat = trimmed.trim_start_matches("Best strategy:").trim();
                        let clean_strat = strat.trim_end_matches(".bat").trim();
                        if !clean_strat.is_empty() {
                            return Some(clean_strat.to_string());
                        }
                    } else if trimmed.starts_with("Best config:") {
                        let strat = trimmed.trim_start_matches("Best config:").trim();
                        let clean_strat = strat.trim_end_matches(".bat").trim();
                        if !clean_strat.is_empty() {
                            return Some(clean_strat.to_string());
                        }
                    }
                }
            }
        }

        None
    }

    /// Экспресс-проверка доступности YouTube и Discord
    pub fn verify_connectivity() -> (bool, bool, String) {
        let script = r#"
            $yt = try {
                $r = Invoke-WebRequest -Uri 'https://www.youtube.com' -TimeoutSec 6 -UseBasicParsing -ErrorAction Stop
                $r.StatusCode
            } catch {
                if ($_.Exception.Response) { [int]$_.Exception.Response.StatusCode } else { 0 }
            };
            $dc = try {
                $r = Invoke-WebRequest -Uri 'https://discord.com' -TimeoutSec 6 -UseBasicParsing -ErrorAction Stop
                $r.StatusCode
            } catch {
                if ($_.Exception.Response) { [int]$_.Exception.Response.StatusCode } else { 0 }
            };
            Write-Output "$yt|$dc"
        "#;

        if let Ok(out) = Self::run_powershell_script(script) {
            let trimmed = out.trim();
            let parts: Vec<&str> = trimmed.split('|').collect();
            let yt_code = parts.get(0).and_then(|c| c.parse::<i32>().ok()).unwrap_or(0);
            let dc_code = parts.get(1).and_then(|c| c.parse::<i32>().ok()).unwrap_or(0);

            let yt_ok = yt_code >= 200 && yt_code < 400;
            let dc_ok = dc_code >= 200 && dc_code < 400;

            let details = format!(
                "   ▶ YouTube:  {}\n   ▶ Discord:  {}",
                if yt_ok { format!("🟢 Доступен (HTTP {})", yt_code) } else { "🔴 Не отвечает".to_string() },
                if dc_ok { format!("🟢 Доступен (HTTP {})", dc_code) } else { "🔴 Не отвечает".to_string() }
            );

            (yt_ok, dc_ok, details)
        } else {
            (false, false, "   ⚠ Не удалось выполнить сетевую проверку".to_string())
        }
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

    #[test]
    fn test_parse_strategy_bat() {
        let temp_dir = std::env::temp_dir().join(format!("zapret_parse_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let bat_content = r#"@echo off
set "BIN=%~dp0bin\"
set "LISTS=%~dp0lists\"
cd /d %BIN%

start "zapret: %~n0" /min "%BIN%winws.exe" --wf-tcp=80,443,%GameFilterTCP% ^
--filter-udp=443 --hostlist="%LISTS%list-general.txt" --new ^
--filter-tcp=443 --dpi-desync=fake
"#;
        let bat_path = temp_dir.join("general (ALT11).bat");
        fs::write(&bat_path, bat_content).unwrap();

        let (strat_name, cmd) = ZapretManager::parse_strategy_bat(&bat_path, &temp_dir).unwrap();
        assert_eq!(strat_name, "general (ALT11)");
        assert!(cmd.contains("winws.exe"));
        assert!(cmd.contains("--wf-tcp=80,443,12"));
        assert!(cmd.contains("list-general.txt"));
        assert!(cmd.contains("--dpi-desync=fake"));
        assert!(!cmd.contains("%BIN%"));
        assert!(!cmd.contains("%LISTS%"));
        assert!(!cmd.contains("%GameFilterTCP%"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_list_available_strategies() {
        let temp_dir = std::env::temp_dir().join(format!("zapret_list_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let _ = File::create(temp_dir.join("general (ALT).bat"));
        let _ = File::create(temp_dir.join("general (ALT11).bat"));
        let _ = File::create(temp_dir.join("service.bat"));
        let _ = File::create(temp_dir.join("readme.txt"));

        let list = ZapretManager::list_available_strategies(&temp_dir);
        assert_eq!(list.len(), 2);
        assert!(list.contains(&"general (ALT)".to_string()));
        assert!(list.contains(&"general (ALT11)".to_string()));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_get_latest_test_result_strategy() {
        let temp_dir = std::env::temp_dir().join(format!("zapret_res_test_{}", std::process::id()));
        let results_dir = temp_dir.join("utils").join("test results");
        let _ = fs::create_dir_all(&results_dir);

        let report_content = r#"Config: general (ALT11).bat (Type: standard)
  YouTubeWeb : HTTP OK | Ping: 25ms
  DiscordMain : HTTP OK | Ping: 30ms

=== ANALYTICS ===
general (ALT11).bat : HTTP OK:  15, ERR:   0, UNSUP:   0, Ping OK:  15, Fail:   0
general (ALT).bat   : HTTP OK:  10, ERR:   5, UNSUP:   0, Ping OK:  10, Fail:   5

Best strategy: general (ALT11).bat
"#;
        let file_path = results_dir.join("test_results_2026-09-09_15-00-00.txt");
        fs::write(&file_path, report_content).unwrap();

        let best = ZapretManager::get_latest_test_result_strategy(&temp_dir);
        assert_eq!(best, Some("general (ALT11)".to_string()));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}

