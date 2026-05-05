use std::{sync::Arc, time::Instant};

use serde_json::{json, Value};
use tokio::{
    sync::{broadcast, Mutex},
    task::JoinHandle,
    time::{interval, Duration, MissedTickBehavior},
};

use crate::companion::{
    error::Result,
    protocol::{
        button_id_to_name, BtAction, ConnectedDevice, DiscoveredDevice, LastfmAction,
        PlaybackAction,
    },
};

const MOCK_ADDRESS: &str = "MO:CK:BE:EF:00:01";
const MOCK_NAME: &str = "MOCK_JUKEBOY";
const MOCK_SERVICE_UUID: &str = "0000abf0-0000-1000-8000-00805f9b34fb";

pub fn mock_mode_enabled() -> bool {
    std::env::var("JUKEBOY_COMPANION_BACKEND")
        .map(|value| value.eq_ignore_ascii_case("mock"))
        .unwrap_or(false)
}

pub fn request_selects_mock(
    address: Option<&str>,
    name: Option<&str>,
    profile: Option<&str>,
) -> bool {
    mock_mode_enabled()
        || profile
            .map(|value| value.eq_ignore_ascii_case("mock"))
            .unwrap_or(false)
        || address
            .map(|value| {
                value.eq_ignore_ascii_case("mock") || value.eq_ignore_ascii_case(MOCK_ADDRESS)
            })
            .unwrap_or(false)
        || name
            .map(|value| {
                value.eq_ignore_ascii_case("mock") || value.eq_ignore_ascii_case(MOCK_NAME)
            })
            .unwrap_or(false)
}

#[derive(Clone)]
struct MockTrack {
    title: &'static str,
    artist: &'static str,
    duration_sec: u32,
    file_num: u32,
}

#[derive(Clone)]
struct MockTrustedClient {
    client_id: String,
    app_name: String,
    created_at: u32,
}

#[derive(Clone)]
struct MockWifiNetwork {
    ssid: &'static str,
    rssi: i32,
    channel: u8,
    authmode: u8,
}

struct MockState {
    started_at: Instant,
    generation: u32,
    rx_frames: u32,
    tx_frames: u32,
    rx_errors: u32,
    authenticated: bool,
    client_id: String,
    pairing_pending: bool,
    pairing_progress: u8,
    pairing_required: u8,
    pending_client_id: String,
    pending_app_name: String,
    button_sequence: Vec<String>,
    playing: bool,
    paused: bool,
    track_index: u32,
    position_sec: u32,
    volume_percent: u8,
    playback_mode: &'static str,
    output_target: &'static str,
    cartridge_checksum: u32,
    wifi_state: &'static str,
    wifi_internet: bool,
    wifi_autoreconnect: bool,
    wifi_active_slot: Option<u8>,
    wifi_preferred_slot: Option<u8>,
    wifi_ip: Option<&'static str>,
    lastfm_has_auth_url: bool,
    lastfm_has_token: bool,
    lastfm_has_session: bool,
    lastfm_busy: bool,
    lastfm_scrobbling: bool,
    lastfm_now_playing: bool,
    lastfm_pending_commands: u32,
    lastfm_pending_scrobbles: u32,
    lastfm_successful: u32,
    lastfm_failed: u32,
    lastfm_auth_url: String,
    lastfm_username: String,
    bluetooth_a2dp_connected: bool,
    bluetooth_bonded_count: u32,
    trusted_clients: Vec<MockTrustedClient>,
    tracks: Vec<MockTrack>,
    wifi_networks: Vec<MockWifiNetwork>,
}

