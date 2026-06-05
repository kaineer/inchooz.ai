use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph},
};

use crate::app::App;

pub fn render_normal(frame: &mut Frame, app: &App) {
    // Получаем всю область терминала
    let area = frame.size();

    // Создаем одинарную рамку на всю область
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .padding(Padding {
            left: 1,
            right: 1,
            top: 0,
            bottom: 0,
        })
        .border_style(Style::default().fg(Color::White));

    // Внутренняя область (внутри рамки)
    let inner_area = outer_block.inner(area);

    // Отрисовываем рамку
    frame.render_widget(outer_block, area);

    // Разделяем внутреннюю область на 3 части:
    // строка ввода, разделитель, список результатов
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Строка ввода
            Constraint::Length(1), // Разделитель
            Constraint::Min(1),    // Список результатов
        ])
        .margin(0) // Отступы от рамки
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

    // 3. Список результатов от скрипта
    if app.has_results() {
        let items: Vec<ListItem> = app
            .script_output()
            .iter()
            .enumerate()
            .map(|(i, line)| {
                // Показываем только непустые строки
                let display_line = if line.is_empty() { " " } else { line.as_str() };

                if Some(i) == app.selected_index() {
                    // Выделенный элемент
                    ListItem::new(display_line).style(
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    // Обычный элемент
                    ListItem::new(display_line).style(Style::default().fg(Color::White))
                }
            })
            .collect();

        let list = List::new(items)
            .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black))
            .highlight_symbol("> ");

        frame.render_widget(list, chunks[2]);
    } else {
        // Нет результатов - показываем подсказку
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

        let hint_paragraph = Paragraph::new(Span::styled(hint, hint_style)).style(Style::default());
        frame.render_widget(hint_paragraph, chunks[2]);
    }
}
