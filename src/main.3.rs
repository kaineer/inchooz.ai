use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers},
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
use ratatui::prelude::CrosstermBackend;

struct App {
    input: String,
    script_output: Vec<String>,
    selected_index: usize,
    mode: Mode,
    script_name: String,
    should_quit: bool,
    selected_output: Option<String>,
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
        should_quit: false,
        selected_output: None,
    };

    // Запускаем основной цикл
    let result = run_app(&mut terminal, &mut app).await;

    // Восстанавливаем терминал перед выходом
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Выводим выбранную строку, если она есть
    if let Some(selected) = app.selected_output {
        println!("{}", selected);
    }

    if let Err(err) = result {
        println!("Error: {:?}", err);
    }

    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    while !app.should_quit {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            match app.mode {
                Mode::Input => handle_input_mode(app, key).await?,
                Mode::Selecting => handle_selection_mode(app, key)?,
            }
        }
    }
    Ok(())
}

async fn handle_input_mode(app: &mut App, key: KeyEvent) -> Result<(), Box<dyn std::error::Error>> {
    match (key.code, key.modifiers) {
        // Enter или Ctrl+J - запуск скрипта
        (KeyCode::Enter, _) | (KeyCode::Char('j'), KeyModifiers::CONTROL) => {
            // Запускаем скрипт с введенным текстом
            app.script_output = run_script(&app.script_name, &app.input).await;
            if !app.script_output.is_empty() {
                app.mode = Mode::Selecting;
                app.selected_index = 0;
            }
        }
        (KeyCode::Char(c), _) => {
            app.input.push(c);
        }
        (KeyCode::Backspace, _) => {
            app.input.pop();
        }
        (KeyCode::Esc, _) => {
            // Выход из приложения
            app.should_quit = true;
        }
        _ => {}
    }
    Ok(())
}

fn handle_selection_mode(app: &mut App, key: KeyEvent) -> Result<(), Box<dyn std::error::Error>> {
    match (key.code, key.modifiers) {
        // Стрелка вверх или Ctrl+P - предыдущий элемент
        (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
            if app.selected_index > 0 {
                app.selected_index -= 1;
            }
        }

        // Стрелка вниз или Ctrl+N - следующий элемент
        (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
            if app.selected_index < app.script_output.len().saturating_sub(1) {
                app.selected_index += 1;
            }
        }

        // Enter или Ctrl+J - выбор текущего элемента
        (KeyCode::Enter, _) | (KeyCode::Char('j'), KeyModifiers::CONTROL) => {
            if let Some(selected) = app.script_output.get(app.selected_index) {
                app.selected_output = Some(selected.clone());
            }
            app.should_quit = true;
        }

        // Esc - возврат к вводу
        (KeyCode::Esc, _) => {
            app.mode = Mode::Input;
            app.script_output.clear();
        }

        // Игнорируем все остальные клавиши
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

fn ui(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
        .split(frame.size());

    // Поле ввода с подсказкой
    let input_title = if matches!(app.mode, Mode::Input) {
        "Input (Enter or Ctrl+J to run script, Esc to quit)"
    } else {
        "Input"
    };

    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title(input_title));
    frame.render_widget(input, chunks[0]);

    // Область вывода скрипта
    if !app.script_output.is_empty() {
        let items: Vec<ListItem> = app.script_output
            .iter()
            .enumerate()
            .map(|(i, line)| {
                if i == app.selected_index && matches!(app.mode, Mode::Selecting) {
                    ListItem::new(line.as_str()).style(Style::default().fg(Color::Yellow))
                } else {
                    ListItem::new(line.as_str())
                }
            })
            .collect();

        let list_title = if matches!(app.mode, Mode::Selecting) {
            "Script Output (↑/↓ or Ctrl+P/Ctrl+N, Enter or Ctrl+J to select, Esc to go back)"
        } else {
            "Script Output"
        };

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(list_title));
        frame.render_widget(list, chunks[1]);
    } else if matches!(app.mode, Mode::Selecting) {
        // Если режим Selecting но нет вывода, возвращаемся в Input
        let empty_list = List::new(vec![ListItem::new("No output from script")])
            .block(Block::default().borders(Borders::ALL).title("Script Output"));
        frame.render_widget(empty_list, chunks[1]);
    }
}
