mod ocr;
mod portal;
mod storage;

pub mod models;

use std::time::Duration;
use std::sync::{Mutex, OnceLock};

use models::{LoginFailureKind, LoginOptions, LoginResponse, SavedCredentials};
use ocr::OcrEngine;
use portal::PortalClient;

const EMBEDDED_REC_ONNX: &[u8] = include_bytes!("../../resources/rec.onnx");
const EMBEDDED_DICT_TXT: &str = include_str!("../../resources/dict.txt");

static OCR_ENGINE: OnceLock<Result<Mutex<OcrEngine>, String>> = OnceLock::new();

fn shared_ocr_engine() -> Result<&'static Mutex<OcrEngine>, String> {
    OCR_ENGINE
        .get_or_init(|| {
            OcrEngine::from_embedded(EMBEDDED_REC_ONNX, EMBEDDED_DICT_TXT)
                .map(Mutex::new)
                .map_err(|e| e.to_string())
        })
        .as_ref()
        .map_err(Clone::clone)
}

pub async fn login_with_ocr(
    account: String,
    password: String,
    options: Option<LoginOptions>,
) -> Result<LoginResponse, String> {
    let opts = options.unwrap_or_default();
    let ocr_engine = shared_ocr_engine()?;

    let mut client =
        PortalClient::new(account, password, opts.timeout_secs).map_err(|e| e.to_string())?;
    if let Err(e) = client.init_session().await {
        return Ok(transport_failure_response(
            &e.to_string(),
            0,
            String::new(),
            0.0,
        ));
    }

    let mut last_error_message: Option<String> = None;
    let mut last_failure_kind: Option<LoginFailureKind> = None;
    let mut last_captcha_text = String::new();
    let mut last_confidence = 0.0_f32;

    for attempt in 1..=opts.max_login_retries {
        let image = match client.fetch_captcha_image().await {
            Ok(image) => image,
            Err(e) => {
                return Ok(transport_failure_response(
                    &e.to_string(),
                    attempt,
                    String::new(),
                    0.0,
                ));
            }
        };
        let ocr_result = {
            let mut engine = ocr_engine
                .lock()
                .map_err(|_| "ocr engine lock poisoned".to_string())?;
            engine
                .recognize(&image)
                .map_err(|e| format!("captcha OCR failed: {e}"))?
        };

        last_captcha_text = ocr_result.text.clone();
        last_confidence = ocr_result.confidence;

        if ocr_result.text.is_empty() {
            continue;
        }

        let submit = client
            .login(&ocr_result.text)
            .await;

        let submit = match submit {
            Ok(submit) => submit,
            Err(e) => {
                return Ok(transport_failure_response(
                    &e.to_string(),
                    attempt,
                    ocr_result.text,
                    ocr_result.confidence,
                ));
            }
        };

        if !submit.success {
            if let Some(msg) = submit.message {
                last_error_message = Some(msg.clone());
                last_failure_kind = submit.failure_kind.clone();

                if matches!(
                    submit.failure_kind,
                    Some(LoginFailureKind::InvalidCredentials)
                        | Some(LoginFailureKind::InvalidCredentialsOrLocked)
                        | Some(LoginFailureKind::AccountLocked)
                ) {
                    return Ok(failed_response(
                        msg,
                        ocr_result.text,
                        ocr_result.confidence,
                        attempt,
                        false,
                        last_failure_kind,
                    ));
                }
            }
        }

        if submit.success {
            if let Err(e) = client.post_login_sync().await {
                return Ok(transport_failure_response(
                    &format!("post login sync failed: {e}"),
                    attempt,
                    ocr_result.text,
                    ocr_result.confidence,
                ));
            }

            let probe_passed = client
                .probe_connectivity(3, Duration::from_millis(1200))
                .await
                .map_err(|e| e.to_string());

            let probe_passed = match probe_passed {
                Ok(passed) => passed,
                Err(e) => {
                    return Ok(transport_failure_response(
                        &e,
                        attempt,
                        ocr_result.text,
                        ocr_result.confidence,
                    ));
                }
            };

            if opts.probe_required && !probe_passed {
                return Ok(failed_response(
                    "登录成功但连通性检测失败".to_string(),
                    ocr_result.text,
                    ocr_result.confidence,
                    attempt,
                    probe_passed,
                    Some(LoginFailureKind::ConnectivityProbeFailed),
                ));
            }

            return Ok(LoginResponse {
                success: true,
                message: "登录成功".to_string(),
                captcha_text: ocr_result.text,
                confidence: ocr_result.confidence,
                attempt,
                probe_passed,
                failure_kind: None,
            });
        }

        if attempt < opts.max_login_retries {
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    let message = last_error_message.unwrap_or_else(|| "登录失败，已达到最大重试次数".to_string());
    let failure_kind = last_failure_kind.or(Some(LoginFailureKind::MaxRetriesExceeded));

    Ok(failed_response(
        message,
        last_captcha_text,
        last_confidence,
        opts.max_login_retries,
        false,
        failure_kind,
    ))
}

fn transport_failure_response(
    raw_error: &str,
    attempt: u32,
    captcha_text: String,
    confidence: f32,
) -> LoginResponse {
    let (failure_kind, message) = classify_transport_failure(raw_error);
    failed_response(message, captcha_text, confidence, attempt, false, Some(failure_kind))
}

fn classify_transport_failure(raw_error: &str) -> (LoginFailureKind, String) {
    let lowered = raw_error.to_lowercase();

    let network_unavailable = lowered.contains("network is unreachable")
        || lowered.contains("no route to host")
        || lowered.contains("dns error")
        || lowered.contains("failed to lookup address information")
        || lowered.contains("temporary failure in name resolution");

    if network_unavailable {
        return (
            LoginFailureKind::NetworkUnavailable,
            "当前网络不可用，请检查网络连接后重试".to_string(),
        );
    }

    let portal_unreachable = lowered.contains("connection refused")
        || lowered.contains("timed out")
        || lowered.contains("deadline has elapsed")
        || lowered.contains("endpoint=");

    if portal_unreachable {
        return (
            LoginFailureKind::PortalPageUnreachable,
            "认证网页暂时无法访问，请确认校园网环境或稍后重试".to_string(),
        );
    }

    (
        LoginFailureKind::Unknown,
        format!("网络请求失败: {raw_error}"),
    )
}

fn failed_response(
    message: String,
    captcha_text: String,
    confidence: f32,
    attempt: u32,
    probe_passed: bool,
    failure_kind: Option<LoginFailureKind>,
) -> LoginResponse {
    LoginResponse {
        success: false,
        message,
        captcha_text,
        confidence,
        attempt,
        probe_passed,
        failure_kind,
    }
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
