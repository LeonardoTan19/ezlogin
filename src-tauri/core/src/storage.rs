use std::fs;
use std::path::PathBuf;

#[cfg(target_os = "android")]
use std::path::Path;

use aes_gcm::aead::{Aead, OsRng};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::models::{LoginOptions, SavedCredentials};

#[derive(Debug, Serialize, Deserialize)]
struct StoredCredentials {
    account: String,
    password_cipher: String,
}

fn app_config_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    if let Some(base) = android_app_files_dir() {
        return base.join("ezlogin");
    }

    dirs::config_dir()
        .or_else(dirs::data_local_dir)
        .or_else(|| std::env::var("XDG_CONFIG_HOME").ok().map(PathBuf::from))
        .or_else(|| std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config")))
        .or_else(|| std::env::var("TMPDIR").ok().map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir)
        .join("ezlogin")
}

fn credentials_path() -> Result<PathBuf, String> {
    Ok(app_config_dir().join("credentials.json"))
}

fn login_options_path() -> PathBuf {
    app_config_dir().join("login-options.json")
}

#[cfg(target_os = "android")]
fn android_app_files_dir() -> Option<PathBuf> {
    let package = fs::read("/proc/self/cmdline")
        .ok()
        .and_then(|bytes| {
            let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
            String::from_utf8(bytes[..end].to_vec()).ok()
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())?;

    let candidates = [
        PathBuf::from(format!("/data/user/0/{package}/files")),
        PathBuf::from(format!("/data/data/{package}/files")),
    ];

    candidates
        .into_iter()
        .find(|dir| ensure_writable_dir(dir.as_path()))
}

#[cfg(target_os = "android")]
fn ensure_writable_dir(path: &Path) -> bool {
    if !path.exists() && fs::create_dir_all(path).is_err() {
        return false;
    }

    let probe = path.join(".ezlogin_write_probe");
    if fs::write(&probe, b"ok").is_err() {
        return false;
    }
    let _ = fs::remove_file(probe);
    true
}

fn derive_key() -> [u8; 32] {
    let user = std::env::var("USER").unwrap_or_else(|_| "unknown-user".to_string());
    let host = std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown-host".to_string());
    let custom = std::env::var("EZLOGIN_SECRET_KEY").unwrap_or_default();
    let seed = format!("ezlogin:{user}:{host}:{custom}");
    let digest = Sha256::digest(seed.as_bytes());
    let mut key = [0_u8; 32];
    key.copy_from_slice(&digest);
    key
}

fn encrypt_password(plain: &str) -> Result<String> {
    use aes_gcm::AeadCore;

    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key).context("failed to init cipher")?;

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let encrypted = cipher
        .encrypt(&nonce, plain.as_bytes())
        .map_err(|_| anyhow::anyhow!("failed to encrypt password"))?;

    Ok(format!("{}:{}", BASE64.encode(nonce), BASE64.encode(encrypted)))
}

fn decrypt_password(payload: &str) -> Result<String> {
    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key).context("failed to init cipher")?;

    let mut split = payload.splitn(2, ':');
    let nonce_raw = split.next().ok_or_else(|| anyhow::anyhow!("missing nonce"))?;
    let encrypted_raw = split
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing cipher text"))?;

    let nonce = BASE64.decode(nonce_raw).context("invalid nonce encoding")?;
    let encrypted = BASE64
        .decode(encrypted_raw)
        .context("invalid cipher encoding")?;

    let decrypted = cipher
        .decrypt(Nonce::from_slice(&nonce), encrypted.as_ref())
        .map_err(|_| anyhow::anyhow!("failed to decrypt password"))?;

    String::from_utf8(decrypted).context("invalid decrypted utf8")
}

pub fn save_credentials(account: &str, password: &str) -> Result<(), String> {
    let path = credentials_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create credentials dir {}: {e}", parent.display()))?;
    }

    let payload = StoredCredentials {
        account: account.to_string(),
        password_cipher: encrypt_password(password).map_err(|e| e.to_string())?,
    };

    let content = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    fs::write(&path, content)
        .map_err(|e| format!("failed to write credentials {}: {e}", path.display()))
}

pub fn load_credentials() -> Result<Option<SavedCredentials>, String> {
    let path = credentials_path()?;
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read credentials {}: {e}", path.display()))?;
    let stored: StoredCredentials = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let password = decrypt_password(&stored.password_cipher).map_err(|e| e.to_string())?;

    Ok(Some(SavedCredentials {
        account: stored.account,
        password,
    }))
}

pub fn clear_credentials() -> Result<(), String> {
    let path = credentials_path()?;
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| format!("failed to remove credentials {}: {e}", path.display()))?;
    }
    Ok(())
}

pub fn save_login_options(options: &LoginOptions) -> Result<(), String> {
    let path = login_options_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create config dir {}: {e}", parent.display()))?;
    }

    let content = serde_json::to_string_pretty(options).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| format!("failed to write config {}: {e}", path.display()))
}

pub fn load_login_options() -> Result<Option<LoginOptions>, String> {
    let path = login_options_path();
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read config {}: {e}", path.display()))?;
    let options: LoginOptions = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(Some(options))
}
