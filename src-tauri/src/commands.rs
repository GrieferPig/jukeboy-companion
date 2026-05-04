use serde_json::Value;
use tauri::{AppHandle, State};

use crate::companion::{
    AppState, AuthRequest, BtControlRequest, ConnectRequest, ConnectionStatus, LastfmControlRequest,
    PageRequest, PairBeginRequest, PingRequest, PlaybackControlRequest, ScanRequest, ToggleRequest,
    TrustedRevokeRequest, WifiConnectRequest, WifiConnectSlotRequest,
};

type CommandResult<T> = std::result::Result<T, String>;

#[tauri::command]
pub async fn companion_scan(state: State<'_, AppState>, request: Option<ScanRequest>) -> CommandResult<Vec<crate::companion::protocol::DiscoveredDevice>> {
    state.manager().scan(request).await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_connect(app: AppHandle, state: State<'_, AppState>, request: ConnectRequest) -> CommandResult<ConnectionStatus> {
    state
        .manager()
        .connect(&app, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_disconnect(state: State<'_, AppState>) -> CommandResult<ConnectionStatus> {
    state.manager().disconnect().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_connection_status(state: State<'_, AppState>) -> CommandResult<ConnectionStatus> {
    state.manager().status().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_hello(state: State<'_, AppState>) -> CommandResult<Value> {
    state.manager().hello().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_capabilities(state: State<'_, AppState>) -> CommandResult<Value> {
    state.manager().capabilities().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_ping(state: State<'_, AppState>, request: Option<PingRequest>) -> CommandResult<Value> {
    state.manager().ping(request).await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_pair_begin(state: State<'_, AppState>, request: PairBeginRequest) -> CommandResult<Value> {
    state.manager().pair_begin(request).await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_pair_status(state: State<'_, AppState>) -> CommandResult<Value> {
    state.manager().pair_status().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_pair_cancel(state: State<'_, AppState>) -> CommandResult<Value> {
    state.manager().pair_cancel().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_auth(state: State<'_, AppState>, request: Option<AuthRequest>) -> CommandResult<Value> {
    state.manager().auth(request).await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_trusted_list(state: State<'_, AppState>) -> CommandResult<Value> {
    state.manager().trusted_list().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_trusted_revoke(state: State<'_, AppState>, request: TrustedRevokeRequest) -> CommandResult<Value> {
    state.manager().trusted_revoke(request).await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_snapshot(state: State<'_, AppState>) -> CommandResult<Value> {
    state.manager().snapshot().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_playback_status(state: State<'_, AppState>) -> CommandResult<Value> {
    state.manager().playback_status().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_playback_control(state: State<'_, AppState>, request: PlaybackControlRequest) -> CommandResult<Value> {
    state.manager().playback_control(request).await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_library_album(state: State<'_, AppState>) -> CommandResult<Value> {
    state.manager().library_album().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_library_tracks(state: State<'_, AppState>, request: Option<PageRequest>) -> CommandResult<Value> {
    state
        .manager()
        .library_track_page(request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_wifi_status(state: State<'_, AppState>) -> CommandResult<Value> {
    state.manager().wifi_status().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_wifi_scan_start(state: State<'_, AppState>) -> CommandResult<Value> {
    state.manager().wifi_scan_start().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_wifi_scan_results(state: State<'_, AppState>, request: Option<PageRequest>) -> CommandResult<Value> {
    state
        .manager()
        .wifi_scan_results(request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_wifi_connect(state: State<'_, AppState>, request: WifiConnectRequest) -> CommandResult<Value> {
    state.manager().wifi_connect(request).await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_wifi_connect_slot(state: State<'_, AppState>, request: WifiConnectSlotRequest) -> CommandResult<Value> {
    state
        .manager()
        .wifi_connect_slot(request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_wifi_disconnect(state: State<'_, AppState>) -> CommandResult<Value> {
    state.manager().wifi_disconnect().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_wifi_autoreconnect(state: State<'_, AppState>, request: ToggleRequest) -> CommandResult<Value> {
    state
        .manager()
        .wifi_autoreconnect(request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_lastfm_status(state: State<'_, AppState>) -> CommandResult<Value> {
    state.manager().lastfm_status().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_lastfm_control(state: State<'_, AppState>, request: LastfmControlRequest) -> CommandResult<Value> {
    state
        .manager()
        .lastfm_control(request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_history_summary(state: State<'_, AppState>) -> CommandResult<Value> {
    state.manager().history_summary().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_history_albums(state: State<'_, AppState>, request: Option<PageRequest>) -> CommandResult<Value> {
    state
        .manager()
        .history_album_page(request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_bt_status(state: State<'_, AppState>) -> CommandResult<Value> {
    state.manager().bt_audio_status().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn companion_bt_control(state: State<'_, AppState>, request: BtControlRequest) -> CommandResult<Value> {
    state.manager().bt_audio_control(request).await.map_err(|error| error.to_string())
}