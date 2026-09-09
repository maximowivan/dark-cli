use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    pub default_cwd: Option<String>,
    pub shell: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_category")]
    pub category: String,
    pub command: String,
    pub cwd: Option<String>,
    pub shortcut: Option<String>,
    pub env: Option<HashMap<String, String>>,
}

fn default_category() -> String {
    "General".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZapretConfig {
    pub path: Option<String>,
    #[serde(default = "default_zapret_service")]
    pub service_name: String,
    #[serde(default = "default_zapret_repo")]
    pub github_repo: String,
}

fn default_zapret_service() -> String {
    "zapret".to_string()
}

fn default_zapret_repo() -> String {
    "Flowseal/zapret-discord-youtube".to_string()
}

impl Default for ZapretConfig {
    fn default() -> Self {
        Self {
            path: None,
            service_name: default_zapret_service(),
            github_repo: default_zapret_repo(),
        }
    }
}

impl ZapretConfig {
    pub fn get_resolved_path(&self) -> Option<PathBuf> {
        if let Some(ref p) = self.path {
            let pb = PathBuf::from(p);
            if pb.exists() {
                return Some(pb);
            }
        }

        // Try detecting from registry on Windows
        #[cfg(target_os = "windows")]
        {
            if let Some(reg_path) = Self::detect_from_registry() {
                if reg_path.exists() {
                    return Some(reg_path);
                }
            }
        }

        // Check common candidate locations
        let candidates = [
            "S:\\zapret",
            "C:\\zapret",
            "D:\\zapret",
            "E:\\zapret",
            "C:\\Program Files\\zapret",
        ];

        for c in &candidates {
            let pb = PathBuf::from(c);
            if pb.exists() && (pb.join("service.bat").exists() || pb.join("bin\\winws.exe").exists()) {
                return Some(pb);
            }
        }

        None
    }

    #[cfg(target_os = "windows")]
    fn detect_from_registry() -> Option<PathBuf> {
        use std::process::Command;
        let output = Command::new("reg")
            .args(["query", "HKLM\\SYSTEM\\CurrentControlSet\\Services\\zapret", "/v", "ImagePath"])
            .output()
            .ok()?;

        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if line.contains("ImagePath") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    for part in parts {
                        let clean = part.trim_matches('"');
                        if clean.ends_with("winws.exe") {
                            let exe_path = PathBuf::from(clean);
                            // Path is ...\bin\winws.exe -> get root zapret dir
                            if let Some(bin_dir) = exe_path.parent() {
                                if let Some(root) = bin_dir.parent() {
                                    return Some(root.to_path_buf());
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub zapret: Option<ZapretConfig>,
    #[serde(default)]
    pub actions: Vec<ActionItem>,
}

impl AppConfig {
    pub fn get_config_path() -> PathBuf {
        let local_path = Path::new("config.toml");
        if local_path.exists() {
            return local_path.to_path_buf();
        }

        if let Some(config_dir) = dirs::config_dir() {
            let app_dir = config_dir.join("dark-cli");
            let global_path = app_dir.join("config.toml");
            if global_path.exists() {
                return global_path;
            }
        }

        // Default to local config.toml
        local_path.to_path_buf()
    }

    pub fn load_or_create() -> Result<(Self, PathBuf), String> {
        let path = Self::get_config_path();
        if path.exists() {
            let content = fs::read_to_string(&path)
                .map_err(|e| format!("Не удалось прочитать {}: {}", path.display(), e))?;
            let config: AppConfig = toml::from_str(&content)
                .map_err(|e| format!("Ошибка парсинга {}: {}", path.display(), e))?;
            Ok((config, path))
        } else {
            let default_config = Self::default_with_samples();
            let toml_str = default_config.to_toml_string();
            fs::write(&path, &toml_str)
                .map_err(|e| format!("Не удалось создать {}: {}", path.display(), e))?;
            Ok((default_config, path))
        }
    }

    pub fn reload(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Не удалось прочитать {}: {}", path.display(), e))?;
        let config: AppConfig = toml::from_str(&content)
            .map_err(|e| format!("Ошибка парсинга {}: {}", path.display(), e))?;
        Ok(config)
    }

    pub fn default_with_samples() -> Self {
        Self {
            settings: Settings {
                default_cwd: None,
                shell: None,
            },
            zapret: None,
            actions: vec![
                ActionItem {
                    id: "cargo-check".to_string(),
                    name: "Проверка Cargo".to_string(),
                    description: "Быстрая проверка компиляции без сборки исполняемого файла".to_string(),
                    category: "Разработка".to_string(),
                    command: "cargo check".to_string(),
                    cwd: None,
                    shortcut: Some("c".to_string()),
                    env: None,
                },
                ActionItem {
                    id: "cargo-build".to_string(),
                    name: "Сборка Cargo".to_string(),
                    description: "Сборка debug-бинарника проекта".to_string(),
                    category: "Разработка".to_string(),
                    command: "cargo build".to_string(),
                    cwd: None,
                    shortcut: Some("b".to_string()),
                    env: None,
                },
                ActionItem {
                    id: "cargo-test".to_string(),
                    name: "Тесты Cargo".to_string(),
                    description: "Запуск автоматических тестов".to_string(),
                    category: "Разработка".to_string(),
                    command: "cargo test".to_string(),
                    cwd: None,
                    shortcut: Some("t".to_string()),
                    env: None,
                },
                ActionItem {
                    id: "git-status".to_string(),
                    name: "Статус Git".to_string(),
                    description: "Просмотр статуса репозитория и изменений".to_string(),
                    category: "Git".to_string(),
                    command: "git status -s".to_string(),
                    cwd: None,
                    shortcut: Some("g".to_string()),
                    env: None,
                },
                ActionItem {
                    id: "git-log".to_string(),
                    name: "История Git (Log)".to_string(),
                    description: "Последние 10 коммитов в компактном виде".to_string(),
                    category: "Git".to_string(),
                    command: "git log --oneline -n 10 --graph".to_string(),
                    cwd: None,
                    shortcut: Some("l".to_string()),
                    env: None,
                },
                ActionItem {
                    id: "open-folder".to_string(),
                    name: "Открыть папку".to_string(),
                    description: "Открыть текущую папку в Проводнике Windows".to_string(),
                    category: "Система".to_string(),
                    command: "explorer .".to_string(),
                    cwd: None,
                    shortcut: Some("e".to_string()),
                    env: None,
                },
                ActionItem {
                    id: "sys-ip".to_string(),
                    name: "Сетевые настройки".to_string(),
                    description: "Показать текущие сетевые адреса (ipconfig)".to_string(),
                    category: "Система".to_string(),
                    command: "ipconfig".to_string(),
                    cwd: None,
                    shortcut: None,
                    env: None,
                },
                ActionItem {
                    id: "open-github".to_string(),
                    name: "Открыть GitHub".to_string(),
                    description: "Открыть сайт GitHub в браузере по умолчанию".to_string(),
                    category: "Веб".to_string(),
                    command: "cmd /c start https://github.com".to_string(),
                    cwd: None,
                    shortcut: None,
                    env: None,
                },
            ],
        }
    }

    pub fn to_toml_string(&self) -> String {
        let mut out = String::new();
        out.push_str("# Конфигурационный файл Dark CLI / Launcher\n");
        out.push_str("# Вы можете свободно добавлять сюда любые свои команды и скрипты!\n\n");
        out.push_str("[settings]\n");
        out.push_str("# default_cwd = \"C:\\\\projects\"\n");
        out.push_str("# shell = \"powershell\" # или \"cmd\"\n\n");

        for action in &self.actions {
            out.push_str("[[actions]]\n");
            out.push_str(&format!("id = {:?}\n", action.id));
            out.push_str(&format!("name = {:?}\n", action.name));
            out.push_str(&format!("description = {:?}\n", action.description));
            out.push_str(&format!("category = {:?}\n", action.category));
            out.push_str(&format!("command = {:?}\n", action.command));
            if let Some(ref cwd) = action.cwd {
                out.push_str(&format!("cwd = {:?}\n", cwd));
            }
            if let Some(ref sc) = action.shortcut {
                out.push_str(&format!("shortcut = {:?}\n", sc));
            }
            out.push('\n');
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_parse() {
        let default_cfg = AppConfig::default_with_samples();
        let toml_str = default_cfg.to_toml_string();
        let parsed: Result<AppConfig, _> = toml::from_str(&toml_str);
        assert!(parsed.is_ok(), "Config should deserialize cleanly from toml string");
        let parsed = parsed.unwrap();
        assert_eq!(parsed.actions.len(), default_cfg.actions.len());
        assert_eq!(parsed.actions[0].id, "cargo-check");
    }

    #[test]
    fn test_custom_action_deserialization() {
        let sample = r#"
            [settings]
            default_cwd = "C:/test"

            [[actions]]
            id = "custom-test"
            name = "Custom Test"
            description = "A test command"
            category = "Testing"
            command = "echo hello"
            shortcut = "x"
        "#;

        let parsed: AppConfig = toml::from_str(sample).expect("Failed to parse sample");
        assert_eq!(parsed.settings.default_cwd.as_deref(), Some("C:/test"));
        assert_eq!(parsed.actions.len(), 1);
        assert_eq!(parsed.actions[0].id, "custom-test");
        assert_eq!(parsed.actions[0].shortcut.as_deref(), Some("x"));
    }
}
