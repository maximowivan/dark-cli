use crate::config::{ActionItem, AppConfig};
use crate::executor::{CommandExecutor, ExecutionResult};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Actions,
    Output,
    History,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    CommandInput,
    PaletteSearch,
}

pub struct App {
    pub config: AppConfig,
    pub config_path: PathBuf,
    pub active_tab: Tab,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub selected_action_idx: usize,
    pub palette_selected_idx: usize,
    pub output_scroll: u16,
    pub history_scroll: u16,
    pub last_result: Option<ExecutionResult>,
    pub history: Vec<ExecutionResult>,
    pub status_message: Option<(String, bool)>,
    pub should_quit: bool,
    matcher: SkimMatcherV2,
}

impl App {
    pub fn new(config: AppConfig, config_path: PathBuf) -> Self {
        Self {
            config,
            config_path,
            active_tab: Tab::Actions,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            selected_action_idx: 0,
            palette_selected_idx: 0,
            output_scroll: 0,
            history_scroll: 0,
            last_result: None,
            history: Vec::new(),
            status_message: Some(("Готов к работе. Нажмите [?] для справки.".to_string(), false)),
            should_quit: false,
            matcher: SkimMatcherV2::default(),
        }
    }

    pub fn filtered_actions(&self) -> Vec<(usize, &ActionItem)> {
        if self.input_buffer.is_empty() {
            return self.config.actions.iter().enumerate().collect();
        }

        let query = &self.input_buffer;
        let mut matches: Vec<(i64, usize, &ActionItem)> = self
            .config
            .actions
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                let target = format!("{} {} {} {}", item.name, item.id, item.category, item.description);
                self.matcher.fuzzy_match(&target, query).map(|score| (score, idx, item))
            })
            .collect();

        // Sort by fuzzy match score descending
        matches.sort_by(|a, b| b.0.cmp(&a.0));
        matches.into_iter().map(|(_, idx, item)| (idx, item)).collect()
    }

    pub fn execute_selected_action(&mut self) {
        let filtered = self.filtered_actions();
        let action = if self.input_mode == InputMode::PaletteSearch {
            filtered.get(self.palette_selected_idx).map(|(_, a)| (*a).clone())
        } else {
            filtered.get(self.selected_action_idx).map(|(_, a)| (*a).clone())
        };

        if let Some(action) = action {
            self.execute_action(&action);
        } else {
            self.set_status("Команда не найдена", true);
        }

        if self.input_mode == InputMode::PaletteSearch {
            self.input_mode = InputMode::Normal;
            self.input_buffer.clear();
        }
    }

    pub fn execute_action(&mut self, action: &ActionItem) {
        self.set_status(format!("Выполняется: {}...", action.name), false);
        
        let result = CommandExecutor::execute(
            action,
            self.config.settings.default_cwd.as_deref(),
        );

        if result.success {
            self.set_status(
                format!("Успешно выполнено: {} (за {:.2?})", result.name, result.duration),
                false,
            );
        } else {
            self.set_status(
                format!("Ошибка выполнения: {} (код: {:?})", result.name, result.exit_code),
                true,
            );
        }

        self.last_result = Some(result.clone());
        self.history.push(result);
        self.output_scroll = 0;
        self.active_tab = Tab::Output;
    }

    pub fn execute_action_by_id(&mut self, id: &str) -> bool {
        if let Some(action) = self.config.actions.iter().find(|a| a.id.eq_ignore_ascii_case(id)).cloned() {
            self.execute_action(&action);
            true
        } else {
            self.set_status(format!("Действие с ID '{}' не найдено", id), true);
            false
        }
    }

    pub fn handle_slash_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.trim().split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        match parts[0] {
            "/run" => {
                if parts.len() > 1 {
                    self.execute_action_by_id(parts[1]);
                } else {
                    self.set_status("Использование: /run <id>", true);
                }
            }
            "/reload" | "/r" => {
                self.reload_config();
            }
            "/clear" => {
                self.last_result = None;
                self.output_scroll = 0;
                self.set_status("Окно вывода очищено", false);
            }
            "/help" | "/h" => {
                self.active_tab = Tab::Help;
            }
            "/exit" | "/quit" | "/q" => {
                self.should_quit = true;
            }
            other => {
                self.set_status(format!("Неизвестная команда: {}. Введите /help", other), true);
            }
        }
    }

    pub fn reload_config(&mut self) {
        match AppConfig::reload(&self.config_path) {
            Ok(new_config) => {
                self.config = new_config;
                self.selected_action_idx = 0;
                self.set_status("Конфигурация успешно перезагружена!", false);
            }
            Err(e) => {
                self.set_status(format!("Ошибка перезагрузки: {}", e), true);
            }
        }
    }

    pub fn set_status<S: Into<String>>(&mut self, message: S, is_error: bool) {
        self.status_message = Some((message.into(), is_error));
    }

    pub fn next_action(&mut self) {
        let count = self.filtered_actions().len();
        if count > 0 {
            self.selected_action_idx = (self.selected_action_idx + 1) % count;
        }
    }

    pub fn prev_action(&mut self) {
        let count = self.filtered_actions().len();
        if count > 0 {
            if self.selected_action_idx == 0 {
                self.selected_action_idx = count - 1;
            } else {
                self.selected_action_idx -= 1;
            }
        }
    }

    pub fn next_palette_item(&mut self) {
        let count = self.filtered_actions().len();
        if count > 0 {
            self.palette_selected_idx = (self.palette_selected_idx + 1) % count;
        }
    }

    pub fn prev_palette_item(&mut self) {
        let count = self.filtered_actions().len();
        if count > 0 {
            if self.palette_selected_idx == 0 {
                self.palette_selected_idx = count - 1;
            } else {
                self.palette_selected_idx -= 1;
            }
        }
    }

    pub fn next_tab(&mut self) {
        self.active_tab = match self.active_tab {
            Tab::Actions => Tab::Output,
            Tab::Output => Tab::History,
            Tab::History => Tab::Help,
            Tab::Help => Tab::Actions,
        };
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = match self.active_tab {
            Tab::Actions => Tab::Help,
            Tab::Output => Tab::Actions,
            Tab::History => Tab::Output,
            Tab::Help => Tab::History,
        };
    }
}
