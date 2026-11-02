mod art;
mod panels;

use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    panels::draw(frame, app);
}
