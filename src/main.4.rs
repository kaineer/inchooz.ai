use std::process::Stdio;
use std::time::{Duration, Instant};
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
    selected_index: Option<usize>,  // Теперь Option - может быть ничего не выбрано
    script_name: String,
    should_quit: bool,
    selected_output: Option<String>,
    last_input_time: Instant,
    pending_update: bool,
}

const INPUT_DEBOUNCE_MS: u64 = 500;

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
        selected_index: None,
        script_name,
        should_quit: false,
        selected_output: None,
        last_input_time: Instant::now(),
        pending_update: false,
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
    let mut last_update_check = Instant::now();

    while !app.should_quit {
        terminal.draw(|f| ui(f, app))?;

        // Проверяем, не прошло ли достаточно времени для обновления
        if app.pending_update && last_update_check.elapsed() > Duration::from_millis(100) {
            if app.last_input_time.elapsed() > Duration::from_millis(INPUT_DEBOUNCE_MS) {
                update_script_output(app).await;
                app.pending_update = false;
            }
            last_update_check = Instant::now();
        }

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                handle_key(app, key).await?;
            }
        }
    }
    Ok(())
}

async fn handle_key(app: &mut App, key: KeyEvent) -> Result<(), Box<dyn std::error::Error>> {
    match (key.code, key.modifiers) {
        // Обычные символы - добавляем ввод
        (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
            app.input.push(c);
            app.last_input_time = Instant::now();
            app.pending_update = true;
            // При изменении ввода сбрасываем выделение, если список пуст
            if app.script_output.is_empty() {
                app.selected_index = None;
            }
        }

        // Backspace
        (KeyCode::Backspace, _) => {
            app.input.pop();
            app.last_input_time = Instant::now();
            app.pending_update = true;
            if app.script_output.is_empty() {
                app.selected_index = None;
            }
        }

        // Ctrl+J - выполнить действие (как Enter)
        (KeyCode::Char('j'), KeyModifiers::CONTROL) => {
            handle_enter(app).await?;
        }

        // Enter - выполнить действие
        (KeyCode::Enter, _) => {
            handle_enter(app).await?;
        }

        // Стрелка вниз или Ctrl+N - следующий элемент
        (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
            handle_down(app);
        }

        // Стрелка вверх или Ctrl+P - предыдущий элемент
        (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
            handle_up(app);
        }

        // Esc - выход или сброс выбора
        (KeyCode::Esc, _) => {
            if app.selected_index.is_some() {
                // Если что-то выбрано - сначала сбрасываем выбор
                app.selected_index = None;
            } else {
                // Иначе выходим
                app.should_quit = true;
            }
        }

        _ => {}
    }
    Ok(())
}

async fn handle_enter(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    if app.script_output.is_empty() {
        // Если ничего не загружено - загружаем принудительно
        update_script_output(app).await;
        if !app.script_output.is_empty() {
            app.selected_index = Some(0);
        }
    } else if let Some(index) = app.selected_index {
        // Если что-то выбрано - выбираем этот элемент
        if let Some(selected) = app.script_output.get(index) {
            app.selected_output = Some(selected.clone());
            app.should_quit = true;
        }
    } else {
        // Если ничего не выбрано - выбираем первый элемент
        if !app.script_output.is_empty() {
            app.selected_index = Some(0);
        }
    }
    Ok(())
}

fn handle_down(app: &mut App) {
    if app.script_output.is_empty() {
        return;
    }

    match app.selected_index {
        None => {
            // Если ничего не выбрано - выбираем первый
            app.selected_index = Some(0);
        }
        Some(i) if i < app.script_output.len() - 1 => {
            // Если не последний - двигаемся вниз
            app.selected_index = Some(i + 1);
        }
        _ => {} // На последнем элементе ничего не делаем
    }
}

fn handle_up(app: &mut App) {
    if app.script_output.is_empty() {
        return;
    }

    match app.selected_index {
        None => {
            // Если ничего не выбрано - выбираем последний
            if !app.script_output.is_empty() {
                app.selected_index = Some(app.script_output.len() - 1);
            }
        }
        Some(i) if i > 0 => {
            // Если не первый - двигаемся вверх
            app.selected_index = Some(i - 1);
        }
        _ => {} // На первом элементе ничего не делаем
    }
}

async fn update_script_output(app: &mut App) {
    let new_output = run_script(&app.script_name, &app.input).await;

    // Сохраняем текущий выбранный текст, если он есть
    let selected_text = app.selected_index.and_then(|i| app.script_output.get(i).cloned());

    app.script_output = new_output;

    // Обновляем позицию выбранного элемента, если он все еще существует
    if let Some(text) = selected_text {
        if let Some(new_index) = app.script_output.iter().position(|s| s == &text) {
            app.selected_index = Some(new_index);
        } else {
            // Если старый выбранный элемент исчез - сбрасываем выбор
            app.selected_index = None;
        }
    } else {
        app.selected_index = None;
    }
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
    let input_title = format!(
        "Input (type to search, Enter/Ctrl+J to select, Esc: {})",
        if app.selected_index.is_some() { "clear selection" } else { "quit" }
    );

    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title(input_title));
    frame.render_widget(input, chunks[0]);

    // Область вывода скрипта
    if !app.script_output.is_empty() {
        let items: Vec<ListItem> = app.script_output
            .iter()
            .enumerate()
            .map(|(i, line)| {
                if Some(i) == app.selected_index {
                    ListItem::new(line.as_str()).style(Style::default().fg(Color::Yellow).bg(Color::DarkGray))
                } else {
                    ListItem::new(line.as_str())
                }
            })
            .collect();

        let list_title = format!(
            "Results ({} items, ↑/↓ or Ctrl+P/Ctrl+N to navigate)",
            app.script_output.len()
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
