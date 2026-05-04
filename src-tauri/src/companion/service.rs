use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};
use tokio::{sync::Mutex, task::JoinHandle, time::timeout};

use crate::companion::{
    client::CompanionBleClient,
    error::{CompanionError, Result},
    protocol::{
        bt_action_from_request, default_scan_timeout, default_timeout, generate_pairing_credentials,
        lastfm_action_from_request, output_target_to_id, parse_button_sequence,
        playback_action_from_request, playback_mode_to_id, CompanionCredentials, ConnectedDevice,
        EVENT_NAME, AUTH_NONCE_LEN,
    },
    storage::CredentialStore,
};

pub struct AppState {
    manager: CompanionManager,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            manager: CompanionManager::default(),
        }
    }
}

impl AppState {
    pub fn manager(&self) -> &CompanionManager {
        &self.manager
    }
}

#[derive(Default)]
pub struct CompanionManager {
    session: Mutex<Option<CompanionSession>>,
}

struct CompanionSession {
    client: CompanionBleClient,
    profile: String,
    credential_override: Option<CompanionCredentials>,
    store: CredentialStore,
    event_task: JoinHandle<()>,
}

#[derive(Debug, Deserialize)]
pub struct ScanRequest {
    pub scan_timeout_secs: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct ConnectRequest {
    pub address: Option<String>,
    pub name: Option<String>,
    pub profile: Option<String>,
    pub client_id: Option<String>,
    pub app_name: Option<String>,
    pub secret_hex: Option<String>,
    pub timeout_secs: Option<f64>,
    pub scan_timeout_secs: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct PingRequest {
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PairBeginRequest {
    pub client_id: Option<String>,
    pub app_name: Option<String>,
    pub secret_hex: Option<String>,
    pub sequence: Option<Vec<String>>,
    pub wait: Option<bool>,
    pub wait_timeout_secs: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    pub client_id: Option<String>,
    pub app_name: Option<String>,
    pub secret_hex: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TrustedRevokeRequest {
    pub client_id: String,
}

#[derive(Debug, Deserialize)]
pub struct PlaybackControlRequest {
    pub action: String,
    pub value: Option<u32>,
    pub mode: Option<String>,
    pub output_target: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PageRequest {
    pub offset: Option<u32>,
    pub count: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct WifiConnectRequest {
    pub ssid: String,
    pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WifiConnectSlotRequest {
    pub slot: u32,
}

#[derive(Debug, Deserialize)]
pub struct ToggleRequest {
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct LastfmControlRequest {
    pub action: String,
    pub url: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct BtControlRequest {
    pub action: String,
}

#[derive(Debug, Serialize)]
pub struct ConnectionStatus {
    pub connected: bool,
    pub device: Option<ConnectedDevice>,
}

impl CompanionManager {
    pub async fn scan(&self, request: Option<ScanRequest>) -> Result<Vec<crate::companion::protocol::DiscoveredDevice>> {
        let timeout_duration = request
            .and_then(|request| request.scan_timeout_secs)
            .map(Duration::from_secs_f64)
            .unwrap_or_else(default_scan_timeout);
        CompanionBleClient::scan(timeout_duration).await
    }

    pub async fn connect(&self, app: &AppHandle, request: ConnectRequest) -> Result<ConnectionStatus> {
        let profile = resolve_profile(request.profile.as_deref(), request.address.as_deref(), request.name.as_deref());
        let store = CredentialStore::for_app(app)?;
        let credential_override = explicit_credentials(request.client_id, request.app_name, request.secret_hex)?;
        let client = CompanionBleClient::connect(
            request.address.as_deref(),
            request.name.as_deref(),
            &profile,
            request
                .scan_timeout_secs
                .map(Duration::from_secs_f64)
                .unwrap_or_else(default_scan_timeout),
            request
                .timeout_secs
                .map(Duration::from_secs_f64)
                .unwrap_or_else(default_timeout),
        )
        .await?;

        let mut event_rx = client.subscribe_events();
        let event_app = app.clone();
        let event_task = tokio::spawn(async move {
            loop {
                match event_rx.recv().await {
                    Ok(event) => {
                        let _ = event_app.emit(EVENT_NAME, event);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });

        let device = client.connected_device().clone();
        let session = CompanionSession {
            client,
            profile,
            credential_override,
            store,
            event_task,
        };

        let mut guard = self.session.lock().await;
        if let Some(mut current) = guard.take() {
            current.event_task.abort();
            let _ = current.client.disconnect().await;
        }
        *guard = Some(session);

        Ok(ConnectionStatus {
            connected: true,
            device: Some(device),
        })
    }

    pub async fn disconnect(&self) -> Result<ConnectionStatus> {
        let mut guard = self.session.lock().await;
        if let Some(mut session) = guard.take() {
            session.event_task.abort();
            session.client.disconnect().await?;
        }
        Ok(ConnectionStatus {
            connected: false,
            device: None,
        })
    }

    pub async fn status(&self) -> Result<ConnectionStatus> {
        let guard = self.session.lock().await;
        Ok(match guard.as_ref() {
            Some(session) => ConnectionStatus {
                connected: true,
                device: Some(session.client.connected_device().clone()),
            },
            None => ConnectionStatus {
                connected: false,
                device: None,
            },
        })
    }

    pub async fn hello(&self) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        session.client.hello().await
    }

    pub async fn capabilities(&self) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        session.client.capabilities().await
    }

    pub async fn ping(&self, request: Option<PingRequest>) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        session
            .client
            .ping(request.and_then(|request| request.text).as_deref().unwrap_or("ping"))
            .await
    }

    pub async fn pair_begin(&self, request: PairBeginRequest) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        let credentials = generate_pairing_credentials(request.client_id, request.app_name, request.secret_hex);
        let secret = credentials.secret()?;
        let sequence = parse_button_sequence(request.sequence.as_deref())?;
        let pair_status = session
            .client
            .pair_begin(&credentials.client_id, &credentials.app_name, &secret, &sequence)
            .await?;
        session
            .store
            .put_credentials(&session.profile, credentials.clone())?;
        session.credential_override = Some(credentials.clone());

        let mut result = json!({
            "credentials_saved_to": session.store.path(),
            "profile": session.profile,
            "client_id": credentials.client_id,
            "app_name": credentials.app_name,
            "secret_hex": credentials.secret_hex,
            "button_sequence": sequence,
            "pair_status": pair_status,
        });
        result["button_sequence"] = json!(sequence.iter().map(|value| crate::companion::protocol::button_id_to_name(*value)).collect::<Vec<_>>());

        if request.wait.unwrap_or(true) {
            let deadline = Instant::now() + Duration::from_secs_f64(request.wait_timeout_secs.unwrap_or(120.0));
            let mut events = session.client.subscribe_events();
            loop {
                let capabilities = session.client.capabilities().await?;
                if capabilities
                    .get("authenticated")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    result["authenticated"] = json!(true);
                    result["capabilities"] = capabilities;
                    break;
                }
                if Instant::now() >= deadline {
                    result["authenticated"] = json!(false);
                    result["timeout"] = json!(true);
                    break;
                }
                if let Ok(Ok(event)) = timeout(Duration::from_secs(1), events.recv()).await {
                    result["last_event"] = event;
                }
            }
        }

        Ok(result)
    }

    pub async fn pair_status(&self) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        session.client.pair_status().await
    }

    pub async fn pair_cancel(&self) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        session.client.pair_cancel().await
    }

    pub async fn auth(&self, request: Option<AuthRequest>) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        authenticate_session(session, request).await
    }

    pub async fn trusted_list(&self) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        ensure_authenticated(session, None).await?;
        session.client.trusted_list().await
    }

    pub async fn trusted_revoke(&self, request: TrustedRevokeRequest) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        ensure_authenticated(session, None).await?;
        session.client.trusted_revoke(&request.client_id).await
    }

    pub async fn snapshot(&self) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        ensure_authenticated(session, None).await?;
        session.client.snapshot().await
    }

    pub async fn playback_status(&self) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        ensure_authenticated(session, None).await?;
        session.client.playback_status().await
    }

