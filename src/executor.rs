use crate::config::ActionItem;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub id: String,
    pub name: String,
    pub command: String,
    pub output: String,
    pub exit_code: Option<i32>,
    pub duration: Duration,
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub success: bool,
}

pub struct CommandExecutor;

impl CommandExecutor {
    pub fn execute(action: &ActionItem, default_cwd: Option<&str>) -> ExecutionResult {
        let start = Instant::now();
        let timestamp = chrono::Local::now();

        let is_self_cmd = action.command.starts_with("dark-cli ") || action.command.starts_with("git_project1 ");
        let mut cmd = if is_self_cmd {
            if let Ok(current_exe) = std::env::current_exe() {
                let sub = if action.command.starts_with("dark-cli ") {
                    &action.command[9..]
                } else {
                    &action.command[13..]
                };
                let mut c = Command::new(current_exe);
                c.args(split_cmd_args(sub));
                c
            } else if cfg!(target_os = "windows") {
                let mut c = Command::new("cmd");
                c.args(["/C", &action.command]);
                c
            } else {
                let mut c = Command::new("sh");
                c.args(["-c", &action.command]);
                c
            }
        } else if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(["/C", &action.command]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", &action.command]);
            c
        };

        // Determine working directory
        if let Some(ref cwd) = action.cwd {
            cmd.current_dir(Path::new(cwd));
        } else if let Some(cwd) = default_cwd {
            cmd.current_dir(Path::new(cwd));
        }

        // Set custom environment variables
        if let Some(ref envs) = action.env {
            for (key, val) in envs {
                cmd.env(key, val);
            }
        }

        let output_res = cmd.output();
        let duration = start.elapsed();

        match output_res {
            Ok(output) => {
                let stdout = decode_bytes(&output.stdout);
                let stderr = decode_bytes(&output.stderr);

                let mut combined = String::new();
                if !stdout.is_empty() {
                    combined.push_str(&stdout);
                }
                if !stderr.is_empty() {
                    if !combined.is_empty() && !combined.ends_with('\n') {
                        combined.push('\n');
                    }
                    combined.push_str(&stderr);
                }

                if combined.trim().is_empty() {
                    combined = "[Команда завершилась без текстового вывода]".to_string();
                }

                ExecutionResult {
                    id: action.id.clone(),
                    name: action.name.clone(),
                    command: action.command.clone(),
                    output: combined,
                    exit_code: output.status.code(),
                    duration,
                    timestamp,
                    success: output.status.success(),
                }
            }
            Err(e) => ExecutionResult {
                id: action.id.clone(),
                name: action.name.clone(),
                command: action.command.clone(),
                output: format!("Ошибка запуска процесса: {}", e),
                exit_code: None,
                duration,
                timestamp,
                success: false,
            },
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

fn split_cmd_args(cmd_str: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = ' ';

    for ch in cmd_str.chars() {
        if (ch == '"' || ch == '\'') && (!in_quotes || ch == quote_char) {
            in_quotes = !in_quotes;
            quote_char = ch;
        } else if ch.is_whitespace() && !in_quotes {
            if !current.is_empty() {
                args.push(current.clone());
                current.clear();
            }
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_cmd_args() {
        let args = split_cmd_args("zapret service-install --strategy \"general (ALT11)\"");
        assert_eq!(args, vec!["zapret", "service-install", "--strategy", "general (ALT11)"]);

        let args2 = split_cmd_args("tg-proxy update --path 'C:\\Program Files\\tg'");
        assert_eq!(args2, vec!["tg-proxy", "update", "--path", "C:\\Program Files\\tg"]);
    }
}

