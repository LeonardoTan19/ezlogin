use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrResult {
    pub text: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
    pub captcha_text: String,
    pub confidence: f32,
    pub attempt: u32,
    pub probe_passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedCredentials {
    pub account: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginOptions {
    pub max_login_retries: u32,
    pub probe_required: bool,
    pub timeout_secs: u64,
}

impl Default for LoginOptions {
    fn default() -> Self {
        Self {
            max_login_retries: 5,
            probe_required: false,
            timeout_secs: 10,
        }
    }
}
