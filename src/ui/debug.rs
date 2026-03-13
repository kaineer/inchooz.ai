use ratatui::{
    Frame,
    layout::{Layout, Constraint, Direction},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    style::{Style, Color},
};

use crate::app::App;

pub fn render_debug(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(1)].as_ref())
        .split(frame.size());

    // Верхняя панель с вводом и статусом
    let input_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(2)].as_ref())
        .split(chunks[0]);

    // Поле ввода с подсказкой
    let input_title = format!(
        "Input (Esc: {})",
        if app.has_selection() { "clear selection" } else { "quit" }
    );

    let input = Paragraph::new(app.input())
        .block(Block::default().borders(Borders::ALL).title(input_title));
    frame.render_widget(input, input_chunks[0]);

    // Статусная строка с последней командой или индикатором загрузки
    if app.is_loading() {
        let loading = Paragraph::new("⏳ Loading...")
            .block(Block::default().borders(Borders::NONE))
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(loading, input_chunks[1]);
    } else if !app.last_command().is_empty() {
        let status = Paragraph::new(format!("🔄 Last command: {}", app.last_command()))
            .block(Block::default().borders(Borders::NONE))
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(status, input_chunks[1]);
    }

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

        let list_title = format!(
            "Results ({} items, ↑/↓ or Ctrl+P/Ctrl+N to navigate, Enter/Ctrl+J to select)",
            app.script_output().len()
        );

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(list_title));
        frame.render_widget(list, chunks[1]);
    } else {
        let empty_list = List::new(vec![ListItem::new("No results - type to search...")])
            .block(Block::default().borders(Borders::ALL).title("Results"));
        frame.render_widget(empty_list, chunks[1]);
    }
}