impl Default for MockState {
    fn default() -> Self {
        Self {
            started_at: Instant::now(),
            generation: 1,
            rx_frames: 0,
            tx_frames: 0,
            rx_errors: 0,
            authenticated: true,
            client_id: "mock-client".into(),
            pairing_pending: false,
            pairing_progress: 0,
            pairing_required: 4,
            pending_client_id: String::new(),
            pending_app_name: String::new(),
            button_sequence: Vec::new(),
            playing: true,
            paused: false,
            track_index: 0,
            position_sec: 42,
            volume_percent: 62,
            playback_mode: "sequential",
            output_target: "i2s",
            cartridge_checksum: 0x4a55_4b45,
            wifi_state: "connected",
            wifi_internet: true,
            wifi_autoreconnect: true,
            wifi_active_slot: Some(0),
            wifi_preferred_slot: Some(0),
            wifi_ip: Some("192.168.4.42"),
            lastfm_has_auth_url: true,
            lastfm_has_token: true,
            lastfm_has_session: true,
            lastfm_busy: false,
            lastfm_scrobbling: true,
            lastfm_now_playing: true,
            lastfm_pending_commands: 0,
            lastfm_pending_scrobbles: 1,
            lastfm_successful: 12,
            lastfm_failed: 0,
            lastfm_auth_url: "https://ws.audioscrobbler.com/2.0".into(),
            lastfm_username: "mock-listener".into(),
            bluetooth_a2dp_connected: false,
            bluetooth_bonded_count: 2,
            trusted_clients: vec![MockTrustedClient {
                client_id: "mock-client".into(),
                app_name: "jukeboy-companion".into(),
                created_at: 1_777_744_000,
            }],
            tracks: vec![
                MockTrack {
                    title: "Signal Mirror",
                    artist: "Test Pressing",
                    duration_sec: 241,
                    file_num: 1,
                },
                MockTrack {
                    title: "Immediate Event",
                    artist: "Test Pressing",
                    duration_sec: 198,
                    file_num: 2,
                },
                MockTrack {
                    title: "Heartbeat Window",
                    artist: "Firmware Choir",
                    duration_sec: 225,
                    file_num: 3,
                },
                MockTrack {
                    title: "Queue Depth Sixteen",
                    artist: "Firmware Choir",
                    duration_sec: 264,
                    file_num: 4,
                },
                MockTrack {
                    title: "Pairing Sequence",
                    artist: "Button Matrix",
                    duration_sec: 211,
                    file_num: 5,
                },
            ],
            wifi_networks: vec![
                MockWifiNetwork {
                    ssid: "Jukeboy Lab",
                    rssi: -42,
                    channel: 6,
                    authmode: 3,
                },
                MockWifiNetwork {
                    ssid: "Stage Router",
                    rssi: -61,
                    channel: 11,
                    authmode: 3,
                },
                MockWifiNetwork {
                    ssid: "Open Bench",
                    rssi: -73,
                    channel: 1,
                    authmode: 0,
                },
            ],
        }
    }
}

