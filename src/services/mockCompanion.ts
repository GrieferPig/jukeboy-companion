import type {
  BluetoothAction,
  CommandRequest,
  CompanionEventPayload,
  CompanionTransport,
  ConnectionStatus,
  LastfmAction,
  OutputTarget,
  PlaybackAction,
  PlaybackMode,
} from "./companion";

type Listener = (payload: CompanionEventPayload) => void;

interface MockTrack {
  title: string;
  artist: string;
  duration_sec: number;
  file_num: number;
}

interface MockState {
  connected: boolean;
  generation: number;
  startedAt: number;
  authenticated: boolean;
  pairing_pending: boolean;
  pairing_progress: number;
  pairing_required: number;
  pending_client_id: string;
  pending_app_name: string;
  button_sequence: string[];
  playing: boolean;
  paused: boolean;
  track_index: number;
  position_sec: number;
  volume_percent: number;
  playback_mode: PlaybackMode;
  output_target: OutputTarget;
  wifi_state: string;
  wifi_internet: boolean;
  wifi_autoreconnect: boolean;
  wifi_active_slot: number | null;
  wifi_preferred_slot: number | null;
  wifi_ip: string | null;
  lastfm_auth_url: string;
  lastfm_username: string;
  lastfm_has_token: boolean;
  lastfm_has_session: boolean;
  lastfm_scrobbling: boolean;
  lastfm_now_playing: boolean;
  bluetooth_a2dp_connected: boolean;
  bluetooth_bonded_count: number;
  tracks: MockTrack[];
}

const cartridgeChecksum = 0x4a554b45;

function createState(): MockState {
  return {
    connected: false,
    generation: 1,
    startedAt: Date.now(),
    authenticated: true,
    pairing_pending: false,
    pairing_progress: 0,
    pairing_required: 4,
    pending_client_id: "",
    pending_app_name: "",
    button_sequence: [],
    playing: true,
    paused: false,
    track_index: 0,
    position_sec: 42,
    volume_percent: 62,
    playback_mode: "sequential",
    output_target: "i2s",
    wifi_state: "connected",
    wifi_internet: true,
    wifi_autoreconnect: true,
    wifi_active_slot: 0,
    wifi_preferred_slot: 0,
    wifi_ip: "192.168.4.42",
    lastfm_auth_url: "https://ws.audioscrobbler.com/2.0",
    lastfm_username: "mock-listener",
    lastfm_has_token: true,
    lastfm_has_session: true,
    lastfm_scrobbling: true,
    lastfm_now_playing: true,
    bluetooth_a2dp_connected: false,
    bluetooth_bonded_count: 2,
    tracks: [
      { title: "Signal Mirror", artist: "Test Pressing", duration_sec: 241, file_num: 1 },
      { title: "Immediate Event", artist: "Test Pressing", duration_sec: 198, file_num: 2 },
      { title: "Heartbeat Window", artist: "Firmware Choir", duration_sec: 225, file_num: 3 },
      { title: "Queue Depth Sixteen", artist: "Firmware Choir", duration_sec: 264, file_num: 4 },
      { title: "Pairing Sequence", artist: "Button Matrix", duration_sec: 211, file_num: 5 },
    ],
  };
}

function requestRecord(request: CommandRequest): Record<string, unknown> {
  return typeof request === "object" && request !== null ? request as Record<string, unknown> : {};
}

function pageRequest(request: CommandRequest, defaultCount: number): { offset: number; count: number } {
  const record = requestRecord(request);
  return {
    offset: typeof record.offset === "number" ? record.offset : 0,
    count: typeof record.count === "number" ? record.count : defaultCount,
  };
}

class BrowserMockCompanionTransport implements CompanionTransport {
  private readonly listeners = new Set<Listener>();
  private readonly state = createState();
  private heartbeatId: number | null = null;

  async invoke<T>(command: string, request?: CommandRequest): Promise<T> {
    const response = this.dispatch(command, request);
    return response as T;
  }

  async listen(handler: Listener): Promise<() => void> {
    this.listeners.add(handler);
    this.startHeartbeat();
    return () => {
      this.listeners.delete(handler);
      if (this.listeners.size === 0 && this.heartbeatId !== null) {
        window.clearInterval(this.heartbeatId);
        this.heartbeatId = null;
      }
    };
  }

