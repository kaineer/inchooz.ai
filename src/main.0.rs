use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::Backend,
    layout::{Layout, Constraint, Direction},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    style::{Style, Color},
    Frame, Terminal,
};
use ratatui::prelude::CrosstermBackend;  // Добавлен этот импорт
//
struct App {
    input: String,
    script_output: Vec<String>,
    selected_index: usize,
    mode: Mode,
    script_name: String,
}

enum Mode {
    Input,
    Selecting,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Получаем имя скрипта из аргументов командной строки
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <script_path>", args[0]);
        std::process::exit(1);
    }
    let script_name = args[1].clone();

    // Инициализация терминала
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Создаем приложение
    let mut app = App {
        input: String::new(),
        script_output: Vec::new(),
        selected_index: 0,
        mode: Mode::Input,
        script_name,
    };

    // Запускаем основной цикл
    let res = run_app(&mut terminal, &mut app).await;

    // Восстанавливаем терминал перед выходом
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            match app.mode {
                Mode::Input => handle_input_mode(app, key).await?,
                Mode::Selecting => handle_selection_mode(app, key)?,
            }
        }
    }
}

async fn handle_input_mode(app: &mut App, key: KeyEvent) -> Result<(), Box<dyn std::error::Error>> {
    match key.code {
        KeyCode::Enter => {
            // Запускаем скрипт с введенным текстом
            app.script_output = run_script(&app.script_name, &app.input).await;
            app.mode = Mode::Selecting;
            app.selected_index = 0;
        }
        KeyCode::Char(c) => {
            app.input.push(c);
        }
        KeyCode::Backspace => {
            app.input.pop();
        }
        KeyCode::Esc => {
            // Выход из приложения
            std::process::exit(0);
        }
        _ => {}
    }
    Ok(())
}

fn handle_selection_mode(app: &mut App, key: KeyEvent) -> Result<(), Box<dyn std::error::Error>> {
    match key.code {
        KeyCode::Up => {
            if app.selected_index > 0 {
                app.selected_index -= 1;
            }
        }
        KeyCode::Down => {
            if app.selected_index < app.script_output.len().saturating_sub(1) {
                app.selected_index += 1;
            }
        }
        KeyCode::Enter => {
            // Выводим выбранную строку в stdout и выходим
            if let Some(selected) = app.script_output.get(app.selected_index) {
                println!("{}", selected);
            }
            std::process::exit(0);
        }
        KeyCode::Esc => {
            // Возвращаемся к вводу
            app.mode = Mode::Input;
        }
        _ => {}
    }
    Ok(())
}

async fn run_script(script_name: &str, input: &str) -> Vec<String> {
    let mut child = match Command::new(script_name)
        .arg(input)
        .stdout(Stdio::piped())
        .spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to execute script: {}", e);
                return vec![format!("Error: Failed to execute script - {}", e)];
            }
        };

    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => return vec!["Error: Failed to get script output".to_string()],
    };

    let reader = BufReader::new(stdout);
    let mut lines = Vec::new();

    let mut line_reader = reader.lines();
    while let Ok(Some(line)) = line_reader.next_line().await {
        lines.push(line);
    }

    let _ = child.wait().await;
    lines
}

// Исправлено: убрана аннотация типа <B: Backend>
fn ui(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
        .split(frame.size());

    // Поле ввода
    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title("Input"));
    frame.render_widget(input, chunks[0]);

    // Область вывода скрипта
    if !app.script_output.is_empty() {
        let items: Vec<ListItem> = app.script_output
            .iter()
            .enumerate()
            .map(|(i, line)| {
                // Используем сам line, так как он уже содержит текст
                if i == app.selected_index && matches!(app.mode, Mode::Selecting) {
                    ListItem::new(line.as_str()).style(Style::default().fg(Color::Yellow))
                } else {
                    ListItem::new(line.as_str())
                }
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Script Output"));
        frame.render_widget(list, chunks[1]);
    }
}
