use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::crypto::CryptoManager;
use crate::vault::{DeletedEntry, Entry, Vault};

const CLIENT_ID: &str =
    "184164491921-ds7k9t99jfe0eq91siu14m8s7s501c5f.apps.googleusercontent.com";
const CLIENT_SECRET: &str = "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";
const DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const DRIVE_FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";
const DRIVE_UPLOAD_URL: &str = "https://www.googleapis.com/upload/drive/v3/files";
const VAULT_FILE_NAME: &str = "securepass_vault.dat";
const OAUTH_TIMEOUT_SECS: u64 = 300; // 5 minutes

// ── Persisted types ──────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CloudTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SyncState {
    pub connected: bool,
    pub drive_file_id: Option<String>,
    pub last_sync: Option<DateTime<Utc>>,
}

// ── Public result types ──────────────────────────────────────────────────────

pub struct MergeResult {
    pub merged: Vault,
    /// (local_entry, remote_entry) for each entry modified on both sides.
    pub raw_conflicts: Vec<(Entry, Entry)>,
    pub entries_added: usize,
    pub entries_updated: usize,
}

// ── File paths ───────────────────────────────────────────────────────────────

fn securepass_dir() -> PathBuf {
    dirs::home_dir()
        .expect("home dir")
        .join(".securepass")
}

fn tokens_path() -> PathBuf { securepass_dir().join("cloud_tokens.json") }
fn sync_state_path() -> PathBuf { securepass_dir().join("sync_state.json") }

// ── Token + state persistence ────────────────────────────────────────────────

pub fn load_tokens() -> Option<CloudTokens> {
    let data = std::fs::read_to_string(tokens_path()).ok()?;
    serde_json::from_str(&data).ok()
}