  private dispatch(command: string, request?: CommandRequest): unknown {
    switch (command) {
      case "companion_scan":
        return [{ address: "MO:CK:BE:EF:00:01", name: "MOCK_JUKEBOY", service_match: true, uuids: ["0000abf0-0000-1000-8000-00805f9b34fb"] }];
      case "companion_connect":
        this.state.connected = true;
        this.touch("link_connected");
        return this.connectionStatus();
      case "companion_disconnect":
        this.state.connected = false;
        this.emit({ frame_type: "event", event: "link_disconnected" });
        return this.connectionStatus();
      case "companion_connection_status":
        return this.connectionStatus();
      case "companion_hello":
        return this.hello();
      case "companion_capabilities":
        return this.capabilities();
      case "companion_ping":
        return { opcode: "ping", request_id: null, echo: String(requestRecord(request).text ?? "ping") };
      case "companion_pair_begin":
        return this.beginPairing();
      case "companion_pair_status":
        return this.pairing("pair_status");
      case "companion_pair_cancel":
        this.state.pairing_pending = false;
        this.state.pairing_progress = 0;
        this.state.pending_client_id = "";
        this.state.pending_app_name = "";
        this.state.button_sequence = [];
        this.touch("pairing");
        return this.pairing("pair_cancel");
      case "companion_auth":
        this.state.authenticated = true;
        this.touch("auth");
        return { opcode: "auth_proof", request_id: null, authenticated: true, client_id: "mock-client", trusted_client_count: 1 };
      case "companion_trusted_list":
        return this.trustedList();
      case "companion_snapshot":
      case "companion_playback_status":
      case "companion_wifi_status":
      case "companion_lastfm_status":
      case "companion_history_summary":
      case "companion_bt_status":
        return this.snapshot(command.replace("companion_", ""));
      case "companion_playback_control":
        this.playbackControl(request);
        return this.touch("playback", "playback_control");
      case "companion_library_album":
        return this.libraryAlbum();
      case "companion_library_tracks":
        return this.trackPage(request);
      case "companion_wifi_scan_start":
        this.state.wifi_state = "scanning";
        return this.touch("wifi", "wifi_scan_start");
      case "companion_wifi_scan_results":
        this.state.wifi_state = "connected";
        return this.wifiScanResults(request);
      case "companion_wifi_connect":
        this.state.wifi_state = "connected";
        this.state.wifi_internet = true;
        this.state.wifi_ip = "192.168.4.42";
        this.state.wifi_active_slot = 0;
        return this.touch("wifi", "wifi_connect");
      case "companion_wifi_connect_slot":
        this.state.wifi_state = "connected";
        this.state.wifi_internet = true;
        this.state.wifi_active_slot = Number(requestRecord(request).slot ?? 0);
        return this.touch("wifi", "wifi_connect_slot");
      case "companion_wifi_disconnect":
        this.state.wifi_state = "disconnected";
        this.state.wifi_internet = false;
        this.state.wifi_ip = null;
        this.state.wifi_active_slot = null;
        return this.touch("wifi", "wifi_disconnect");
      case "companion_wifi_autoreconnect":
        this.state.wifi_autoreconnect = Boolean(requestRecord(request).enabled);
        return this.touch("wifi", "wifi_autoreconnect");
      case "companion_lastfm_control":
        this.lastfmControl(request);
        return this.touch("lastfm", "lastfm_control");
      case "companion_history_albums":
        return this.historyAlbums(request);
      case "companion_bt_control":
        this.bluetoothControl(request);
        return this.touch("bluetooth", "bt_audio_control");
      default:
        throw new Error(`Unknown mock command: ${command}`);
    }
  }

  private connectionStatus(): ConnectionStatus {
    return {
      connected: this.state.connected,
      device: this.state.connected ? { address: "MO:CK:BE:EF:00:01", name: "MOCK_JUKEBOY", profile: "mock" } : null,
    };
  }

  private hello() {
    return { opcode: "hello", request_id: null, authenticated: this.state.authenticated, client_id: "mock-client", trusted_client_count: 1, app_name: "jukeboy-companion-mock", protocol_version: 1 };
  }

  private capabilities() {
    return { ...this.hello(), opcode: "capabilities", max_frame: 2048, mtu: 512, max_payload: 2036, feature_bits: 65535, pairing: this.pairing("pair_status") };
  }

