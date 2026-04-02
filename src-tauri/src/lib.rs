use ezlogin_core::models::{LoginOptions, LoginResponse, SavedCredentials};
#[cfg(any(target_os = "windows", target_os = "android", target_os = "macos"))]
use std::process::Command;

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

#[tauri::command]
fn open_network_settings() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", "ms-settings:network"])
            .spawn()
            .map_err(|e| format!("无法打开 Windows 网络设置: {e}"))?;
        return Ok(());
    }

    #[cfg(target_os = "android")]
    {
        Command::new("am")
            .args(["start", "-a", "android.settings.WIFI_SETTINGS"])
            .spawn()
            .map_err(|e| format!("无法打开 Android 网络设置: {e}"))?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        return Err(
            "Linux CLI 模式请手动检测连通性：ping -c 1 192.168.200.127；curl -I --max-time 5 http://www.msftconnecttest.com/redirect"
                .to_string(),
        );
    }

    #[allow(unreachable_code)]
    Err("当前系统暂不支持自动打开网络设置".to_string())
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
            open_network_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
