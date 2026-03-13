use ratatui::{
    Frame,
    layout::{Layout, Constraint, Direction},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    style::{Style, Color},
};

use crate::app::App;

pub fn render_normal(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
        .split(frame.size());

    // Поле ввода
    let input = Paragraph::new(app.input())
        .block(Block::default().borders(Borders::ALL).title("Input"));
    frame.render_widget(input, chunks[0]);

    // Область вывода скрипта
    if app.has_results() {
        let items: Vec<ListItem> = app.script_output()
            .iter()
            .enumerate()
            .map(|(i, line)| {
                if Some(i) == app.selected_index() {
                    ListItem::new(line.as_str()).style(Style::default().fg(Color::Yellow).bg(Color::DarkGray))
                } else {
                    ListItem::new(line.as_str())
                }
            })
            .collect();

        let list_title = format!("Results ({} items)", app.script_output().len());

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(list_title));
        frame.render_widget(list, chunks[1]);
    } else {
        let empty_list = List::new(vec![ListItem::new("No results - type to search...")])
            .block(Block::default().borders(Borders::ALL).title("Results"));
        frame.render_widget(empty_list, chunks[1]);
    }
}
