fn unsupported<T>() -> Result<T, String> {
    Err("PDF import is not available in desktop mode.".to_string())
}

#[tauri::command]
pub async fn get_pdf_imports_staged() -> Result<Vec<serde_json::Value>, String> {
    unsupported()
}

#[tauri::command]
pub async fn get_pdf_import_detail(id: String) -> Result<serde_json::Value, String> {
    let _ = id;
    unsupported()
}

#[tauri::command]
pub async fn delete_pdf_import_staged(id: String) -> Result<(), String> {
    let _ = id;
    unsupported()
}

#[tauri::command]
pub async fn confirm_pdf_import(
    id: String,
    request: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = (id, request);
    unsupported()
}

#[tauri::command]
pub async fn check_pdf_import(
    id: String,
    request: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = (id, request);
    unsupported()
}