  private snapshot(opcode = "snapshot"): CompanionEventPayload {
    const track = this.state.tracks[this.state.track_index % this.state.tracks.length];
    return {
      opcode,
      request_id: null,
      generation: this.state.generation,
      uptime_ms: Date.now() - this.state.startedAt,
      auth: { authenticated: this.state.authenticated, client_id: "mock-client", trusted_client_count: 1 },
      pairing: this.pairing("pair_status"),
      playback: {
        playing: this.state.playing,
        paused: this.state.paused,
        cartridge_checksum: cartridgeChecksum,
        track_index: this.state.track_index,
        track_count: this.state.tracks.length,
        position_sec: this.state.position_sec,
        started_at: 1777744000,
        duration_sec: track.duration_sec,
        volume_percent: this.state.volume_percent,
        playback_mode: this.state.playback_mode,
        track_title: track.title,
        track_artist: track.artist,
        track_file: `${String(track.file_num).padStart(3, "0")}.jbt`,
        output_target: this.state.output_target,
      },
      cartridge: { status: "ready", mounted: true, checksum: cartridgeChecksum, metadata_version: 1, track_count: this.state.tracks.length },
      wifi: { state: this.state.wifi_state, internet: this.state.wifi_internet, autoreconnect: this.state.wifi_autoreconnect, active_slot: this.state.wifi_active_slot, preferred_slot: this.state.wifi_preferred_slot, ip: this.state.wifi_ip },
      lastfm: { has_auth_url: Boolean(this.state.lastfm_auth_url), has_token: this.state.lastfm_has_token, has_session: this.state.lastfm_has_session, busy: false, scrobbling: this.state.lastfm_scrobbling, now_playing: this.state.lastfm_now_playing, pending_commands: 0, pending_scrobbles: 1, successful: 12, failed: 0, auth_url: this.state.lastfm_auth_url, username: this.state.lastfm_username },
      history: { album_count: 2, track_count: this.state.tracks.length },
      bluetooth: { a2dp_connected: this.state.bluetooth_a2dp_connected, bonded_count: this.state.bluetooth_bonded_count },
    };
  }

  private pairing(opcode: string) {
    return { opcode, request_id: null, pairing_pending: this.state.pairing_pending, pairing_progress: this.state.pairing_progress, pairing_required: this.state.pairing_required, pending_client_id: this.state.pending_client_id, pending_app_name: this.state.pending_app_name, button_sequence: this.state.button_sequence };
  }

  private trustedList() {
    return { opcode: "trusted_list", request_id: null, trusted_count: 1, clients: [{ client_id: "mock-client", app_name: "jukeboy-companion", created_at: 1777744000 }] };
  }

  private beginPairing() {
    this.state.pairing_pending = true;
    this.state.pairing_progress = 0;
    this.state.pending_client_id = "mock-client";
    this.state.pending_app_name = "jukeboy-companion";
    this.state.button_sequence = ["main1", "main2", "misc1", "misc3"];
    const pairing = this.pairing("pair_begin");
    this.state.generation += 1;
    this.emit({ ...pairing, frame_type: "event" });
    return pairing;
  }

  private playbackControl(request: CommandRequest): void {
    const record = requestRecord(request);
    const action = String(record.action ?? "next") as PlaybackAction;
    if (action === "next") this.state.track_index = (this.state.track_index + 1) % this.state.tracks.length;
    if (action === "prev") this.state.track_index = (this.state.track_index + this.state.tracks.length - 1) % this.state.tracks.length;
    if (action === "pause") {
      this.state.paused = !this.state.paused;
      this.state.playing = !this.state.paused;
    }
    if (action === "seek") this.state.position_sec = Number(record.value ?? 0);
    if (action === "volume") this.state.volume_percent = Number(record.value ?? 0);
    if (action === "mode") this.state.playback_mode = String(record.mode ?? "sequential") as PlaybackMode;
    if (action === "output") this.state.output_target = String(record.output_target ?? "i2s") as OutputTarget;
    if (action === "play_index") this.state.track_index = Number(record.value ?? 0) % this.state.tracks.length;
  }

