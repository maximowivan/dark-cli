use std::path::{Path, PathBuf};
use std::process::Command;

const RUN_KEY: &str = "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run";

pub struct AutorunManager;

impl AutorunManager {
    /// Проверка, включен ли автозапуск Dark-CLI
    pub fn is_dark_cli_enabled() -> bool {
        #[cfg(target_os = "windows")]
        {
            let output = Command::new("reg")
                .args(["query", RUN_KEY, "/v", "DarkCLI"])
                .output();

            if let Ok(out) = output {
                return out.status.success();
            }
        }
        false
    }

    /// Включение автозапуска Dark-CLI (в свернутом режиме в трее)
    pub fn enable_dark_cli() -> Result<String, String> {
        #[cfg(target_os = "windows")]
        {
            let current_exe = std::env::current_exe()
                .map_err(|e| format!("Не удалось определить путь к текущей программе: {}", e))?;

            let exe_str = current_exe.display().to_string();
            let cmd_val = format!("\"{}\" --tray", exe_str);

            let output = Command::new("reg")
                .args(["add", RUN_KEY, "/v", "DarkCLI", "/t", "REG_SZ", "/d", &cmd_val, "/f"])
                .output()
                .map_err(|e| format!("Не удалось выполнить reg add: {}", e))?;

            if output.status.success() {
                Ok(format!(
                    "✨ Автозапуск Dark-CLI успешно включен!\n\
                     📁 Путь: {}\n\
                     Параметр: --tray (программа будет стартовать свернутой в трей)",
                    exe_str
                ))
            } else {
                Err(format!(
                    "Ошибка записи в реестр: {}",
                    String::from_utf8_lossy(&output.stderr)
                ))
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err("Автозагрузка через реестр доступна только на Windows".to_string())
        }
    }

    /// Отключение автозапуска Dark-CLI
    pub fn disable_dark_cli() -> Result<String, String> {
        #[cfg(target_os = "windows")]
        {
            let output = Command::new("reg")
                .args(["delete", RUN_KEY, "/v", "DarkCLI", "/f"])
                .output()
                .map_err(|e| format!("Не удалось выполнить reg delete: {}", e))?;

            if output.status.success() {
                Ok("✨ Автозапуск Dark-CLI успешно отключен.".to_string())
            } else {
                // Если записи и так не было
                Ok("Автозапуск Dark-CLI уже был выключен (запись отсутствует).".to_string())
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err("Доступно только на Windows".to_string())
        }
    }

    /// Проверка, включен ли автозапуск Flowseal TG WS Proxy
    pub fn is_tg_proxy_enabled() -> bool {
        #[cfg(target_os = "windows")]
        {
            let output = Command::new("reg")
                .args(["query", RUN_KEY, "/v", "TgWsProxy"])
                .output();

            if let Ok(out) = output {
                return out.status.success();
            }
        }
        false
    }

    /// Включение автозапуска TG WS Proxy
    pub fn enable_tg_proxy(custom_path: Option<&Path>) -> Result<String, String> {
        #[cfg(target_os = "windows")]
        {
            let target_path = if let Some(p) = custom_path {
                p.to_path_buf()
            } else {
                Self::find_tg_proxy_path().ok_or_else(|| {
                    "Файл TgWsProxy_windows.exe не найден. Установите прокси через меню Telegram Proxy перед включением автозагрузки.".to_string()
                })?
            };

            let path_str = target_path.display().to_string();
            let cmd_val = format!("\"{}\"", path_str);

            let output = Command::new("reg")
                .args(["add", RUN_KEY, "/v", "TgWsProxy", "/t", "REG_SZ", "/d", &cmd_val, "/f"])
                .output()
                .map_err(|e| format!("Не удалось записать в реестр: {}", e))?;

            if output.status.success() {
                Ok(format!(
                    "✨ Автозапуск Flowseal TG WS Proxy успешно включен!\n\
                     📁 Запускаемый файл: {}\n\
                     Прокси будет автоматически стартовать в фоновом режиме (в трее) при входе в Windows.",
                    path_str
                ))
            } else {
                Err(format!(
                    "Ошибка записи в реестр: {}",
                    String::from_utf8_lossy(&output.stderr)
                ))
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err("Доступно только на Windows".to_string())
        }
    }

    /// Отключение автозапуска TG WS Proxy
    pub fn disable_tg_proxy() -> Result<String, String> {
        #[cfg(target_os = "windows")]
        {
            let output = Command::new("reg")
                .args(["delete", RUN_KEY, "/v", "TgWsProxy", "/f"])
                .output()
                .map_err(|e| format!("Не удалось выполнить reg delete: {}", e))?;

            if output.status.success() {
                Ok("✨ Автозапуск Flowseal TG WS Proxy успешно отключен.".to_string())
            } else {
                Ok("Автозапуск TG WS Proxy уже был выключен.".to_string())
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err("Доступно только на Windows".to_string())
        }
    }

    /// Поиск пути к TgWsProxy_windows.exe
    fn find_tg_proxy_path() -> Option<PathBuf> {
        let mut candidates = Vec::new();
        if let Some(home) = dirs::home_dir() {
            candidates.push(home.join("Downloads").join("TgWsProxy_windows.exe"));
            candidates.push(home.join("Desktop").join("TgWsProxy_windows.exe"));
        }
        if let Some(config_dir) = dirs::config_dir() {
            candidates.push(config_dir.join("TgWsProxy").join("TgWsProxy_windows.exe"));
        }
        candidates.push(PathBuf::from("C:\\tg-ws-proxy\\TgWsProxy_windows.exe"));
        candidates.push(PathBuf::from("C:\\zapret\\TgWsProxy_windows.exe"));

        for c in candidates {
            if c.exists() {
                return Some(c);
            }
        }
        None
    }

    /// Получение типа запуска системной службы Zapret
    pub fn get_zapret_service_autorun() -> String {
        #[cfg(target_os = "windows")]
        {
            // 1. Проверяем регистрацию службы в системном реестре HKLM (наиболее надежный способ,
            // не подверженный ошибке RPC 1734 при длинных параметрах ImagePath)
            let reg_output = Command::new("reg")
                .args(["query", "HKLM\\SYSTEM\\CurrentControlSet\\Services\\zapret"])
                .output();

            if let Ok(out) = reg_output {
                if out.status.success() {
                    let text = String::from_utf8_lossy(&out.stdout);

                    // Проверяем текущее состояние работы службы через sc query zapret
                    let is_running = if let Ok(q) = Command::new("sc").args(["query", "zapret"]).output() {
                        let q_text = String::from_utf8_lossy(&q.stdout).to_uppercase();
                        q_text.contains("RUNNING")
                    } else {
                        false
                    };

                    let run_str = if is_running { "🟢 Работает" } else { "🔴 Остановлена" };

                    // Получаем сохраненную стратегию
                    let strategy = text
                        .lines()
                        .find(|l| l.contains("zapret-discord-youtube"))
                        .and_then(|l| l.split("REG_SZ").nth(1))
                        .map(|s| s.trim())
                        .unwrap_or("");

                    let strat_str = if !strategy.is_empty() {
                        format!(" [{}]", strategy)
                    } else {
                        String::new()
                    };

                    // Значение параметра Start (0x2 = AUTO_START, 0x3 = DEMAND_START, 0x4 = DISABLED)
                    let start_val = text
                        .lines()
                        .find(|l| {
                            let trimmed = l.trim_start();
                            trimmed.starts_with("Start ") || trimmed.starts_with("Start\t")
                        })
                        .and_then(|l| l.split_whitespace().last())
                        .unwrap_or("");

                    let type_str = match start_val {
                        "0x2" => "🟢 Автозапуск Windows (AUTO_START)",
                        "0x3" => "🟡 Вручную (DEMAND_START)",
                        "0x4" => "🔴 Отключена (DISABLED)",
                        _ => "🟢 Зарегистрирована",
                    };

                    return format!("{} • {}{}", type_str, run_str, strat_str);
                }
            }

            // 2. Резервный вариант через sc qc
            if let Ok(out) = Command::new("sc").args(["qc", "zapret"]).output() {
                if out.status.success() {
                    let text = String::from_utf8_lossy(&out.stdout).to_uppercase();
                    if text.contains("AUTO_START") {
                        return "🟢 Автозапуск Windows (AUTO_START)".to_string();
                    } else if text.contains("DEMAND_START") {
                        return "🟡 Вручную (DEMAND_START)".to_string();
                    } else if text.contains("DISABLED") {
                        return "🔴 Отключена (DISABLED)".to_string();
                    }
                    return "🟢 Зарегистрирована".to_string();
                }
            }
        }
        "🔴 Не установлена как служба".to_string()
    }

    /// Формирование полного отчета о состоянии системной интеграции
    pub fn get_status_report() -> String {
        let dark_cli_auto = if Self::is_dark_cli_enabled() {
            "🟢 Включен (запуск в трее при старте Windows)"
        } else {
            "🔴 Отключен"
        };

        let tg_auto = if Self::is_tg_proxy_enabled() {
            "🟢 Включен (старт прокси в трее)"
        } else {
            "🔴 Отключен"
        };

        let zapret_auto = Self::get_zapret_service_autorun();

        let mut out = String::new();
        out.push_str("───────────────────────────────────────────────────────────────────────────\n");
        out.push_str("              СТАТУС СИСТЕМНОЙ ИНТЕГРАЦИИ И АВТОЗАГРУЗКИ                   \n");
        out.push_str("───────────────────────────────────────────────────────────────────────────\n");
        out.push_str(&format!("  🖥 Dark-CLI:               {}\n", dark_cli_auto));
        out.push_str(&format!("  ✈️ Flowseal TG WS Proxy:   {}\n", tg_auto));
        out.push_str(&format!("  ⚡ Служба Zapret:          {}\n", zapret_auto));
        out.push_str("  🔔 Системный трей Windows: 🟢 Активен (иконка в области уведомлений возле часов)\n");
        out.push_str("───────────────────────────────────────────────────────────────────────────\n\n");

        out.push_str("💡 ПОЛЕЗНЫЕ СОВЕТЫ:\n");
        out.push_str("• Нажмите [h] в главном меню для мгновенного скрытия окна Dark-CLI в трей.\n");
        out.push_str("• По правому клику на иконку в трее доступно быстрое управление Zapret и Telegram.\n");
        out.push_str("• Для вызова окна из трея дважды кликните по иконке или выберите «Показать / Скрыть».\n");
        out.push_str("• При включенном автозапуске Dark-CLI запускается скрытым в трей через ключ --tray.\n");

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_report_format() {
        let report = AutorunManager::get_status_report();
        assert!(report.contains("СТАТУС СИСТЕМНОЙ ИНТЕГРАЦИИ"));
        assert!(report.contains("Dark-CLI"));
        assert!(report.contains("Flowseal TG WS Proxy"));
        assert!(report.contains("Служба Zapret"));
    }
}
