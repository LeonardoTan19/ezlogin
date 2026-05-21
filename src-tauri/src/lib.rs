use ezlogin_core::models::{LoginOptions, LoginResponse, SavedCredentials};
#[cfg(any(target_os = "windows", target_os = "macos"))]
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
        return open_wifi_settings_android();
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

#[cfg(target_os = "android")]
fn open_wifi_settings_android() -> Result<(), String> {
    use jni::objects::{JObject, JValue};
    use jni::JavaVM;

    let ctx = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }
        .map_err(|e| format!("获取 JavaVM 失败: {e}"))?;
    let activity = unsafe { JObject::from_raw(ctx.context().cast()) };
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("attach JNI 线程失败: {e}"))?;

    let action = env
        .new_string("android.settings.WIFI_SETTINGS")
        .map_err(|e| format!("构造 action 字符串失败: {e}"))?;
    let intent = env
        .new_object(
            "android/content/Intent",
            "(Ljava/lang/String;)V",
            &[(&action).into()],
        )
        .map_err(|e| format!("构造 Intent 失败: {e}"))?;
    env.call_method(
        &intent,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[JValue::Int(0x1000_0000)],
    )
    .map_err(|e| format!("addFlags 失败: {e}"))?;
    env.call_method(
        &activity,
        "startActivity",
        "(Landroid/content/Intent;)V",
        &[(&intent).into()],
    )
    .map_err(|e| format!("startActivity 失败: {e}"))?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|_app| {
            tauri::async_runtime::spawn_blocking(|| {
                let _ = ezlogin_core::init_ocr_engine();
            });
            Ok(())
        })
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
