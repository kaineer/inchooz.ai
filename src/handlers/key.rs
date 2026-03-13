use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::app::App;
use super::script;

pub async fn handle_key(app: &mut App, key: KeyEvent) -> Result<(), Box<dyn std::error::Error>> {
    // Не обрабатываем клавиши во время загрузки
    if app.is_loading() {
        return Ok(());
    }

    match (key.code, key.modifiers) {
        // Обычные символы - добавляем ввод
        (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
            app.push_char(c);
        }

        // Backspace
        (KeyCode::Backspace, _) => {
            app.pop_char();
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
            app.select_next();
        }

        // Стрелка вверх или Ctrl+P - предыдущий элемент
        (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
            app.select_previous();
        }

        // Esc - выход или сброс выбора
        (KeyCode::Esc, _) => {
            if app.has_selection() {
                // Если что-то выбрано - сначала сбрасываем выбор
                app.clear_selection();
            } else {
                // Иначе выходим
                app.quit();
            }
        }

        _ => {}
    }
    Ok(())
}

async fn handle_enter(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    if app.is_loading() {
        return Ok(());
    }

    if !app.has_results() {
        // Если ничего не загружено - загружаем принудительно
        app.set_loading(true);
        let input = app.input().to_string();
        let script_name = app.script_name().to_string();

        let (new_output, cmd) = script::run_script(&script_name, &input).await;

        app.update_results(new_output, cmd);
        app.set_loading(false);

        if app.has_results() {
            app.select_first();
        }
    } else if let Some(selected) = app.select_current() {
        // Если что-то выбрано - выбираем этот элемент
        app.selected_output = Some(selected);
        app.quit();
    } else {
        // Если ничего не выбрано - выбираем первый элемент
        app.select_first();
    }
    Ok(())
}