    pub async fn playback_control(&self, request: PlaybackControlRequest) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        ensure_authenticated(session, None).await?;

        let action = playback_action_from_request(&request.action)?;
        let value = match action {
            crate::companion::protocol::PlaybackAction::SetMode => Some(playback_mode_to_id(request.mode.as_deref().ok_or_else(|| CompanionError::UnknownPlaybackMode(String::new()))?)? as u32),
            crate::companion::protocol::PlaybackAction::SetOutputTarget => Some(output_target_to_id(request.output_target.as_deref().ok_or_else(|| CompanionError::UnknownOutputTarget(String::new()))?)? as u32),
            crate::companion::protocol::PlaybackAction::Next
            | crate::companion::protocol::PlaybackAction::Previous
            | crate::companion::protocol::PlaybackAction::PauseToggle
            | crate::companion::protocol::PlaybackAction::FastForward
            | crate::companion::protocol::PlaybackAction::Rewind => None,
            _ => request.value,
        };
        session.client.playback_control(action, value).await
    }

    pub async fn library_album(&self) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        ensure_authenticated(session, None).await?;
        session.client.library_album().await
    }

    pub async fn library_track_page(&self, request: Option<PageRequest>) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        ensure_authenticated(session, None).await?;
        let request = request.unwrap_or(PageRequest {
            offset: Some(0),
            count: Some(8),
        });
        session
            .client
            .library_track_page(request.offset.unwrap_or(0), request.count.unwrap_or(8))
            .await
    }

    pub async fn wifi_status(&self) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        ensure_authenticated(session, None).await?;
        session.client.wifi_status().await
    }

    pub async fn wifi_scan_start(&self) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        ensure_authenticated(session, None).await?;
        session.client.wifi_scan_start().await
    }

    pub async fn wifi_scan_results(&self, request: Option<PageRequest>) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        ensure_authenticated(session, None).await?;
        let request = request.unwrap_or(PageRequest {
            offset: Some(0),
            count: Some(8),
        });
        session
            .client
            .wifi_scan_results(request.offset.unwrap_or(0), request.count.unwrap_or(8))
            .await
    }

    pub async fn wifi_connect(&self, request: WifiConnectRequest) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        ensure_authenticated(session, None).await?;
        session
            .client
            .wifi_connect(&request.ssid, request.password.as_deref().unwrap_or(""))
            .await
    }

    pub async fn wifi_connect_slot(&self, request: WifiConnectSlotRequest) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        ensure_authenticated(session, None).await?;
        session.client.wifi_connect_slot(request.slot).await
    }

    pub async fn wifi_disconnect(&self) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        ensure_authenticated(session, None).await?;
        session.client.wifi_disconnect().await
    }

    pub async fn wifi_autoreconnect(&self, request: ToggleRequest) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        ensure_authenticated(session, None).await?;
        session.client.wifi_autoreconnect(request.enabled).await
    }

    pub async fn lastfm_status(&self) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        ensure_authenticated(session, None).await?;
        session.client.lastfm_status().await
    }

    pub async fn lastfm_control(&self, request: LastfmControlRequest) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        ensure_authenticated(session, None).await?;
        let action = lastfm_action_from_request(&request.action)?;
        session
            .client
            .lastfm_control(
                action,
                request.url.as_deref(),
                request.username.as_deref(),
                request.password.as_deref(),
                request.enabled,
            )
            .await
    }

    pub async fn history_summary(&self) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        ensure_authenticated(session, None).await?;
        session.client.history_summary().await
    }

    pub async fn history_album_page(&self, request: Option<PageRequest>) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        ensure_authenticated(session, None).await?;
        let request = request.unwrap_or(PageRequest {
            offset: Some(0),
            count: Some(4),
        });
        session
            .client
            .history_album_page(request.offset.unwrap_or(0), request.count.unwrap_or(4))
            .await
    }

    pub async fn bt_audio_status(&self) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        ensure_authenticated(session, None).await?;
        session.client.bt_audio_status().await
    }

    pub async fn bt_audio_control(&self, request: BtControlRequest) -> Result<Value> {
        let mut guard = self.session.lock().await;
        let session = session_mut(&mut guard)?;
        ensure_authenticated(session, None).await?;
        session
            .client
            .bt_audio_control(bt_action_from_request(&request.action)?)
            .await
    }
}

