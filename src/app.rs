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

#[derive(Clone, Debug, PartialEq)]
pub enum InstallDialogState {
    ChooseOption { selected: usize }, // 0: C:\zapret, 1: Custom path
    EnterPath { input: String },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ViewItem<'a> {
    Back,
    Folder { name: String, count: usize },
    Action(&'a ActionItem),
}

pub struct App {
    pub config: AppConfig,
    pub config_path: PathBuf,
    pub active_tab: Tab,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub current_folder: Option<String>,
    pub selected_action_idx: usize,
    pub palette_selected_idx: usize,
    pub output_scroll: u16,
    pub history_scroll: u16,
    pub last_result: Option<ExecutionResult>,
    pub history: Vec<ExecutionResult>,
    pub status_message: Option<(String, bool)>,
    pub should_quit: bool,
    pub install_dialog: Option<InstallDialogState>,
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
            current_folder: None,
            selected_action_idx: 0,
            palette_selected_idx: 0,
            output_scroll: 0,
            history_scroll: 0,
            last_result: None,
            history: Vec::new(),
            status_message: Some(("Готов к работе. Нажмите [?] для справки.".to_string(), false)),
            should_quit: false,
            install_dialog: None,
            matcher: SkimMatcherV2::default(),
        }
    }

    pub fn categories(&self) -> Vec<String> {
        let mut cats = Vec::new();
        for action in &self.config.actions {
            if !cats.iter().any(|c: &String| c.eq_ignore_ascii_case(&action.category)) {
                cats.push(action.category.clone());
            }
        }
        cats
    }

    pub fn current_items(&self) -> Vec<ViewItem<'_>> {
        // Поиск по всем действиям сразу, если введен поисковый запрос
        if !self.input_buffer.is_empty() {
            let query = &self.input_buffer;
            let mut matches: Vec<(i64, &ActionItem)> = self
                .config
                .actions
                .iter()
                .filter_map(|item| {
                    let target = format!("{} {} {} {}", item.name, item.id, item.category, item.description);
                    self.matcher.fuzzy_match(&target, query).map(|score| (score, item))
                })
                .collect();
            matches.sort_by(|a, b| b.0.cmp(&a.0));
            return matches.into_iter().map(|(_, item)| ViewItem::Action(item)).collect();
        }

        // Вложенные папки
        match self.current_folder {
            None => {
                // Корень: отображаем список папок категорий
                let mut items = Vec::new();
                for cat in self.categories() {
                    let count = self.config.actions.iter().filter(|a| a.category.eq_ignore_ascii_case(&cat)).count();
                    items.push(ViewItem::Folder { name: cat, count });
                }
                items
            }
            Some(ref cat) => {
                // Внутри папки: кнопка возврата + действия этой категории
                let mut items = vec![ViewItem::Back];
                for action in &self.config.actions {
                    if action.category.eq_ignore_ascii_case(cat) {
                        items.push(ViewItem::Action(action));
                    }
                }
                items
            }
        }
    }

    pub fn enter_selected(&mut self) {
        if self.input_mode == InputMode::PaletteSearch {
            let filtered = self.filtered_actions();
            if let Some((_, action)) = filtered.get(self.palette_selected_idx) {
                let a = (*action).clone();
                self.execute_action(&a);
            }
            self.input_mode = InputMode::Normal;
            self.input_buffer.clear();
            return;
        }

        let items = self.current_items();
        match items.get(self.selected_action_idx) {
            Some(ViewItem::Folder { name, .. }) => {
                self.current_folder = Some(name.clone());
                self.selected_action_idx = 0;
            }
            Some(ViewItem::Back) => {
                self.go_back();
            }
            Some(ViewItem::Action(action)) => {
                if action.id == "zapret-install" || (action.id == "zapret-update" && !self.is_zapret_installed()) {
                    self.start_zapret_install_dialog();
                    return;
                }
                let a = (*action).clone();
                self.execute_action(&a);
            }
            None => {
                self.set_status("Команда не найдена", true);
            }
        }
    }

    pub fn is_zapret_installed(&self) -> bool {
        self.config.zapret.as_ref().map(|z| z.is_installed()).unwrap_or(false)
            || crate::config::ZapretConfig::default().is_installed()
    }

    pub fn get_zapret_path(&self) -> Option<PathBuf> {
        self.config.zapret.as_ref().and_then(|z| z.get_resolved_path())
            .or_else(|| crate::config::ZapretConfig::default().get_resolved_path())
    }

    pub fn get_zapret_summary(&self) -> (bool, Option<PathBuf>, Option<String>) {
        let default_cfg = crate::config::ZapretConfig::default();
        let cfg = self.config.zapret.as_ref().unwrap_or(&default_cfg);
        crate::zapret::ZapretManager::get_install_summary(cfg)
    }

    pub fn start_zapret_install_dialog(&mut self) {
        self.install_dialog = Some(InstallDialogState::ChooseOption { selected: 0 });
    }

    pub fn confirm_install_path(&mut self, chosen_path: &str) {
        self.install_dialog = None;
        let p = chosen_path.trim().trim_matches('"');
        let target_str = if p.is_empty() { "C:\\zapret" } else { p };

        // Save path to config
        self.config.set_zapret_path(target_str.to_string());
        let _ = self.config.save(&self.config_path);

        // Run installation command
        let action = ActionItem {
            id: "zapret-install-run".to_string(),
            name: "Установка Zapret".to_string(),
            description: format!("Установка Zapret в {}", target_str),
            category: "Zapret".to_string(),
            command: format!("dark-cli zapret update --force --path \"{}\"", target_str),
            cwd: None,
            shortcut: None,
            env: None,
        };
        self.execute_action(&action);
    }

    pub fn execute_selected_action(&mut self) {
        self.enter_selected();
    }

    pub fn go_back(&mut self) -> bool {
        if let Some(folder) = self.current_folder.take() {
            if let Some(idx) = self.categories().iter().position(|c| c.eq_ignore_ascii_case(&folder)) {
                self.selected_action_idx = idx;
            } else {
                self.selected_action_idx = 0;
            }
            true
        } else {
            false
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

        matches.sort_by(|a, b| b.0.cmp(&a.0));
        matches.into_iter().map(|(_, idx, item)| (idx, item)).collect()
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
        let count = self.current_items().len();
        if count > 0 {
            self.selected_action_idx = (self.selected_action_idx + 1) % count;
        }
    }

    pub fn prev_action(&mut self) {
        let count = self.current_items().len();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_app() -> App {
        let config = AppConfig::default_with_samples();
        App::new(config, std::env::temp_dir().join("dark_cli_test_config.toml"))
    }

    #[test]
    fn test_tab_cycling() {
        let mut app = create_test_app();
        assert_eq!(app.active_tab, Tab::Actions);

        app.next_tab();
        assert_eq!(app.active_tab, Tab::Output);

        app.next_tab();
        assert_eq!(app.active_tab, Tab::History);

        app.next_tab();
        assert_eq!(app.active_tab, Tab::Help);

        app.next_tab();
        assert_eq!(app.active_tab, Tab::Actions);

        app.prev_tab();
        assert_eq!(app.active_tab, Tab::Help);
    }

    #[test]
    fn test_action_navigation() {
        let mut app = create_test_app();
        let total = app.current_items().len();
        assert!(total > 0);

        assert_eq!(app.selected_action_idx, 0);
        app.next_action();
        assert_eq!(app.selected_action_idx, 1);

        app.prev_action();
        assert_eq!(app.selected_action_idx, 0);

        // Wrap around to the end
        app.prev_action();
        assert_eq!(app.selected_action_idx, total - 1);
    }

    #[test]
    fn test_folder_navigation() {
        let mut app = create_test_app();
        assert!(app.current_folder.is_none());

        // In root, items are folders
        let root_items = app.current_items();
        assert!(!root_items.is_empty());
        match &root_items[0] {
            ViewItem::Folder { name, count } => {
                assert!(*count > 0);
                let first_folder = name.clone();
                app.enter_selected();
                assert_eq!(app.current_folder.as_deref(), Some(first_folder.as_str()));
            }
            _ => panic!("Expected ViewItem::Folder in root"),
        }

        // Inside folder, first item is Back
        let folder_items = app.current_items();
        assert_eq!(folder_items[0], ViewItem::Back);

        // Enter on Back goes back to root
        app.selected_action_idx = 0;
        app.enter_selected();
        assert!(app.current_folder.is_none());
    }

    #[test]
    fn test_install_dialog_lifecycle() {
        let mut app = create_test_app();
        assert!(app.install_dialog.is_none());

        app.start_zapret_install_dialog();
        assert_eq!(app.install_dialog, Some(InstallDialogState::ChooseOption { selected: 0 }));

        app.confirm_install_path("C:\\custom_zapret");
        assert!(app.install_dialog.is_none());
        assert_eq!(app.config.zapret.as_ref().and_then(|z| z.path.as_deref()), Some("C:\\custom_zapret"));
    }

    #[test]
    fn test_fuzzy_filtering() {
        let mut app = create_test_app();

        // No query: all actions returned
        assert_eq!(app.filtered_actions().len(), app.config.actions.len());

        // Filter by "cargo"
        app.input_buffer = "cargo".to_string();
        let filtered = app.filtered_actions();
        assert!(!filtered.is_empty());
        for (_, action) in &filtered {
            let matches = action.name.to_lowercase().contains("cargo")
                || action.command.to_lowercase().contains("cargo")
                || action.category.to_lowercase().contains("cargo");
            assert!(matches);
        }

        // Filter by non-existent text
        app.input_buffer = "non_existent_query_xyz_123".to_string();
        assert_eq!(app.filtered_actions().len(), 0);
    }

    #[test]
    fn test_execution_and_history() {
        let mut app = create_test_app();

        let test_action = ActionItem {
            id: "test-echo".to_string(),
            name: "Test Echo".to_string(),
            description: "Echo test".to_string(),
            category: "Test".to_string(),
            command: "echo dark_test_output_123".to_string(),
            cwd: None,
            shortcut: None,
            env: None,
        };

        app.execute_action(&test_action);

        assert_eq!(app.active_tab, Tab::Output);
        assert!(app.last_result.is_some());

        let res = app.last_result.as_ref().unwrap();
        assert!(res.success);
        assert_eq!(res.exit_code, Some(0));
        assert!(res.output.contains("dark_test_output_123"));

        assert_eq!(app.history.len(), 1);
        assert_eq!(app.history[0].id, "test-echo");
    }

    #[test]
    fn test_slash_commands() {
        let mut app = create_test_app();

        // Test /help
        app.handle_slash_command("/help");
        assert_eq!(app.active_tab, Tab::Help);

        // Test /clear
        app.last_result = Some(ExecutionResult {
            id: "dummy".to_string(),
            name: "dummy".to_string(),
            command: "dummy".to_string(),
            output: "dummy output".to_string(),
            exit_code: Some(0),
            duration: std::time::Duration::from_millis(10),
            timestamp: chrono::Local::now(),
            success: true,
        });
        assert!(app.last_result.is_some());
        app.handle_slash_command("/clear");
        assert!(app.last_result.is_none());

        // Test /exit
        assert!(!app.should_quit);
        app.handle_slash_command("/exit");
        assert!(app.should_quit);
    }
}