fn save_tokens(tokens: &CloudTokens) -> Result<(), String> {
    let dir = securepass_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let data = serde_json::to_string(tokens).map_err(|e| e.to_string())?;
    std::fs::write(tokens_path(), data).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_sync_state() -> SyncState {
    std::fs::read_to_string(sync_state_path())
        .ok()
        .and_then(|d| serde_json::from_str(&d).ok())
        .unwrap_or_default()
}

pub fn save_sync_state(state: &SyncState) -> Result<(), String> {
    let dir = securepass_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let data = serde_json::to_string(state).map_err(|e| e.to_string())?;
    std::fs::write(sync_state_path(), data).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn clear_cloud_data() -> Result<(), String> {
    for path in [tokens_path(), sync_state_path()] {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

// ── OAuth 2.0 flow ───────────────────────────────────────────────────────────

fn random_state() -> String {
    use rand::Rng;
    rand::thread_rng()
        .sample_iter(rand::distributions::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

/// Percent-encode a string for use in a URL query parameter.
fn url_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{:02X}", b),
        })
        .collect()
}

/// Percent-decode a URL query parameter value.
fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut result = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                result.push(byte as char);
                i += 3;
                continue;
            }
        } else if bytes[i] == b'+' {
            result.push(' ');
            i += 1;
            continue;
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

fn parse_query_param(request: &str, param: &str) -> Option<String> {
    let prefix = format!("{}=", param);
    let line = request.lines().next()?;
    let q_start = line.find('?')?;
    let q_end = line.rfind(" HTTP")?;
    let query = &line[q_start + 1..q_end];
    for part in query.split('&') {
        if let Some(raw) = part.strip_prefix(&prefix) {
            return Some(url_decode(raw));
        }
    }
    None
}

fn open_in_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(url).spawn().map_err(|e| e.to_string())?;
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open").arg(url).spawn().map_err(|e| e.to_string())?;
    #[cfg(target_os = "windows")]
    std::process::Command::new("cmd").args(["/c", "start", "", url]).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

/// Start the OAuth 2.0 authorization code flow. Opens the user's browser,
/// spins up a temporary local HTTP server to catch the redirect, exchanges
/// the code for tokens, and persists them. Times out after 5 minutes.
pub async fn start_oauth_flow() -> Result<CloudTokens, String> {
    use tokio::net::TcpListener;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::time::{timeout, Duration};

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Failed to bind local port: {}", e))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();

    let redirect_uri = format!("http://localhost:{}", port);
    let state = random_state();

    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&access_type=offline&prompt=consent",
        AUTH_URL,
        url_encode(CLIENT_ID),
        url_encode(&redirect_uri),
        url_encode(DRIVE_SCOPE),
        state,
    );

    open_in_browser(&auth_url)?;

    let (mut stream, _) = timeout(Duration::from_secs(OAUTH_TIMEOUT_SECS), listener.accept())
        .await
        .map_err(|_| "OAuth timed out — please try again")?
        .map_err(|e| format!("Failed to accept redirect: {}", e))?;

    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await.map_err(|e| e.to_string())?;
    let request = String::from_utf8_lossy(&buf[..n]).to_string();

    let code = parse_query_param(&request, "code")
        .ok_or("No authorization code in redirect")?;
    let returned_state = parse_query_param(&request, "state")
        .ok_or("No state parameter in redirect")?;

    if returned_state != state {
        return Err("OAuth state mismatch — possible CSRF, please try again".to_string());
    }

    let html = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
        <html><body style='font-family:sans-serif;text-align:center;padding:60px'>\
        <h2>SecurePass connected!</h2>\
        <p>You can close this tab and return to the app.</p>\
        </body></html>";
    stream.write_all(html.as_bytes()).await.ok();

    let tokens = exchange_code_for_tokens(&code, &redirect_uri).await?;
    save_tokens(&tokens)?;
    Ok(tokens)
}

async fn exchange_code_for_tokens(code: &str, redirect_uri: &str) -> Result<CloudTokens, String> {
    let client = reqwest::Client::new();
    let params = [
        ("code", code),
        ("client_id", CLIENT_ID),
        ("client_secret", CLIENT_SECRET),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code"),
    ];

    let resp: serde_json::Value = client
        .post(TOKEN_URL)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Token exchange failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {}", e))?;

    tokens_from_json(&resp, None)
}

async fn refresh_access_token(refresh_token: &str) -> Result<CloudTokens, String> {
    let client = reqwest::Client::new();
    let params = [
        ("refresh_token", refresh_token),
        ("client_id", CLIENT_ID),
        ("client_secret", CLIENT_SECRET),
        ("grant_type", "refresh_token"),
    ];

    let resp: serde_json::Value = client
        .post(TOKEN_URL)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Token refresh failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse refresh response: {}", e))?;

    tokens_from_json(&resp, Some(refresh_token))
}

fn tokens_from_json(json: &serde_json::Value, fallback_refresh: Option<&str>) -> Result<CloudTokens, String> {
    let access_token = json["access_token"]
        .as_str()
        .ok_or("No access_token in response")?
        .to_string();
    let refresh_token = json["refresh_token"]
        .as_str()
        .map(str::to_string)
        .or_else(|| fallback_refresh.map(str::to_string))
        .ok_or("No refresh_token in response")?;
    let expires_in = json["expires_in"].as_i64().unwrap_or(3600);

    Ok(CloudTokens {
        access_token,
        refresh_token,
        expires_at: Utc::now() + chrono::Duration::seconds(expires_in),
    })
}

/// Returns a valid access token, refreshing if needed.
pub async fn get_valid_access_token() -> Result<String, String> {
    let tokens = load_tokens().ok_or("Not connected to Google Drive. Please connect in Settings.")?;

    if tokens.expires_at > Utc::now() + chrono::Duration::seconds(60) {
        return Ok(tokens.access_token);
    }

    let refreshed = refresh_access_token(&tokens.refresh_token).await?;
    save_tokens(&refreshed)?;
    Ok(refreshed.access_token)
}

// ── Google Drive API ─────────────────────────────────────────────────────────

/// Find the vault file on Drive by name; returns its file ID or None.
pub async fn find_drive_file(access_token: &str) -> Result<Option<String>, String> {
    let client = reqwest::Client::new();
    let query = format!("name='{}' and not trashed", VAULT_FILE_NAME);

    let resp: serde_json::Value = client
        .get(DRIVE_FILES_URL)
        .bearer_auth(access_token)
        .query(&[("q", query.as_str()), ("fields", "files(id)")])
        .send()
        .await
        .map_err(|e| format!("Drive search failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse Drive search: {}", e))?;

    let id = resp["files"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|f| f["id"].as_str())
        .map(str::to_string);

    Ok(id)
}

/// Create or update the vault file on Drive using a multipart/related upload.
/// Returns the Drive file ID.
pub async fn upload_vault_to_drive(
    access_token: &str,
    vault_bytes: Vec<u8>,
    file_id: Option<&str>,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let boundary = "securepass_boundary_42";

    let meta = format!("{{\"name\":\"{}\"}}", VAULT_FILE_NAME);
    let mut body: Vec<u8> = format!(
        "--{b}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{meta}\r\n\
         --{b}\r\nContent-Type: application/octet-stream\r\n\r\n",
        b = boundary,
        meta = meta,
    )
    .into_bytes();
    body.extend_from_slice(&vault_bytes);
    body.write_all(format!("\r\n--{}--", boundary).as_bytes()).unwrap();

    let content_type = format!("multipart/related; boundary={}", boundary);

    let resp: serde_json::Value = if let Some(id) = file_id {
        client
            .patch(format!("{}/{}?uploadType=multipart", DRIVE_UPLOAD_URL, id))
            .bearer_auth(access_token)
            .header("Content-Type", &content_type)
            .body(body)
            .send()
            .await
            .map_err(|e| format!("Drive upload failed: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse upload response: {}", e))?
    } else {
        client
            .post(format!("{}?uploadType=multipart", DRIVE_UPLOAD_URL))
            .bearer_auth(access_token)
            .header("Content-Type", &content_type)
            .body(body)
            .send()
            .await
            .map_err(|e| format!("Drive upload failed: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse upload response: {}", e))?
    };

    resp["id"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("No file ID in upload response: {}", resp))
}

/// Fetch the `modifiedTime` of a Drive file without downloading its content.
pub async fn get_drive_file_modified_time(
    access_token: &str,
    file_id: &str,
) -> Result<Option<DateTime<Utc>>, String> {
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .get(format!("{}/{}", DRIVE_FILES_URL, file_id))
        .bearer_auth(access_token)
        .query(&[("fields", "modifiedTime")])
        .send()
        .await
        .map_err(|e| format!("Drive metadata fetch failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse Drive metadata: {}", e))?;

    let time = resp["modifiedTime"]
        .as_str()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|t| t.with_timezone(&Utc));

    Ok(time)
}

