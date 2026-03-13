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
    selected_index: Option<usize>,
    script_name: String,
    should_quit: bool,
    selected_output: Option<String>,
    last_input_time: Instant,
    pending_update: bool,
    last_command: String,
    is_loading: bool,
    debug_mode: bool,  // Новый флаг для debug режима
}

const INPUT_DEBOUNCE_MS: u64 = 500;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Получаем аргументы командной строки
    let args: Vec<String> = std::env::args().collect();

    // Парсим флаги
    let mut script_name = None;
    let mut debug_mode = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-d" => {
                debug_mode = true;
                i += 1;
            }
            _ => {
                if script_name.is_none() {
                    script_name = Some(args[i].clone());
                    i += 1;
                } else {
                    eprintln!("Usage: {} [-d] <script_path>", args[0]);
                    std::process::exit(1);
                }
            }
        }
    }

    let script_name = match script_name {
        Some(name) => name,
        None => {
            eprintln!("Usage: {} [-d] <script_path>", args[0]);
            std::process::exit(1);
        }
    };

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
        last_command: String::new(),
        is_loading: false,
        debug_mode,
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
        if app.pending_update && !app.is_loading && last_update_check.elapsed() > Duration::from_millis(100) {
            if app.last_input_time.elapsed() > Duration::from_millis(INPUT_DEBOUNCE_MS) {
                app.is_loading = true;
                let input = app.input.clone();
                let script_name = app.script_name.clone();

                // Запускаем скрипт в отдельной задаче
                let script_future = run_script(&script_name, &input);
                let (new_output, cmd) = script_future.await;

                app.script_output = new_output;
                app.last_command = cmd;
                app.is_loading = false;
                app.pending_update = false;

                // Обновляем выбранный элемент
                update_selection(app);
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

fn update_selection(app: &mut App) {
    // Сохраняем текущий выбранный текст, если он есть
    let selected_text = app.selected_index.and_then(|i| app.script_output.get(i).cloned());

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

async fn handle_key(app: &mut App, key: KeyEvent) -> Result<(), Box<dyn std::error::Error>> {
    // Не обрабатываем клавиши во время загрузки
    if app.is_loading {
        return Ok(());
    }

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
    if app.is_loading {
        return Ok(());
    }

    if app.script_output.is_empty() {
        // Если ничего не загружено - загружаем принудительно
        app.is_loading = true;
        let input = app.input.clone();
        let script_name = app.script_name.clone();

        let (new_output, cmd) = run_script(&script_name, &input).await;

        app.script_output = new_output;
        app.last_command = cmd;
        app.is_loading = false;

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
    if app.script_output.is_empty() || app.is_loading {
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
    if app.script_output.is_empty() || app.is_loading {
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

async fn run_script(script_name: &str, input: &str) -> (Vec<String>, String) {
    let cmd = format!("{} {}", script_name, input);

    let mut child = match Command::new(script_name)
        .arg(input)
        .stdout(Stdio::piped())
        .spawn() {
            Ok(c) => c,
            Err(e) => {
                return (vec![format!("Error: Failed to execute script - {}", e)], cmd);
            }
        };

    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => return (vec!["Error: Failed to get script output".to_string()], cmd),
    };

    let reader = BufReader::new(stdout);
    let mut lines = Vec::new();

    let mut line_reader = reader.lines();
    while let Ok(Some(line)) = line_reader.next_line().await {
        lines.push(line);
    }

    let _ = child.wait().await;
    (lines, cmd)
}

fn ui(frame: &mut Frame, app: &App) {
    // В debug режиме показываем больше информации
    if app.debug_mode {
        ui_debug(frame, app)
    } else {
        ui_normal(frame, app)
    }
}

fn ui_normal(frame: &mut Frame, app: &App) {
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
                if Some(i) == app.selected_index {
                    ListItem::new(line.as_str()).style(Style::default().fg(Color::Yellow).bg(Color::DarkGray))
                } else {
                    ListItem::new(line.as_str())
                }
            })
            .collect();

        let list_title = format!("Results ({} items)", app.script_output.len());

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(list_title));
        frame.render_widget(list, chunks[1]);
    } else {
        let empty_list = List::new(vec![ListItem::new("No results - type to search...")])
            .block(Block::default().borders(Borders::ALL).title("Results"));
        frame.render_widget(empty_list, chunks[1]);
    }
}

fn ui_debug(frame: &mut Frame, app: &App) {
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
        if app.selected_index.is_some() { "clear selection" } else { "quit" }
    );

    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title(input_title));
    frame.render_widget(input, input_chunks[0]);

    // Статусная строка с последней командой или индикатором загрузки
    if app.is_loading {
        let loading = Paragraph::new("⏳ Loading...")
            .block(Block::default().borders(Borders::NONE))
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(loading, input_chunks[1]);
    } else if !app.last_command.is_empty() {
        let status = Paragraph::new(format!("🔄 Last command: {}", app.last_command))
            .block(Block::default().borders(Borders::NONE))
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(status, input_chunks[1]);
    }

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
            "Results ({} items, ↑/↓ or Ctrl+P/Ctrl+N to navigate, Enter/Ctrl+J to select)",
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
