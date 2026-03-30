mod ocr;
mod portal;
mod storage;

pub mod models;

use std::time::Duration;

use models::{LoginOptions, LoginResponse, SavedCredentials};
use ocr::OcrEngine;
use portal::PortalClient;

const EMBEDDED_REC_ONNX: &[u8] = include_bytes!("../../resources/rec.onnx");
const EMBEDDED_DICT_TXT: &str = include_str!("../../resources/dict.txt");

pub async fn login_with_ocr(
    account: String,
    password: String,
    options: Option<LoginOptions>,
) -> Result<LoginResponse, String> {
    let opts = options.unwrap_or_default();
    let mut ocr_engine =
        OcrEngine::from_embedded(EMBEDDED_REC_ONNX, EMBEDDED_DICT_TXT).map_err(|e| e.to_string())?;

    let mut client =
        PortalClient::new(account, password, opts.timeout_secs).map_err(|e| e.to_string())?;
    client.init_session().await.map_err(|e| e.to_string())?;

    let mut last_error_message: Option<String> = None;

    for attempt in 1..=opts.max_login_retries {
        let image = client.fetch_captcha_image().await.map_err(|e| e.to_string())?;
        let ocr_result = ocr_engine
            .recognize(&image)
            .map_err(|e| format!("captcha OCR failed: {e}"))?;

        if ocr_result.text.is_empty() {
            continue;
        }

        let submit = client
            .login(&ocr_result.text)
            .await
            .map_err(|e| e.to_string())?;

        if !submit.success {
            if let Some(msg) = submit.message {
                last_error_message = Some(msg.clone());

                if msg.contains("密码错误") || msg.contains("用户名或密码错误") {
                    return Ok(LoginResponse {
                        success: false,
                        message: msg,
                        captcha_text: ocr_result.text,
                        confidence: ocr_result.confidence,
                        attempt,
                        probe_passed: false,
                    });
                }
            }
        }

        if submit.success {
            client
                .post_login_sync()
                .await
                .map_err(|e| format!("post login sync failed: {e}"))?;

            let probe_passed = client
                .probe_connectivity(3, Duration::from_millis(1200))
                .await
                .map_err(|e| e.to_string())?;

            if opts.probe_required && !probe_passed {
                return Ok(LoginResponse {
                    success: false,
                    message: "登录成功但连通性检测失败".to_string(),
                    captcha_text: ocr_result.text,
                    confidence: ocr_result.confidence,
                    attempt,
                    probe_passed,
                });
            }

            return Ok(LoginResponse {
                success: true,
                message: "登录成功".to_string(),
                captcha_text: ocr_result.text,
                confidence: ocr_result.confidence,
                attempt,
                probe_passed,
            });
        }

        if attempt < opts.max_login_retries {
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    Ok(LoginResponse {
        success: false,
        message: last_error_message.unwrap_or_else(|| "登录失败，已达到最大重试次数".to_string()),
        captcha_text: String::new(),
        confidence: 0.0,
        attempt: opts.max_login_retries,
        probe_passed: false,
    })
}

pub fn save_credentials(account: &str, password: &str) -> Result<(), String> {
    storage::save_credentials(account, password)
}

pub fn load_credentials() -> Result<Option<SavedCredentials>, String> {
    storage::load_credentials()
}

pub fn clear_credentials() -> Result<(), String> {
    storage::clear_credentials()
}

pub fn save_login_options(options: &LoginOptions) -> Result<(), String> {
    storage::save_login_options(options)
}

pub fn load_login_options() -> Result<Option<LoginOptions>, String> {
    storage::load_login_options()
}
