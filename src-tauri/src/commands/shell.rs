#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    // Validate: only allow http/https URLs
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("Only http/https URLs are allowed".into());
    }
    opener::open(&url).map_err(|e| format!("Failed to open URL: {}", e))
}
