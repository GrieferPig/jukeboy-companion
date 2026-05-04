mod commands;
pub mod companion;

use commands::{
    companion_auth, companion_bt_control, companion_bt_status, companion_capabilities,
    companion_connect, companion_connection_status, companion_disconnect, companion_hello,
    companion_history_albums, companion_history_summary, companion_lastfm_control,
    companion_lastfm_status, companion_library_album, companion_library_tracks,
    companion_pair_begin, companion_pair_cancel, companion_pair_status, companion_ping,
    companion_playback_control, companion_playback_status, companion_scan, companion_snapshot,
    companion_trusted_list, companion_trusted_revoke, companion_wifi_autoreconnect,
    companion_wifi_connect, companion_wifi_connect_slot, companion_wifi_disconnect,
    companion_wifi_scan_results, companion_wifi_scan_start, companion_wifi_status,
};
use companion::AppState;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! You've been greeted from Rust!")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            companion_scan,
            companion_connect,
            companion_disconnect,
            companion_connection_status,
            companion_hello,
            companion_capabilities,
            companion_ping,
            companion_pair_begin,
            companion_pair_status,
            companion_pair_cancel,
            companion_auth,
            companion_trusted_list,
            companion_trusted_revoke,
            companion_snapshot,
            companion_playback_status,
            companion_playback_control,
            companion_library_album,
            companion_library_tracks,
            companion_wifi_status,
            companion_wifi_scan_start,
            companion_wifi_scan_results,
            companion_wifi_connect,
            companion_wifi_connect_slot,
            companion_wifi_disconnect,
            companion_wifi_autoreconnect,
            companion_lastfm_status,
            companion_lastfm_control,
            companion_history_summary,
            companion_history_albums,
            companion_bt_status,
            companion_bt_control,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
