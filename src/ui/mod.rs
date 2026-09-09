pub mod actions_view;
pub mod header;
pub mod help_view;
pub mod history_view;
pub mod input_bar;
pub mod output_view;
pub mod palette;
pub mod theme;

use crate::app::{App, InputMode, Tab};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

pub fn render(f: &mut Frame, app: &App) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(5),    // Main Content
            Constraint::Length(3), // Bottom Input/Hint Bar
        ])
        .split(size);

    // 1. Header
    header::render_header(f, app, chunks[0]);

    // 2. Main Body
    match app.active_tab {
        Tab::Actions => actions_view::render_actions_view(f, app, chunks[1]),
        Tab::Output => output_view::render_output_view(f, app, chunks[1]),
        Tab::History => history_view::render_history_view(f, app, chunks[1]),
        Tab::Help => help_view::render_help_view(f, app, chunks[1]),
    }

    // 3. Bottom Bar
    input_bar::render_input_bar(f, app, chunks[2]);

    // 4. Modal Overlays
    if app.input_mode == InputMode::PaletteSearch {
        palette::render_palette(f, app, size);
    }
}
