mod normal;
mod debug;

// use ratatui::{Frame, layout::{Layout, Constraint, Direction}};
use ratatui::{Frame};
use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    if app.debug_mode {
        debug::render_debug(frame, app);
    } else {
        normal::render_normal(frame, app);
    }
}
