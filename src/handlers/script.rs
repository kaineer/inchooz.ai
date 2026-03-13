use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};

pub async fn run_script(script_name: &str, input: &str) -> (Vec<String>, String) {
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