/// Download the raw vault bytes from Drive.
pub async fn download_vault_from_drive(access_token: &str, file_id: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::new();
    let bytes = client
        .get(format!("{}/{}", DRIVE_FILES_URL, file_id))
        .bearer_auth(access_token)
        .query(&[("alt", "media")])
        .send()
        .await
        .map_err(|e| format!("Drive download failed: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("Failed to read Drive file: {}", e))?;

    Ok(bytes.to_vec())
}

// ── Vault encrypt / decrypt helpers ─────────────────────────────────────────

pub fn encrypt_vault_bytes(vault: &Vault, crypto: &CryptoManager) -> Result<Vec<u8>, String> {
    let json = serde_json::to_string(vault).map_err(|e| e.to_string())?;
    let nonce = CryptoManager::generate_nonce();
    let ciphertext = crypto
        .encrypt(json.as_bytes(), &nonce, b"vault")
        .map_err(|e| e.to_string())?;

    let mut data = vault.salt.to_vec();
    data.extend_from_slice(&nonce);
    data.extend(ciphertext);
    Ok(data)
}

pub fn decrypt_vault_bytes(data: &[u8], crypto: &CryptoManager) -> Result<Vault, String> {
    if data.len() < 44 {
        return Err("Invalid vault data — too short".to_string());
    }
    let nonce_bytes: [u8; 12] = data[32..44].try_into().map_err(|_| "Bad nonce")?;
    let nonce = chacha20poly1305::Nonce::from(nonce_bytes);
    let decrypted = crypto
        .decrypt(&data[44..], &nonce, b"vault")
        .map_err(|_| "Failed to decrypt remote vault — ensure you are using the same master password on all devices".to_string())?;
    let json = String::from_utf8(decrypted).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| format!("Invalid vault format: {}", e))
}

// ── Merge algorithm ──────────────────────────────────────────────────────────

/// Perform an entry-level three-way merge of `local` and `remote` vaults.
///
/// - Entries modified on only one side since `last_sync` are taken automatically.
/// - Entries modified on both sides are returned as conflicts for the user to resolve.
/// - Deletions (tombstones) from either side are propagated to the merged result.
pub fn merge_vaults(
    local: &Vault,
    remote: &Vault,
    last_sync: Option<DateTime<Utc>>,
) -> MergeResult {
    let epoch = last_sync.unwrap_or(DateTime::<Utc>::MIN_UTC);

    let local_map: HashMap<Uuid, &Entry> = local.entries.iter().map(|e| (e.id, e)).collect();
    let remote_map: HashMap<Uuid, &Entry> = remote.entries.iter().map(|e| (e.id, e)).collect();
    let local_deleted: HashSet<Uuid> = local.deleted_entries.iter().map(|d| d.id).collect();
    let remote_deleted: HashSet<Uuid> = remote.deleted_entries.iter().map(|d| d.id).collect();

    let all_ids: HashSet<Uuid> = local_map.keys().chain(remote_map.keys()).cloned().collect();

    let mut merged_entries: Vec<Entry> = Vec::new();
    let mut raw_conflicts: Vec<(Entry, Entry)> = Vec::new();
    let mut entries_added = 0usize;
    let mut entries_updated = 0usize;

    for id in &all_ids {
        match (local_map.get(id), remote_map.get(id)) {
            (Some(le), Some(re)) => {
                let local_changed = le.updated_at > epoch;
                let remote_changed = re.updated_at > epoch;

                match (local_changed, remote_changed) {
                    (true, true) if le.updated_at != re.updated_at => {
                        // Both sides modified since last sync — conflict.
                        raw_conflicts.push(((*le).clone(), (*re).clone()));
                    }
                    (false, true) => {
                        merged_entries.push((*re).clone());
                        entries_updated += 1;
                    }
                    _ => {
                        // Local is newer or both unchanged — keep local.
                        merged_entries.push((*le).clone());
                    }
                }
            }
            (Some(le), None) => {
                // In local only.
                if !remote_deleted.contains(id) {
                    if le.created_at > epoch {
                        entries_added += 1;
                    }
                    merged_entries.push((*le).clone());
                }
                // If remote has a tombstone for this ID, deletion propagates — skip.
            }
            (None, Some(re)) => {
                // In remote only.
                if !local_deleted.contains(id) {
                    if re.created_at > epoch {
                        entries_added += 1;
                    }
                    merged_entries.push((*re).clone());
                }
            }
            (None, None) => {} // unreachable
        }
    }

    // Union of both tombstone lists so deletions sync in all directions.
    let mut merged_deleted = local.deleted_entries.clone();
    for d in &remote.deleted_entries {
        if !merged_deleted.iter().any(|ld: &DeletedEntry| ld.id == d.id) {
            merged_deleted.push(d.clone());
        }
    }

    let mut merged = local.clone();
    merged.entries = merged_entries;
    merged.deleted_entries = merged_deleted;
    merged.updated_at = Utc::now();

    MergeResult { merged, raw_conflicts, entries_added, entries_updated }
}
