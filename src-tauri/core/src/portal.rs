use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use crate::models::LoginFailureKind;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT, CONTENT_TYPE, ORIGIN, REFERER};
use reqwest::{Method, StatusCode};
use serde_json::Value;

pub struct LoginSubmitResult {
    pub success: bool,
    pub message: Option<String>,
    pub failure_kind: Option<LoginFailureKind>,
}

pub struct PortalClient {
    username: String,
    password: String,
    timeout: Duration,
    base_url: String,
    folder_name: String,
    custom_page_config_id: String,
    auth_url: String,
    success_url: String,
    xsrf_token: Option<String>,
    client: reqwest::Client,
}

impl PortalClient {
    pub fn new(username: String, password: String, timeout_secs: u64) -> Result<Self> {
        let base_url = "https://192.168.200.127:8445".to_string();
        let folder_name = "1606381611261/pc".to_string();
        let custom_page_config_id = "ff808081760371a1017603ce291b008d".to_string();

        let query = serde_urlencoded::to_string(vec![
            ("isPasscode", "N"),
            ("browserFlag", "zh"),
            ("folderName", folder_name.as_str()),
            ("httpsFlag", "Y"),
            ("publicBarcodeEncode", "null"),
            ("ssid", "edu_classroom"),
            ("url", "http://www.msftconnecttest.com/redirect"),
            ("authSuccess", "2"),
            ("redirectUrl", ""),
            ("urlParameter", "http://www.msftconnecttest.com/redirect"),
            ("currentTime", current_time_millis().as_str()),
            ("authislogoff", "true"),
        ])
        .context("failed to encode auth query")?;

        let auth_url = format!("{base_url}/PortalServer/customize/{folder_name}/auth.jsp?{query}");
        let success_url = format!("{base_url}/PortalServer/customize/{folder_name}/success.jsp?{query}");

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true)
            .http1_only()
            .cookie_store(true)
            .timeout(Duration::from_secs(timeout_secs))
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36 Edg/146.0.0.0",
            )
            .build()
            .context("failed to create reqwest client")?;

        Ok(Self {
            username,
            password,
            timeout: Duration::from_secs(timeout_secs),
            base_url,
            folder_name,
            custom_page_config_id,
            auth_url,
            success_url,
            xsrf_token: None,
            client,
        })
    }

    fn sync_xsrf_from_headers(&mut self, headers: &HeaderMap) {
        for value in headers.get_all("set-cookie") {
            if let Ok(raw) = value.to_str() {
                if let Some(rest) = raw.strip_prefix("XSRF-TOKEN=") {
                    let token = rest.split(';').next().unwrap_or_default().to_string();
                    if !token.is_empty() {
                        self.xsrf_token = Some(token);
                        break;
                    }
                }
            }
        }
    }

    async fn request(
        &mut self,
        method: Method,
        path_or_url: &str,
        referer: Option<&str>,
        mut headers: HeaderMap,
        form: Option<&[(&str, &str)]>,
    ) -> Result<reqwest::Response> {
        let url = if path_or_url.starts_with("http") {
            path_or_url.to_string()
        } else {
            format!("{}{}", self.base_url, path_or_url)
        };

        if let Some(referer) = referer {
            headers.insert(
                REFERER,
                HeaderValue::from_str(referer).context("invalid referer")?,
            );
        }
        if let Some(token) = &self.xsrf_token {
            headers.insert(
                HeaderName::from_static("x-xsrf-token"),
                HeaderValue::from_str(token).context("invalid xsrf token")?,
            );
        }

        #[cfg(target_os = "android")]
        let req = {
            let mut req = self.client.request(method.clone(), &url).headers(headers.clone());
            if let Some(form) = form {
                req = req.form(form);
            }
            req
        };

        #[cfg(not(target_os = "android"))]
        let req = {
            let mut req = self.client.request(method, &url).headers(headers);
            if let Some(form) = form {
                req = req.form(form);
            }
            req
        };

        let response = match req.send().await {
            Ok(response) => response,
            Err(primary_err) => {
                #[cfg(target_os = "android")]
                {
                    if let Some(fallback_url) = android_http_fallback_url(&url) {
                        let mut fallback_headers = headers.clone();

                        if let Some(origin) = header_to_string(&fallback_headers, ORIGIN) {
                            if origin.starts_with("https://") {
                                let fallback_origin = origin.replacen("https://", "http://", 1);
                                if let Ok(value) = HeaderValue::from_str(&fallback_origin) {
                                    fallback_headers.insert(ORIGIN, value);
                                }
                            }
                        }

                        if let Some(referer) = header_to_string(&fallback_headers, REFERER) {
                            if referer.starts_with("https://") {
                                let fallback_referer = referer
                                    .replacen("https://", "http://", 1)
                                    .replace("httpsFlag=Y", "httpsFlag=N");
                                if let Ok(value) = HeaderValue::from_str(&fallback_referer) {
                                    fallback_headers.insert(REFERER, value);
                                }
                            }
                        }

                        let mut fallback_req = self.client.request(method, &fallback_url).headers(fallback_headers);
                        if let Some(form) = form {
                            fallback_req = fallback_req.form(form);
                        }

                        match fallback_req.send().await {
                            Ok(response) => response,
                            Err(fallback_err) => {
                                return Err(anyhow!(
                                    "http request failed: {primary_err}; fallback failed: {fallback_err}; endpoint={}",
                                    summarize_url(&url)
                                ));
                            }
                        }
                    } else {
                        return Err(anyhow!(
                            "http request failed: {primary_err}; endpoint={}",
                            summarize_url(&url)
                        ));
                    }
                }

                #[cfg(not(target_os = "android"))]
                {
                    return Err(anyhow!(
                        "http request failed: {primary_err}; endpoint={}",
                        summarize_url(&url)
                    ));
                }
            }
        };
        self.sync_xsrf_from_headers(response.headers());
        Ok(response)
    }

    fn ajax_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded; charset=UTF-8"),
        );
        headers.insert(
            ORIGIN,
            HeaderValue::from_str(&self.base_url).context("invalid origin")?,
        );
        headers.insert(
            HeaderName::from_static("x-requested-with"),
            HeaderValue::from_static("XMLHttpRequest"),
        );
        Ok(headers)
    }

    fn valid_code_url(&self) -> String {
        let date = current_time_for_valid_code();
        let query = serde_urlencoded::to_string(vec![
            ("date", date.as_str()),
            ("includeLetter", "true"),
            ("folderName", self.folder_name.as_str()),
            ("httpsFlag", "Y"),
        ])
        .unwrap_or_default();

        format!("/PortalServer/validCodeImg?{query}")
    }

    pub async fn init_session(&mut self) -> Result<()> {
        let empty = HeaderMap::new();
        let auth_url = self.auth_url.clone();
        self.request(Method::GET, &auth_url, Some(&auth_url), empty.clone(), None)
            .await?;
        self.request(
            Method::GET,
            "/PortalServer/material/custom/custom.css",
            Some(&auth_url),
            empty.clone(),
            None,
        )
        .await?;
        self.request(
            Method::GET,
            "/PortalServer/material/custom/auth.js",
            Some(&auth_url),
            empty.clone(),
            None,
        )
        .await?;
        self.request(
            Method::GET,
            "/PortalServer/material/custom/lang/auth-zh.js",
            Some(&auth_url),
            empty.clone(),
            None,
        )
        .await?;
        let valid_code = self.valid_code_url();
        self.request(Method::GET, &valid_code, Some(&auth_url), empty, None)
            .await?;

        let custom_page_config_id = self.custom_page_config_id.clone();
        let config = [("customPageConfigId", custom_page_config_id.as_str())];
        let ajax = self.ajax_headers()?;
        self.request(
            Method::POST,
            "/PortalServer/Webauth/webAuthAction!getCustomPageConfig.action",
            Some(&auth_url),
            ajax.clone(),
            Some(&config),
        )
        .await?;

        self.request(
            Method::GET,
            "/PortalServer/Webauth/thirdPartyAuthAction!getAppIdInfo.action",
            Some(&auth_url),
            ajax,
            None,
        )
        .await?;
        Ok(())
    }

    pub async fn fetch_captcha_image(&mut self) -> Result<Vec<u8>> {
        let valid_code = self.valid_code_url();
        let auth_url = self.auth_url.clone();
        let response = self
            .request(Method::GET, &valid_code, Some(&auth_url), HeaderMap::new(), None)
            .await?;

        if response.status() != StatusCode::OK {
            return Err(anyhow!("failed to fetch captcha image: {}", response.status()));
        }

        let bytes = response.bytes().await.context("failed to read captcha bytes")?;
        if bytes.is_empty() {
            return Err(anyhow!("captcha image is empty"));
        }

        Ok(bytes.to_vec())
    }

    pub async fn login(&mut self, valid_code: &str) -> Result<LoginSubmitResult> {
        let username = self.username.clone();
        let password = self.password.clone();
        let payload = [
            ("authType", ""),
            ("userName", username.as_str()),
            ("password", password.as_str()),
            ("validCode", valid_code),
            ("valideCodeFlag", "true"),
            ("authLan", "zh_CN"),
            ("hasValidateNextUpdatePassword", "true"),
            ("rememberPwd", "false"),
            ("browserFlag", "zh"),
            ("hasCheckCode", "false"),
            ("checkcode", ""),
            ("hasRsaToken", "false"),
            ("rsaToken", ""),
            ("autoLogin", "false"),
            ("userMac", ""),
            ("isBoardPage", "false"),
            ("disablePortalMac", "false"),
            ("overdueHour", "0"),
            ("overdueMinute", "0"),
            ("isAccountMsgAuth", ""),
            ("validCodeForAuth", ""),
            ("isAgreeCheck", "1"),
        ];

        let auth_url = self.auth_url.clone();
        let response = self
            .request(
                Method::POST,
                "/PortalServer/Webauth/webAuthAction!login.action",
                Some(&auth_url),
                self.ajax_headers()?,
                Some(&payload),
            )
            .await?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("failed to read login response body")?;
        let payload: Option<Value> = serde_json::from_str(&body).ok();

        let success = status == StatusCode::OK && is_login_success(payload.as_ref(), &body);
        let message = extract_login_message(payload.as_ref());
        let failure_kind = if success {
            None
        } else {
            classify_login_failure_kind(payload.as_ref(), message.as_deref())
        };

        Ok(LoginSubmitResult {
            success,
            message,
            failure_kind,
        })
    }

    pub async fn post_login_sync(&mut self) -> Result<()> {
        let auth_url = self.auth_url.clone();
        let success_url = self.success_url.clone();
        self.request(
            Method::GET,
            &success_url,
            Some(&auth_url),
            HeaderMap::new(),
            None,
        )
        .await?;

        let sync_payload = [("browserFlag", "zh"), ("userMac", "")];
        let bind_payload = [("browserFlag", "zh")];
        let ajax = self.ajax_headers()?;
        self.request(
            Method::POST,
            "/PortalServer/Webauth/webAuthAction!syncPortalAuthResult.action",
            Some(&auth_url),
            ajax.clone(),
            Some(&sync_payload),
        )
        .await?;
        self.request(
            Method::POST,
            "/PortalServer/Webauth/webAuthAction!getBindPolicy.action",
            Some(&success_url),
            ajax,
            Some(&bind_payload),
        )
        .await?;
        Ok(())
    }

    pub async fn probe_connectivity(&self, retries: u32, interval: Duration) -> Result<bool> {
        let probes = ["http://www.baidu.com", "https://www.baidu.com"];

        for attempt in 1..=retries {
            for url in probes {
                let response = self.client.get(url).timeout(self.timeout).send().await;
                if let Ok(response) = response {
                    let status = response.status();
                    let final_url = response.url().to_string().to_lowercase();
                    let preview = response.text().await.unwrap_or_default();
                    let preview_lower = preview.to_lowercase();

                    let intercepted = final_url.contains("192.168.200.127")
                        || final_url.contains("portalserver")
                        || preview_lower.contains("portalserver");

                    if status.as_u16() < 500 && !intercepted {
                        return Ok(true);
                    }
                }
            }

            if attempt < retries {
                tokio::time::sleep(interval).await;
            }
        }

        Ok(false)
    }
}

