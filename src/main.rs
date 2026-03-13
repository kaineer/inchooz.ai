mod app;
mod handlers;
mod ui;
mod utils;

use std::time::{Duration, Instant};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::Backend, Terminal};
use ratatui::prelude::CrosstermBackend;

use app::App;
use handlers::key::handle_key;
use ui::render;

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
    let mut app = App::new(script_name, debug_mode);

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

    while !app.should_quit() {
        terminal.draw(|f| render(f, app))?;

        // Проверяем, не прошло ли достаточно времени для обновления
        if app.pending_update() && !app.is_loading() && last_update_check.elapsed() > Duration::from_millis(100) {
            if app.last_input_time.elapsed() > Duration::from_millis(app::INPUT_DEBOUNCE_MS) {
                app.set_loading(true);
                let input = app.input().to_string();
                let script_name = app.script_name().to_string();

                // Запускаем скрипт в отдельной задаче
                let (new_output, cmd) = handlers::script::run_script(&script_name, &input).await;

                app.update_results(new_output, cmd);
                app.set_loading(false);
                app.set_pending_update(false);
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
