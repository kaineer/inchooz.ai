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
        .border_style(Style::default().fg(Color::DarkGray))
        .padding(Padding {
            left: 1,
            right: 1,
            top: 0,
            bottom: 0,
        });

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
            Constraint::Min(1),    // Список результатов
        ])
        .margin(0)
        .split(inner_area);

    // 1. Строка ввода с префиксом "> " и курсором
    let input_prefix = if app.is_loading() { "⏳ " } else { "> " };

    // Символ курсора (вертикальная черта или подчеркивание)
    let cursor_char = if app.is_loading() { '░' } else { ' ' };

    // Позиция курсора в строке ввода (после всего текста)
    let cursor_pos = app.input().len();

    // Формируем строку с визуальным курсором
    let mut display_text = format!("{}{}", input_prefix, app.input());
    display_text.push(cursor_char);

    let input_style = if app.is_loading() {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };

    let input_paragraph = Paragraph::new(Span::styled(display_text, input_style));
    frame.render_widget(input_paragraph, chunks[0]);

    // Устанавливаем позицию системного курсора в терминале
    let cursor_x = chunks[0].x + (input_prefix.len() + cursor_pos) as u16;
    let cursor_y = chunks[0].y;
    frame.set_cursor(cursor_x, cursor_y);

    // 2. Горизонтальная разделительная линия
    let separator = "─".repeat(chunks[1].width as usize);
    let separator_style = Style::default().fg(Color::DarkGray);
    let separator_paragraph = Paragraph::new(Span::styled(separator, separator_style));
    frame.render_widget(separator_paragraph, chunks[1]);

    // 3. Список результатов
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
                            .fg(Color::White)
                            .bg(Color::Rgb((0), (0), (0)))
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

        frame.render_widget(list, chunks[2]);
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
        frame.render_widget(hint_paragraph, chunks[2]);
    }
}