fn summarize_url(url: &str) -> String {
    match url.split_once('?') {
        Some((base, _)) => base.to_string(),
        None => url.to_string(),
    }
}

#[cfg(target_os = "android")]
fn header_to_string(headers: &HeaderMap, name: HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string())
}

#[cfg(target_os = "android")]
fn android_http_fallback_url(url: &str) -> Option<String> {
    if !url.starts_with("https://") {
        return None;
    }

    Some(
        url.replacen("https://", "http://", 1)
            .replace("httpsFlag=Y", "httpsFlag=N"),
    )
}

fn current_time_millis() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    now.as_millis().to_string()
}

fn current_time_for_valid_code() -> String {
    chrono::Local::now()
        .format("%a %b %d %Y %H:%M:%S GMT+0800 (China Standard Time)")
        .to_string()
}

fn is_login_success(payload: Option<&Value>, body: &str) -> bool {
    if let Some(Value::Object(map)) = payload {
        if let Some(Value::Object(data)) = map.get("data") {
            let portal_auth = value_as_bool(data.get("portalAuth"));
            let status_ok = value_as_i64(data.get("portalAuthStatus")) == Some(0);
            let error_ok = match data.get("portalErrorCode") {
                None | Some(Value::Null) => true,
                Some(v) => value_as_i64(Some(v)) == Some(0),
            };
            if portal_auth && status_ok && error_ok {
                return true;
            }

            let status_code_ok = value_as_i64(data.get("statusCode")) == Some(1000);
            let access_status_ok = value_as_i64(data.get("accessStatus")) == Some(1501);
            if status_ok
                && error_ok
                && status_code_ok
                && access_status_ok
                && value_as_bool(map.get("success"))
            {
                return true;
            }
        }

        if value_as_bool(map.get("success")) {
            return true;
        }

        if value_as_i64(map.get("code")) == Some(0) {
            return true;
        }

        if let Some(result) = map.get("result") {
            if value_as_bool(Some(result)) || value_as_i64(Some(result)) == Some(0) {
                return true;
            }
        }

        return false;
    }

    let lowered = body.to_lowercase();
    lowered.contains("success.jsp")
        || lowered.contains("\"success\":true")
        || lowered.contains("\"code\":0")
}

