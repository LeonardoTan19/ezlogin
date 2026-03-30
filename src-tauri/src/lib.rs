use ezlogin_core::models::{LoginOptions, LoginResponse, SavedCredentials};

#[tauri::command]
async fn portal_login_with_ocr(
    account: String,
    password: String,
    options: Option<LoginOptions>,
) -> Result<LoginResponse, String> {
    ezlogin_core::login_with_ocr(account, password, options).await
}

#[tauri::command]
fn save_credentials(account: String, password: String) -> Result<(), String> {
    ezlogin_core::save_credentials(&account, &password)
}

#[tauri::command]
fn load_saved_credentials() -> Result<Option<SavedCredentials>, String> {
    ezlogin_core::load_credentials()
}

#[tauri::command]
fn clear_saved_credentials() -> Result<(), String> {
    ezlogin_core::clear_credentials()
}

#[tauri::command]
fn save_login_options(options: LoginOptions) -> Result<(), String> {
    ezlogin_core::save_login_options(&options)
}

#[tauri::command]
fn load_login_options() -> Result<Option<LoginOptions>, String> {
    ezlogin_core::load_login_options()
}

#[tauri::command]
fn is_mobile_platform() -> bool {
    cfg!(any(target_os = "android", target_os = "ios"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            portal_login_with_ocr,
            save_credentials,
            load_saved_credentials,
            clear_saved_credentials,
            save_login_options,
            load_login_options,
            is_mobile_platform,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
