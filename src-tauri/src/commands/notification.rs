use serde::{Deserialize, Serialize};
use tauri_plugin_notification::NotificationExt;

#[derive(Deserialize)]
pub struct NotificationShowRequest {
    pub title: String,
    pub body: Option<String>,
    pub silent: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationShowResponse {
    pub success: bool,
    pub shown: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[tauri::command]
pub fn notification_show(
    app: tauri::AppHandle,
    data: NotificationShowRequest,
) -> NotificationShowResponse {
    let mut builder = app.notification().builder().title(&data.title);
    if let Some(body) = data.body.as_deref() {
        builder = builder.body(body);
    }
    if data.silent.unwrap_or(false) {
        builder = builder.silent();
    }

    // `show` hands the notification to the OS and returns immediately. "Ok"
    // means it was dispatched, not that the system displayed it: macOS drops
    // notifications from an unbundled binary (i.e. `tauri dev`), which is why
    // the UI tells the user to try the built app.
    match builder.show() {
        Ok(()) => {
            ::log::info!("notification dispatched: {:?}", data.title);
            NotificationShowResponse {
                success: true,
                shown: true,
                error: None,
            }
        }
        Err(err) => {
            ::log::error!("notification failed: {err}");
            NotificationShowResponse {
                success: false,
                shown: false,
                error: Some(err.to_string()),
            }
        }
    }
}