fn extract_login_message(payload: Option<&Value>) -> Option<String> {
    let root_keys = ["message", "msg", "errorMsg", "errMsg", "reason"];
    let data_keys = ["message", "msg", "errorMsg", "errMsg", "reason", "portalErrorMsg"];

    for key in root_keys {
        if let Some(msg) = payload
            .and_then(|v| v.get(key))
            .and_then(|m| m.as_str())
            .map(str::trim)
            .filter(|m| !m.is_empty())
        {
            return Some(msg.to_string());
        }
    }

    for key in data_keys {
        if let Some(msg) = payload
            .and_then(|v| v.get("data"))
            .and_then(|data| data.get(key))
            .and_then(|m| m.as_str())
            .map(str::trim)
            .filter(|m| !m.is_empty())
        {
            return Some(msg.to_string());
        }
    }

    None
}

fn classify_login_failure_kind(payload: Option<&Value>, message: Option<&str>) -> Option<LoginFailureKind> {
    if let Some(Value::Object(map)) = payload {
        if let Some(Value::Object(data)) = map.get("data") {
            if let Some(code) = value_as_i64(data.get("portalErrorCode")) {
                if code == 0 {
                    return None;
                }
            }
        }
    }

    let message = message?.to_lowercase();

    let is_captcha_error = [
        "验证码错误",
        "验证码有误",
        "验证码不正确",
        "验证码输入错误",
        "请输入正确的验证码",
        "校验码错误",
        "图形码错误",
    ]
    .iter()
    .any(|kw| message.contains(kw));

    if is_captcha_error {
        return Some(LoginFailureKind::InvalidCaptcha);
    }
    if message.contains("用户名或密码错误") && message.contains("锁定") {
        return Some(LoginFailureKind::InvalidCredentialsOrLocked);
    }
    if message.contains("密码错误") || message.contains("用户名或密码错误") {
        return Some(LoginFailureKind::InvalidCredentials);
    }
    if message.contains("锁定") {
        return Some(LoginFailureKind::AccountLocked);
    }

    Some(LoginFailureKind::Unknown)
}