  private lastfmControl(request: CommandRequest): void {
    const record = requestRecord(request);
    const action = String(record.action ?? "token") as LastfmAction;
    if (action === "set_auth_url") this.state.lastfm_auth_url = String(record.url ?? "");
    if (action === "token") this.state.lastfm_has_token = true;
    if (action === "auth") {
      this.state.lastfm_username = String(record.username ?? "mock-listener");
      this.state.lastfm_has_session = true;
    }
    if (action === "logout") {
      this.state.lastfm_has_token = false;
      this.state.lastfm_has_session = false;
      this.state.lastfm_username = "";
    }
    if (action === "scrobble") this.state.lastfm_scrobbling = Boolean(record.enabled);
    if (action === "now_playing") this.state.lastfm_now_playing = Boolean(record.enabled);
  }

  private bluetoothControl(request: CommandRequest): void {
    const action = String(requestRecord(request).action ?? "connect-last") as BluetoothAction;
    this.state.bluetooth_a2dp_connected = action !== "disconnect";
    if (action !== "disconnect") this.state.output_target = "bluetooth";
  }

  private libraryAlbum() {
    return { opcode: "library_album", request_id: null, cartridge: { status: "ready", mounted: true, checksum: cartridgeChecksum, metadata_version: 1, track_count: this.state.tracks.length }, album: { name: "Mock Cartridge", artist: "Jukeboy Test Rig", description: "Deterministic firmware-style state for browser and Tauri sync tests.", year: 2026, duration_sec: this.state.tracks.reduce((total, track) => total + track.duration_sec, 0), genre: "Diagnostics" } };
  }

  private trackPage(request: CommandRequest) {
    const { offset, count } = pageRequest(request, 24);
    const tracks = this.state.tracks.slice(offset, offset + count).map((track, index) => ({ track_index: offset + index, title: track.title, artist: track.artist, duration_sec: track.duration_sec, file_num: track.file_num }));
    return { opcode: "library_track_page", request_id: null, offset, track_count: this.state.tracks.length, returned_count: tracks.length, tracks };
  }

  private wifiScanResults(request: CommandRequest) {
    const { offset, count } = pageRequest(request, 12);
    const allResults = [
      { ssid: "Jukeboy Lab", rssi: -42, channel: 6, authmode: 3 },
      { ssid: "Stage Router", rssi: -61, channel: 11, authmode: 3 },
      { ssid: "Open Bench", rssi: -73, channel: 1, authmode: 0 },
    ];
    const results = allResults.slice(offset, offset + count);
    return { opcode: "wifi_scan_results", request_id: null, offset, total_count: allResults.length, returned_count: results.length, results };
  }

  private historyAlbums(request: CommandRequest) {
    const { offset, count } = pageRequest(request, 8);
    const allAlbums = [
      { checksum: cartridgeChecksum, track_count: this.state.tracks.length, first_seen_sequence: 1, last_seen_sequence: 12, album_name: "Mock Cartridge", album_artist: "Jukeboy Test Rig" },
      { checksum: 0x510e2026, track_count: 8, first_seen_sequence: 13, last_seen_sequence: 19, album_name: "Previous Fixture", album_artist: "Regression Suite" },
    ];
    const albums = allAlbums.slice(offset, offset + count);
    return { opcode: "history_album_page", request_id: null, offset, album_count: allAlbums.length, returned_count: albums.length, albums };
  }

  private touch(event: string, opcode = "snapshot"): CompanionEventPayload {
    this.state.generation += 1;
    const snapshot = this.snapshot(opcode);
    this.emit({ ...snapshot, frame_type: "event", event });
    return snapshot;
  }

  private startHeartbeat(): void {
    if (this.heartbeatId !== null) return;
    this.heartbeatId = window.setInterval(() => {
      this.emit({ opcode: "snapshot", frame_type: "heartbeat", request_id: 0, uptime_ms: Date.now() - this.state.startedAt, generation: this.state.generation, authenticated: this.state.authenticated, queue_free: 16, rx_frames: 0, tx_frames: 0, rx_errors: 0 });
    }, 5000);
  }

  private emit(payload: CompanionEventPayload): void {
    queueMicrotask(() => {
      for (const listener of this.listeners) {
        listener(payload);
      }
    });
  }
}

export function createBrowserMockTransport(): CompanionTransport {
  return new BrowserMockCompanionTransport();
}