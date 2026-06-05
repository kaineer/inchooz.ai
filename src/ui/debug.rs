use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph},
};

use crate::app::App;

pub fn render_debug(frame: &mut Frame, app: &App) {
    // Получаем всю область терминала
    let area = frame.size();

    // Создаем одинарную рамку на всю область
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .title(" Debug Mode (Ctrl+C to quit) ")
        .padding(Padding {
            left: 1,
            right: 1,
            top: 0,
            bottom: 0,
        })
        .title_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    // Внутренняя область (внутри рамки)
    let inner_area = outer_block.inner(area);

    // Отрисовываем рамку
    frame.render_widget(outer_block, area);

    // Разделяем внутреннюю область
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Строка ввода
            Constraint::Length(1), // Разделитель
            Constraint::Length(1), // Статус (для debug)
            Constraint::Min(1),    // Список результатов
        ])
        .margin(0)
        .split(inner_area);

    // 1. Строка ввода с префиксом "> "
    let input_text = format!("> {}", app.input());
    let input_style = if app.is_loading() {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };

    let input_paragraph =
        Paragraph::new(Span::styled(input_text, input_style)).style(Style::default());
    frame.render_widget(input_paragraph, chunks[0]);

    // 2. Горизонтальная разделительная линия
    let separator = "─".repeat(chunks[1].width as usize);
    let separator_style = Style::default().fg(Color::DarkGray);
    let separator_paragraph = Paragraph::new(Span::styled(separator, separator_style));
    frame.render_widget(separator_paragraph, chunks[1]);

    // 3. Статусная строка (debug информация)
    let status_text = if app.is_loading() {
        format!("⏳ Loading: {}", app.last_command())
    } else if !app.last_command().is_empty() {
        format!("🔄 Last command: {}", app.last_command())
    } else {
        "Ready".to_string()
    };

    let status_style = if app.is_loading() {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let status_paragraph = Paragraph::new(Span::styled(status_text, status_style));
    frame.render_widget(status_paragraph, chunks[2]);

    // 4. Список результатов
    if app.has_results() {
        let items: Vec<ListItem> = app
            .script_output()
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let display_line = if line.is_empty() { " " } else { line.as_str() };

                if Some(i) == app.selected_index() {
                    ListItem::new(display_line).style(
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    ListItem::new(display_line).style(Style::default().fg(Color::White))
                }
            })
            .collect();

        let list = List::new(items)
            .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black))
            .highlight_symbol("> ");

        frame.render_widget(list, chunks[3]);
    } else {
        let hint = if app.is_loading() {
            "⏳ Loading..."
        } else if app.input().is_empty() {
            "Type something and wait 500ms to see results..."
        } else {
            "No results - try typing something else..."
        };

        let hint_style = if app.is_loading() {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let hint_paragraph = Paragraph::new(Span::styled(hint, hint_style));
        frame.render_widget(hint_paragraph, chunks[3]);
    }
}