fn resolve_profile(profile: Option<&str>, address: Option<&str>, name: Option<&str>) -> String {
    if let Some(profile) = profile {
        return profile.to_string();
    }
    if let Some(address) = address {
        return address.to_lowercase();
    }
    if let Some(name) = name {
        return name.to_string();
    }
    "default".to_string()
}

fn explicit_credentials(
    client_id: Option<String>,
    app_name: Option<String>,
    secret_hex: Option<String>,
) -> Result<Option<CompanionCredentials>> {
    match (client_id, secret_hex) {
        (Some(client_id), Some(secret_hex)) => Ok(Some(CompanionCredentials {
            client_id,
            app_name: app_name.unwrap_or_else(|| "jukeboy-companion".to_string()),
            secret_hex,
        })),
        (None, None) => Ok(None),
        _ => Err(CompanionError::MissingCredentials),
    }
}

fn session_mut<'a>(
    guard: &'a mut Option<CompanionSession>,
) -> Result<&'a mut CompanionSession> {
    guard.as_mut().ok_or(CompanionError::NotConnected)
}

async fn ensure_authenticated(
    session: &mut CompanionSession,
    request: Option<AuthRequest>,
) -> Result<Value> {
    let capabilities = session.client.capabilities().await?;
    if capabilities
        .get("authenticated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(capabilities);
    }
    authenticate_session(session, request).await
}

async fn authenticate_session(
    session: &mut CompanionSession,
    request: Option<AuthRequest>,
) -> Result<Value> {
    let credentials = if let Some(request) = request {
        if let Some(credentials) = explicit_credentials(request.client_id, request.app_name, request.secret_hex)? {
            session.credential_override = Some(credentials.clone());
            credentials
        } else if let Some(credentials) = session.credential_override.clone() {
            credentials
        } else {
            session
                .store
                .get_credentials(&session.profile)?
                .ok_or(CompanionError::MissingCredentials)?
        }
    } else if let Some(credentials) = session.credential_override.clone() {
        credentials
    } else {
        session
            .store
            .get_credentials(&session.profile)?
            .ok_or(CompanionError::MissingCredentials)?
    };

    let secret = credentials.secret()?;
    let challenge = session.client.auth_challenge(&credentials.client_id).await?;
    let nonce_hex = challenge
        .get("nonce_hex")
        .and_then(Value::as_str)
        .ok_or(CompanionError::InvalidNonce)?;
    if nonce_hex.len() != AUTH_NONCE_LEN * 2 {
        return Err(CompanionError::InvalidNonce);
    }
    let nonce = hex::decode(nonce_hex).map_err(|_| CompanionError::InvalidNonce)?;
    session
        .client
        .auth_proof(&credentials.client_id, &secret, &nonce)
        .await
}