impl MockState {
    fn touch_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1).max(1);
    }

    fn current_track(&self) -> &MockTrack {
        &self.tracks[self.track_index as usize % self.tracks.len()]
    }

    fn uptime_ms(&self) -> u32 {
        self.started_at.elapsed().as_millis().min(u32::MAX as u128) as u32
    }

    fn pair_status(&self, opcode: &str) -> Value {
        json!({
            "opcode": opcode,
            "request_id": null,
            "pairing_pending": self.pairing_pending,
            "pairing_progress": self.pairing_progress,
            "pairing_required": self.pairing_required,
            "pending_client_id": self.pending_client_id,
            "pending_app_name": self.pending_app_name,
            "button_sequence": self.button_sequence,
        })
    }

    fn trusted_list(&self) -> Value {
        json!({
            "opcode": "trusted_list",
            "request_id": null,
            "trusted_count": self.trusted_clients.len(),
            "clients": self.trusted_clients.iter().map(|client| json!({
                "client_id": client.client_id,
                "app_name": client.app_name,
                "created_at": client.created_at,
            })).collect::<Vec<_>>(),
        })
    }

    fn snapshot(&self, opcode: &str) -> Value {
        let current_track = self.current_track();
        json!({
            "opcode": opcode,
            "request_id": null,
            "generation": self.generation,
            "uptime_ms": self.uptime_ms(),
            "auth": {
                "authenticated": self.authenticated,
                "client_id": self.client_id,
                "trusted_client_count": self.trusted_clients.len(),
            },
            "pairing": {
                "pairing_pending": self.pairing_pending,
                "pairing_progress": self.pairing_progress,
                "pairing_required": self.pairing_required,
                "pending_client_id": self.pending_client_id,
                "pending_app_name": self.pending_app_name,
                "button_sequence": self.button_sequence,
            },
            "playback": {
                "playing": self.playing,
                "paused": self.paused,
                "cartridge_checksum": self.cartridge_checksum,
                "track_index": self.track_index,
                "track_count": self.tracks.len(),
                "position_sec": self.position_sec,
                "started_at": 1_777_744_000,
                "duration_sec": current_track.duration_sec,
                "volume_percent": self.volume_percent,
                "playback_mode": self.playback_mode,
                "track_title": current_track.title,
                "track_artist": current_track.artist,
                "track_file": format!("{:03}.jbt", current_track.file_num),
                "output_target": self.output_target,
            },
            "cartridge": {
                "status": "ready",
                "mounted": true,
                "checksum": self.cartridge_checksum,
                "metadata_version": 1,
                "track_count": self.tracks.len(),
            },
            "wifi": {
                "state": self.wifi_state,
                "internet": self.wifi_internet,
                "autoreconnect": self.wifi_autoreconnect,
                "active_slot": self.wifi_active_slot,
                "preferred_slot": self.wifi_preferred_slot,
                "ip": self.wifi_ip,
            },
            "lastfm": {
                "has_auth_url": self.lastfm_has_auth_url,
                "has_token": self.lastfm_has_token,
                "has_session": self.lastfm_has_session,
                "busy": self.lastfm_busy,
                "scrobbling": self.lastfm_scrobbling,
                "now_playing": self.lastfm_now_playing,
                "pending_commands": self.lastfm_pending_commands,
                "pending_scrobbles": self.lastfm_pending_scrobbles,
                "successful": self.lastfm_successful,
                "failed": self.lastfm_failed,
                "auth_url": self.lastfm_auth_url,
                "username": self.lastfm_username,
            },
            "history": {
                "album_count": 2,
                "track_count": self.tracks.len(),
            },
            "bluetooth": {
                "a2dp_connected": self.bluetooth_a2dp_connected,
                "bonded_count": self.bluetooth_bonded_count,
            },
        })
    }

    fn event(&self, event: &str) -> Value {
        let mut payload = self.snapshot("snapshot");
        payload["frame_type"] = json!("event");
        payload["event"] = json!(event);
        payload
    }

    fn heartbeat(&self) -> Value {
        json!({
            "opcode": "snapshot",
            "frame_type": "heartbeat",
            "request_id": 0,
            "uptime_ms": self.uptime_ms(),
            "generation": self.generation,
            "authenticated": self.authenticated,
            "queue_free": 16,
            "rx_frames": self.rx_frames,
            "tx_frames": self.tx_frames,
            "rx_errors": self.rx_errors,
        })
    }

    fn library_album(&self) -> Value {
        json!({
            "opcode": "library_album",
            "request_id": null,
            "cartridge": {
                "status": "ready",
                "mounted": true,
                "checksum": self.cartridge_checksum,
                "metadata_version": 1,
                "track_count": self.tracks.len(),
            },
            "album": {
                "name": "Mock Cartridge",
                "artist": "Jukeboy Test Rig",
                "description": "Deterministic firmware-style state for browser and Tauri sync tests.",
                "year": 2026,
                "duration_sec": self.tracks.iter().map(|track| track.duration_sec).sum::<u32>(),
                "genre": "Diagnostics",
            },
        })
    }

    fn track_page(&self, offset: u32, count: u32) -> Value {
        let tracks = self
            .tracks
            .iter()
            .enumerate()
            .skip(offset as usize)
            .take(count as usize)
            .map(|(track_index, track)| {
                json!({
                    "track_index": track_index,
                    "title": track.title,
                    "artist": track.artist,
                    "duration_sec": track.duration_sec,
                    "file_num": track.file_num,
                })
            })
            .collect::<Vec<_>>();

        json!({
            "opcode": "library_track_page",
            "request_id": null,
            "offset": offset,
            "track_count": self.tracks.len(),
            "returned_count": tracks.len(),
            "tracks": tracks,
        })
    }

    fn wifi_scan_results(&self, offset: u32, count: u32) -> Value {
        let results = self
            .wifi_networks
            .iter()
            .skip(offset as usize)
            .take(count as usize)
            .map(|network| {
                json!({
                    "ssid": network.ssid,
                    "rssi": network.rssi,
                    "channel": network.channel,
                    "authmode": network.authmode,
                })
            })
            .collect::<Vec<_>>();

        json!({
            "opcode": "wifi_scan_results",
            "request_id": null,
            "offset": offset,
            "total_count": self.wifi_networks.len(),
            "returned_count": results.len(),
            "results": results,
        })
    }

    fn history_album_page(&self, offset: u32, count: u32) -> Value {
        let albums = vec![
            json!({
                "checksum": self.cartridge_checksum,
                "track_count": self.tracks.len(),
                "first_seen_sequence": 1,
                "last_seen_sequence": 12,
                "album_name": "Mock Cartridge",
                "album_artist": "Jukeboy Test Rig",
            }),
            json!({
                "checksum": 0x510E_2026u32,
                "track_count": 8,
                "first_seen_sequence": 13,
                "last_seen_sequence": 19,
                "album_name": "Previous Fixture",
                "album_artist": "Regression Suite",
            }),
        ];
        let selected = albums
            .into_iter()
            .skip(offset as usize)
            .take(count as usize)
            .collect::<Vec<_>>();

        json!({
            "opcode": "history_album_page",
            "request_id": null,
            "offset": offset,
            "album_count": 2,
            "returned_count": selected.len(),
            "albums": selected,
        })
    }
}

