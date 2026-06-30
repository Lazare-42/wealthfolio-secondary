fn unsupported<T>() -> Result<T, String> {
    Err("Statement reconciliation is not available in desktop mode.".to_string())
}

#[tauri::command]
pub async fn reconciliation_scan() -> Result<serde_json::Value, String> {
    unsupported()
}

#[tauri::command]
pub async fn reconciliation_pending() -> Result<Vec<serde_json::Value>, String> {
    unsupported()
}

#[tauri::command]
pub async fn reconciliation_detail(run_id: String) -> Result<serde_json::Value, String> {
    let _ = run_id;
    unsupported()
}

#[tauri::command]
pub async fn reconciliation_resolve(
    request: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = request;
    unsupported()
}

#[tauri::command]
pub async fn get_reconciliation_config() -> Result<serde_json::Value, String> {
    unsupported()
}

#[tauri::command]
pub async fn update_reconciliation_config(
    config: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = config;
    unsupported()
}
