use serde::Deserialize;

#[derive(Deserialize)]
pub struct LogSendRequest {
    pub level: String,
    pub module: String,
    pub message: String,
    pub context: Option<serde_json::Value>,
    pub error: Option<LogError>,
}

#[derive(Deserialize)]
pub struct LogError {
    pub name: String,
    pub message: String,
    pub stack: Option<String>,
}

/// Renderer → tauri-plugin-log. Persistence and rotation live in Rust, so the
/// webview never touches the filesystem for logging.
#[tauri::command]
pub fn log_send(data: LogSendRequest) {
    let target = data.module.as_str();
    let message = match (&data.context, &data.error) {
        (Some(context), _) => format!("{} {context}", data.message),
        (None, Some(error)) => match error.stack.as_deref() {
            Some(stack) => format!(
                "{} [{}: {}]\n{stack}",
                data.message, error.name, error.message
            ),
            None => format!("{} [{}: {}]", data.message, error.name, error.message),
        },
        (None, None) => data.message.clone(),
    };

    match data.level.as_str() {
        "debug" => ::log::debug!(target: target, "{message}"),
        "warn" => ::log::warn!(target: target, "{message}"),
        "error" => ::log::error!(target: target, "{message}"),
        "fatal" => ::log::error!(target: target, "{message}"),
        _ => ::log::info!(target: target, "{message}"),
    }
}