pub struct MockCompanionBackend {
    state: Arc<Mutex<MockState>>,
    event_tx: broadcast::Sender<Value>,
    heartbeat_task: Option<JoinHandle<()>>,
    connected_device: ConnectedDevice,
}

impl MockCompanionBackend {
    pub async fn scan() -> Result<Vec<DiscoveredDevice>> {
        Ok(vec![DiscoveredDevice {
            address: MOCK_ADDRESS.into(),
            name: MOCK_NAME.into(),
            service_match: true,
            uuids: vec![MOCK_SERVICE_UUID.into()],
        }])
    }

    pub async fn connect(profile: &str) -> Result<Self> {
        let state = Arc::new(Mutex::new(MockState::default()));
        let (event_tx, _) = broadcast::channel(64);
        let heartbeat_state = Arc::clone(&state);
        let heartbeat_tx = event_tx.clone();
        let heartbeat_task = tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(5));
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                ticker.tick().await;
                let payload = heartbeat_state.lock().await.heartbeat();
                let _ = heartbeat_tx.send(payload);
            }
        });

        let backend = Self {
            state,
            event_tx,
            heartbeat_task: Some(heartbeat_task),
            connected_device: ConnectedDevice {
                address: MOCK_ADDRESS.into(),
                name: MOCK_NAME.into(),
                profile: profile.to_string(),
            },
        };
        backend.emit_snapshot_event("link_connected").await;
        Ok(backend)
    }

    pub fn connected_device(&self) -> &ConnectedDevice {
        &self.connected_device
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<Value> {
        self.event_tx.subscribe()
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(task) = self.heartbeat_task.take() {
            task.abort();
        }
        self.emit_snapshot_event("link_disconnected").await;
        Ok(())
    }

    pub async fn hello(&self) -> Result<Value> {
        let state = self.state.lock().await;
        Ok(json!({
            "opcode": "hello",
            "request_id": null,
            "authenticated": state.authenticated,
            "client_id": state.client_id,
            "trusted_client_count": state.trusted_clients.len(),
            "app_name": "jukeboy-companion-mock",
            "protocol_version": 1,
        }))
    }

    pub async fn capabilities(&self) -> Result<Value> {
        let state = self.state.lock().await;
        Ok(json!({
            "opcode": "capabilities",
            "request_id": null,
            "authenticated": state.authenticated,
            "client_id": state.client_id,
            "trusted_client_count": state.trusted_clients.len(),
            "app_name": "jukeboy-companion-mock",
            "protocol_version": 1,
            "max_frame": 2048,
            "mtu": 512,
            "max_payload": 2036,
            "feature_bits": 0xffff,
            "pairing": {
                "pairing_pending": state.pairing_pending,
                "pairing_progress": state.pairing_progress,
                "pairing_required": state.pairing_required,
                "pending_client_id": state.pending_client_id,
                "pending_app_name": state.pending_app_name,
                "button_sequence": state.button_sequence,
            },
        }))
    }

    pub async fn ping(&self, text: &str) -> Result<Value> {
        Ok(json!({
            "opcode": "ping",
            "request_id": null,
            "echo": text,
            "echo_hex": hex::encode(text.as_bytes()),
        }))
    }

    pub async fn pair_begin(
        &self,
        client_id: &str,
        app_name: &str,
        _secret: &[u8],
        sequence: &[u8],
    ) -> Result<Value> {
        let mut state = self.state.lock().await;
        state.pairing_pending = true;
        state.pairing_progress = 0;
        state.pending_client_id = client_id.into();
        state.pending_app_name = app_name.into();
        state.button_sequence = sequence
            .iter()
            .map(|button| button_id_to_name(*button).to_string())
            .collect();
        state.touch_generation();
        let pair_status = state.pair_status("pair_begin");
        let mut event = pair_status.clone();
        event["frame_type"] = json!("event");
        drop(state);
        let _ = self.event_tx.send(event);
        Ok(pair_status)
    }

    pub async fn pair_status(&self) -> Result<Value> {
        Ok(self.state.lock().await.pair_status("pair_status"))
    }

    pub async fn pair_cancel(&self) -> Result<Value> {
        let mut state = self.state.lock().await;
        state.pairing_pending = false;
        state.pairing_progress = 0;
        state.pending_client_id.clear();
        state.pending_app_name.clear();
        state.button_sequence.clear();
        state.touch_generation();
        let pair_status = state.pair_status("pair_cancel");
        let mut event = pair_status.clone();
        event["frame_type"] = json!("event");
        drop(state);
        let _ = self.event_tx.send(event);
        Ok(pair_status)
    }

    pub async fn auth_challenge(&self, client_id: &str) -> Result<Value> {
        Ok(json!({
            "opcode": "auth_challenge",
            "request_id": null,
            "client_id": client_id,
            "nonce_hex": "000102030405060708090a0b0c0d0e0f",
        }))
    }

    pub async fn auth_proof(
        &self,
        client_id: &str,
        _secret: &[u8],
        _nonce: &[u8],
    ) -> Result<Value> {
        let mut state = self.state.lock().await;
        state.authenticated = true;
        state.client_id = client_id.into();
        state.touch_generation();
        let trusted_count = state.trusted_clients.len();
        drop(state);
        self.emit_snapshot_event("auth").await;
        Ok(json!({
            "opcode": "auth_proof",
            "request_id": null,
            "authenticated": true,
            "client_id": client_id,
            "trusted_client_count": trusted_count,
        }))
    }

    pub async fn trusted_list(&self) -> Result<Value> {
        Ok(self.state.lock().await.trusted_list())
    }

    pub async fn trusted_revoke(&self, client_id: &str) -> Result<Value> {
        let mut state = self.state.lock().await;
        state
            .trusted_clients
            .retain(|client| client.client_id != client_id);
        state.touch_generation();
        let response = state.trusted_list();
        drop(state);
        self.emit_snapshot_event("trusted").await;
        Ok(response)
    }

    pub async fn snapshot(&self) -> Result<Value> {
        Ok(self.state.lock().await.snapshot("snapshot"))
    }

    pub async fn playback_status(&self) -> Result<Value> {
        Ok(self.state.lock().await.snapshot("playback_status"))
    }

    pub async fn playback_control(
        &self,
        action: PlaybackAction,
        value: Option<u32>,
    ) -> Result<Value> {
        let mut state = self.state.lock().await;
        match action {
            PlaybackAction::Next => {
                state.track_index = (state.track_index + 1) % state.tracks.len() as u32;
                state.position_sec = 0;
                state.playing = true;
                state.paused = false;
            }
            PlaybackAction::Previous => {
                state.track_index = if state.track_index == 0 {
                    state.tracks.len() as u32 - 1
                } else {
                    state.track_index - 1
                };
                state.position_sec = 0;
                state.playing = true;
                state.paused = false;
            }
            PlaybackAction::PauseToggle => {
                state.paused = !state.paused;
                state.playing = !state.paused;
            }
            PlaybackAction::FastForward => {
                state.position_sec = state.position_sec.saturating_add(15);
            }
            PlaybackAction::Rewind => {
                state.position_sec = state.position_sec.saturating_sub(15);
            }
            PlaybackAction::PlayIndex => {
                state.track_index = value.unwrap_or(0).min(state.tracks.len() as u32 - 1);
                state.position_sec = 0;
                state.playing = true;
                state.paused = false;
            }
            PlaybackAction::SeekSeconds => {
                state.position_sec = value.unwrap_or(0);
            }
            PlaybackAction::SetVolumePercent => {
                state.volume_percent = value.unwrap_or(0).min(100) as u8;
            }
            PlaybackAction::SetMode => {
                state.playback_mode = match value.unwrap_or(0) {
                    1 => "single_repeat",
                    2 => "shuffle",
                    _ => "sequential",
                };
            }
            PlaybackAction::SetOutputTarget => {
                state.output_target = if value.unwrap_or(0) == 1 {
                    "i2s"
                } else {
                    "bluetooth"
                };
            }
        }
        state.touch_generation();
        let response = state.snapshot("playback_control");
        drop(state);
        self.emit_snapshot_event("playback").await;
        Ok(response)
    }

    pub async fn library_album(&self) -> Result<Value> {
        Ok(self.state.lock().await.library_album())
    }

    pub async fn library_track_page(&self, offset: u32, count: u32) -> Result<Value> {
        Ok(self.state.lock().await.track_page(offset, count))
    }

    pub async fn wifi_status(&self) -> Result<Value> {
        Ok(self.state.lock().await.snapshot("wifi_status"))
    }

    pub async fn wifi_scan_start(&self) -> Result<Value> {
        let mut state = self.state.lock().await;
        state.wifi_state = "scanning";
        state.touch_generation();
        let response = state.snapshot("wifi_scan_start");
        drop(state);
        self.emit_snapshot_event("wifi").await;
        Ok(response)
    }

    pub async fn wifi_scan_results(&self, offset: u32, count: u32) -> Result<Value> {
        let mut state = self.state.lock().await;
        if state.wifi_state == "scanning" {
            state.wifi_state = "connected";
            state.touch_generation();
        }
        Ok(state.wifi_scan_results(offset, count))
    }

    pub async fn wifi_connect(&self, ssid: &str, _password: &str) -> Result<Value> {
        let mut state = self.state.lock().await;
        state.wifi_state = "connected";
        state.wifi_internet = true;
        state.wifi_ip = Some(if ssid == "Stage Router" {
            "10.10.0.42"
        } else {
            "192.168.4.42"
        });
        state.wifi_active_slot = Some(0);
        state.wifi_preferred_slot = Some(0);
        state.touch_generation();
        let response = state.snapshot("wifi_connect");
        drop(state);
        self.emit_snapshot_event("wifi").await;
        Ok(response)
    }

    pub async fn wifi_connect_slot(&self, slot: u32) -> Result<Value> {
        let mut state = self.state.lock().await;
        state.wifi_state = "connected";
        state.wifi_internet = true;
        state.wifi_active_slot = Some(slot.min(2) as u8);
        state.wifi_preferred_slot = state.wifi_active_slot;
        state.wifi_ip = Some("192.168.4.42");
        state.touch_generation();
        let response = state.snapshot("wifi_connect_slot");
        drop(state);
        self.emit_snapshot_event("wifi").await;
        Ok(response)
    }

    pub async fn wifi_disconnect(&self) -> Result<Value> {
        let mut state = self.state.lock().await;
        state.wifi_state = "disconnected";
        state.wifi_internet = false;
        state.wifi_ip = None;
        state.wifi_active_slot = None;
        state.touch_generation();
        let response = state.snapshot("wifi_disconnect");
        drop(state);
        self.emit_snapshot_event("wifi").await;
        Ok(response)
    }

    pub async fn wifi_autoreconnect(&self, enabled: bool) -> Result<Value> {
        let mut state = self.state.lock().await;
        state.wifi_autoreconnect = enabled;
        state.touch_generation();
        let response = state.snapshot("wifi_autoreconnect");
        drop(state);
        self.emit_snapshot_event("wifi").await;
        Ok(response)
    }

    pub async fn lastfm_status(&self) -> Result<Value> {
        Ok(self.state.lock().await.snapshot("lastfm_status"))
    }

    pub async fn lastfm_control(
        &self,
        action: LastfmAction,
        auth_url: Option<&str>,
        username: Option<&str>,
        _password: Option<&str>,
        enabled: Option<bool>,
    ) -> Result<Value> {
        let mut state = self.state.lock().await;
        match action {
            LastfmAction::SetAuthUrl => {
                state.lastfm_auth_url = auth_url.unwrap_or_default().into();
                state.lastfm_has_auth_url = !state.lastfm_auth_url.is_empty();
            }
            LastfmAction::RequestToken => {
                state.lastfm_has_token = true;
                state.lastfm_pending_commands = state.lastfm_pending_commands.saturating_add(1);
            }
            LastfmAction::Auth => {
                state.lastfm_username = username.unwrap_or("mock-listener").into();
                state.lastfm_has_session = true;
                state.lastfm_busy = false;
            }
            LastfmAction::Logout => {
                state.lastfm_has_token = false;
                state.lastfm_has_session = false;
                state.lastfm_username.clear();
            }
            LastfmAction::SetScrobbling => {
                state.lastfm_scrobbling = enabled.unwrap_or(true);
            }
            LastfmAction::SetNowPlaying => {
                state.lastfm_now_playing = enabled.unwrap_or(true);
            }
        }
        state.touch_generation();
        let response = state.snapshot("lastfm_control");
        drop(state);
        self.emit_snapshot_event("lastfm").await;
        Ok(response)
    }

    pub async fn history_summary(&self) -> Result<Value> {
        Ok(self.state.lock().await.snapshot("history_summary"))
    }

    pub async fn history_album_page(&self, offset: u32, count: u32) -> Result<Value> {
        Ok(self.state.lock().await.history_album_page(offset, count))
    }

    pub async fn bt_audio_status(&self) -> Result<Value> {
        Ok(self.state.lock().await.snapshot("bt_audio_status"))
    }

    pub async fn bt_audio_control(&self, action: BtAction) -> Result<Value> {
        let mut state = self.state.lock().await;
        match action {
            BtAction::ConnectLast | BtAction::PairBest => {
                state.bluetooth_a2dp_connected = true;
                state.output_target = "bluetooth";
            }
            BtAction::Disconnect => {
                state.bluetooth_a2dp_connected = false;
            }
        }
        state.touch_generation();
        let response = state.snapshot("bt_audio_control");
        drop(state);
        self.emit_snapshot_event("bluetooth").await;
        Ok(response)
    }

    async fn emit_snapshot_event(&self, event: &str) {
        let payload = self.state.lock().await.event(event);
        let _ = self.event_tx.send(payload);
    }
}