fn value_as_bool(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Bool(v)) => *v,
        Some(Value::Number(n)) => n.as_i64() == Some(1),
        Some(Value::String(s)) => {
            let normalized = s.trim().to_lowercase();
            normalized == "true" || normalized == "1"
        }
        _ => false,
    }
}

fn value_as_i64(value: Option<&Value>) -> Option<i64> {
    match value {
        Some(Value::Number(n)) => n.as_i64(),
        Some(Value::String(s)) => s.trim().parse::<i64>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{classify_login_failure_kind, extract_login_message, is_login_success};
    use crate::models::LoginFailureKind;
    use serde_json::json;

    #[test]
    fn classify_captcha_only_on_explicit_error_keywords() {
        let payload = json!({"message": "验证码错误，请重试"});
        assert!(matches!(
            classify_login_failure_kind(Some(&payload), Some("验证码错误，请重试")),
            Some(LoginFailureKind::InvalidCaptcha)
        ));

        let weak_message = "请先输入验证码后继续";
        assert!(matches!(
            classify_login_failure_kind(Some(&payload), Some(weak_message)),
            Some(LoginFailureKind::Unknown)
        ));
    }

    #[test]
    fn extract_message_from_multiple_common_fields() {
        let payload = json!({"data": {"portalErrorMsg": "用户名或密码错误"}});
        assert_eq!(
            extract_login_message(Some(&payload)).as_deref(),
            Some("用户名或密码错误")
        );
    }

    #[test]
    fn parse_success_with_string_or_numeric_fields() {
        let payload = json!({"success": "true"});
        assert!(is_login_success(Some(&payload), ""));

        let payload = json!({"code": "0"});
        assert!(is_login_success(Some(&payload), ""));

        let payload = json!({"data": {"portalAuth": true, "portalAuthStatus": "0", "portalErrorCode": "0"}});
        assert!(is_login_success(Some(&payload), ""));

        let payload = json!({
            "success": true,
            "token": "token=abc",
            "data": {
                "portalAuth": false,
                "portalAuthStatus": 0,
                "portalErrorCode": 0,
                "statusCode": 1000,
                "accessStatus": 1501
            }
        });
        assert!(is_login_success(Some(&payload), ""));
    }

    #[test]
    fn regression_success_true_with_token_must_not_be_blocked_by_portal_auth_false() {
        let payload = json!({
            "success": true,
            "token": "token=xyz",
            "data": {
                "portalAuth": false,
                "portalAuthStatus": 0,
                "portalErrorCode": 0,
                "statusCode": 1000,
                "accessStatus": 1501
            }
        });

        assert!(is_login_success(Some(&payload), ""));
    }

    #[test]
    fn regression_missing_top_success_should_not_be_treated_as_success() {
        let payload = json!({
            "success": false,
            "token": null,
            "data": {
                "portalAuth": false,
                "portalAuthStatus": 0,
                "portalErrorCode": 0,
                "statusCode": 1000,
                "accessStatus": 1501
            }
        });

        assert!(!is_login_success(Some(&payload), ""));
    }

    #[test]
    fn regression_captcha_error_message_is_classified_explicitly() {
        let payload = json!({"message": "验证码错误。", "success": false});
        assert!(matches!(
            classify_login_failure_kind(Some(&payload), Some("验证码错误。")),
            Some(LoginFailureKind::InvalidCaptcha)
        ));
    }

    #[test]
    fn keep_unknown_when_message_absent_or_not_specific() {
        let payload = json!({"message": ""});
        assert!(classify_login_failure_kind(Some(&payload), None).is_none());
        assert!(matches!(
            classify_login_failure_kind(Some(&payload), Some("验证码已刷新")),
            Some(LoginFailureKind::Unknown)
        ));
    }
}
