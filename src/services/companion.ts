import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { createBrowserMockTransport } from "./mockCompanion";

export const FRAME_EVENT_NAME = "companion://frame";

export type PlaybackMode = "sequential" | "single_repeat" | "shuffle";
export type OutputTarget = "bluetooth" | "i2s";
export type PlaybackAction =
  | "next"
  | "prev"
  | "pause"
  | "ff"
  | "rewind"
  | "play_index"
  | "seek"
  | "volume"
  | "mode"
  | "output";
export type LastfmAction =
  | "set_auth_url"
  | "token"
  | "auth"
  | "logout"
  | "scrobble"
  | "now_playing";
export type BluetoothAction = "connect-last" | "pair-best" | "disconnect";
export type CommandRequest = object | null | undefined;
export type CompanionEventPayload = Record<string, unknown>;

export interface CompanionTransport {
  invoke<T>(command: string, request?: CommandRequest): Promise<T>;
  listen(handler: (payload: CompanionEventPayload) => void): Promise<UnlistenFn>;
}

export interface ConnectedDevice {
  address: string;
  name: string;
  profile: string;
}

export interface DiscoveredDevice {
  address: string;
  name: string;
  service_match: boolean;
  uuids: string[];
}

export interface ConnectionStatus {
  connected: boolean;
  device: ConnectedDevice | null;
}

export interface AuthState {
  authenticated: boolean;
  client_id: string;
  trusted_client_count: number;
}

export interface PairingState {
  pairing_pending: boolean;
  pairing_progress: number;
  pairing_required: number;
  pending_client_id: string;
  pending_app_name: string;
  button_sequence: string[];
}

export interface PlaybackState {
  playing: boolean;
  paused: boolean;
  cartridge_checksum: number | null;
  track_index: number | null;
  track_count: number | null;
  position_sec: number | null;
  started_at: number | null;
  duration_sec: number | null;
  volume_percent: number | null;
  playback_mode: PlaybackMode | null;
  track_title: string;
  track_artist: string;
  track_file: string;
  output_target: OutputTarget | null;
}

export interface CartridgeState {
  status: string | null;
  mounted: boolean;
  checksum: number | null;
  metadata_version: number | null;
  track_count: number | null;
}

export interface WifiState {
  state: string | null;
  internet: boolean;
  autoreconnect: boolean;
  active_slot: number | null;
  preferred_slot: number | null;
  ip: string | null;
}

export interface LastfmState {
  has_auth_url: boolean;
  has_token: boolean;
  has_session: boolean;
  busy: boolean;
  scrobbling: boolean;
  now_playing: boolean;
  pending_commands: number;
  pending_scrobbles: number;
  successful: number;
  failed: number;
  auth_url: string;
  username: string;
}

export interface HistoryState {
  album_count: number;
  track_count: number;
}

export interface BluetoothState {
  a2dp_connected: boolean;
  bonded_count: number;
}

export interface SnapshotResponse {
  opcode: string;
  request_id: number | null;
  generation: number | null;
  uptime_ms: number | null;
  auth: AuthState;
  pairing: PairingState;
  playback: PlaybackState;
  cartridge: CartridgeState;
  wifi: WifiState;
  lastfm: LastfmState;
  history: HistoryState;
  bluetooth: BluetoothState;
}

export interface HelloResponse extends AuthState {
  opcode: string;
  request_id: number | null;
  app_name: string;
  protocol_version: number | null;
}

export interface CapabilitiesResponse extends AuthState {
  opcode: string;
  request_id: number | null;
  app_name: string;
  protocol_version: number | null;
  max_frame: number | null;
  mtu: number | null;
  max_payload: number | null;
  feature_bits: number | null;
  pairing: PairingState;
}

export interface LibraryAlbumResponse {
  opcode: string;
  request_id: number | null;
  cartridge: CartridgeState;
  album: {
    name: string;
    artist: string;
    description: string;
    year: number | null;
    duration_sec: number | null;
    genre: string;
  };
}

export interface LibraryTrack {
  track_index: number;
  title: string;
  artist: string;
  duration_sec: number | null;
  file_num: number | null;
}

export interface TrackPageResponse {
  opcode: string;
  request_id: number | null;
  offset: number;
  track_count: number;
  returned_count: number;
  tracks: LibraryTrack[];
}

export interface WifiScanResult {
  ssid: string;
  rssi: number | null;
  channel: number | null;
  authmode: number | null;
}

export interface WifiScanResultsResponse {
  opcode: string;
  request_id: number | null;
  offset: number;
  total_count: number;
  returned_count: number;
  results: WifiScanResult[];
}

export interface HistoryAlbum {
  checksum: number;
  track_count: number | null;
  first_seen_sequence: number | null;
  last_seen_sequence: number | null;
  album_name: string;
  album_artist: string;
}

export interface HistoryAlbumPageResponse {
  opcode: string;
  request_id: number | null;
  offset: number;
  album_count: number;
  returned_count: number;
  albums: HistoryAlbum[];
}

export interface TrustedClient {
  client_id: string;
  app_name: string;
  created_at: number;
}

export interface TrustedListResponse {
  opcode: string;
  request_id: number | null;
  trusted_count: number;
  clients: TrustedClient[];
}

export interface ScanRequest {
  scan_timeout_secs?: number;
}

export interface ConnectRequest {
  address?: string;
  name?: string;
  profile?: string;
  client_id?: string;
  app_name?: string;
  secret_hex?: string;
  timeout_secs?: number;
  scan_timeout_secs?: number;
}

export interface PlaybackControlRequest {
  action: PlaybackAction;
  value?: number;
  mode?: PlaybackMode;
  output_target?: OutputTarget;
}

export interface PageRequest {
  offset?: number;
  count?: number;
}

export interface WifiConnectRequest {
  ssid: string;
  password?: string;
}

export interface PairBeginRequest {
  client_id?: string;
  app_name?: string;
  secret_hex?: string;
  sequence?: string[];
  wait?: boolean;
  wait_timeout_secs?: number;
}

export interface PairBeginResponse {
  credentials_saved_to?: string;
  profile?: string;
  profiles?: string[];
  client_id?: string;
  app_name?: string;
  secret_hex?: string;
  button_sequence?: string[];
  pair_status?: PairingState;
  pairing_pending?: boolean;
  pairing_progress?: number;
  pairing_required?: number;
  pending_client_id?: string;
  pending_app_name?: string;
}

export interface LastfmControlRequest {
  action: LastfmAction;
  url?: string;
  username?: string;
  password?: string;
  enabled?: boolean;
}

const tauriTransport: CompanionTransport = {
  invoke<T>(command: string, request?: CommandRequest): Promise<T> {
    if (request === undefined || request === null) {
      return invoke<T>(command);
    }
    return invoke<T>(command, { request });
  },
  listen(handler: (payload: CompanionEventPayload) => void): Promise<UnlistenFn> {
    return listen<CompanionEventPayload>(FRAME_EVENT_NAME, (event) => {
      handler(event.payload ?? {});
    });
  },
};

let configuredTransport: CompanionTransport | null = null;
let browserMockTransport: CompanionTransport | null = null;
let commandQueueTail: Promise<void> = Promise.resolve();

function shouldLogToDevConsole(): boolean {
  return typeof console !== "undefined" && Boolean(import.meta.env.DEV);
}

export function logCompanionConsole(scope: string, status: string, details?: unknown): void {
  if (!shouldLogToDevConsole()) {
    return;
  }

  const label = `[companion:${scope}] ${status}`;
  if (details === undefined) {
    console.info(label);
    return;
  }

  if (status.includes("error")) {
    console.error(label, details);
    return;
  }

  console.info(label, details);
}

export function setCompanionTransport(transport: CompanionTransport | null): void {
  configuredTransport = transport;
  // Test helpers swap transports between cases; reset the queue boundary so
  // commands from an old transport cannot bleed into a new one.
  commandQueueTail = Promise.resolve();
}

function shouldUseBrowserMock(): boolean {
  if (typeof window === "undefined") {
    return false;
  }

  const params = new URLSearchParams(window.location.search);
  return params.get("mock") === "1" || params.get("backend") === "mock";
}

function resolveTransport(): CompanionTransport {
  if (configuredTransport) {
    return configuredTransport;
  }
  if (browserMockTransport) {
    return browserMockTransport;
  }
  if (shouldUseBrowserMock()) {
    browserMockTransport ??= createBrowserMockTransport();
    return browserMockTransport;
  }
  return tauriTransport;
}

async function invokeCommand<T>(command: string, request?: CommandRequest): Promise<T> {
  const transport = resolveTransport();
  const previous = commandQueueTail;
  let releaseQueue!: () => void;
  commandQueueTail = new Promise<void>((resolve) => {
    releaseQueue = resolve;
  });

  await previous;
  logCompanionConsole("command", `${command}:start`, request ?? null);

  try {
    const response = await transport.invoke<T>(command, request);
    logCompanionConsole("command", `${command}:success`, response);
    return response;
  } catch (error) {
    logCompanionConsole("command", `${command}:error`, error);
    throw error;
  } finally {
    releaseQueue();
  }
}

export function runCommand<T = unknown>(command: string, request?: CommandRequest): Promise<T> {
  return invokeCommand<T>(command, request);
}

export function listenToFrames(handler: (payload: CompanionEventPayload) => void): Promise<UnlistenFn> {
  return resolveTransport().listen((payload) => {
    logCompanionConsole(
      "frame",
      typeof payload.opcode === "string" ? String(payload.opcode) : "received",
      payload,
    );
    handler(payload);
  });
}

export function scan(request?: ScanRequest): Promise<DiscoveredDevice[]> {
  return invokeCommand("companion_scan", request);
}

export function connect(request: ConnectRequest): Promise<ConnectionStatus> {
  return invokeCommand("companion_connect", request);
}

export function disconnect(): Promise<ConnectionStatus> {
  return invokeCommand("companion_disconnect");
}

export function connectionStatus(): Promise<ConnectionStatus> {
  return invokeCommand("companion_connection_status");
}

export function hello(): Promise<HelloResponse> {
  return invokeCommand("companion_hello");
}

export function capabilities(): Promise<CapabilitiesResponse> {
  return invokeCommand("companion_capabilities");
}

export function snapshot(): Promise<SnapshotResponse> {
  return invokeCommand("companion_snapshot");
}

export function playbackControl(request: PlaybackControlRequest): Promise<unknown> {
  return invokeCommand("companion_playback_control", request);
}

export function libraryAlbum(): Promise<LibraryAlbumResponse> {
  return invokeCommand("companion_library_album");
}

export function libraryTracks(request?: PageRequest): Promise<TrackPageResponse> {
  return invokeCommand("companion_library_tracks", request);
}

export function wifiScanStart(): Promise<unknown> {
  return invokeCommand("companion_wifi_scan_start");
}

export function wifiScanResults(request?: PageRequest): Promise<WifiScanResultsResponse> {
  return invokeCommand("companion_wifi_scan_results", request);
}

export function wifiConnect(request: WifiConnectRequest): Promise<unknown> {
  return invokeCommand("companion_wifi_connect", request);
}

export function wifiConnectSlot(slot: number): Promise<unknown> {
  return invokeCommand("companion_wifi_connect_slot", { slot });
}

export function wifiDisconnect(): Promise<unknown> {
  return invokeCommand("companion_wifi_disconnect");
}

export function wifiAutoreconnect(enabled: boolean): Promise<unknown> {
  return invokeCommand("companion_wifi_autoreconnect", { enabled });
}

export function lastfmControl(request: LastfmControlRequest): Promise<unknown> {
  return invokeCommand("companion_lastfm_control", request);
}

export function historyAlbums(request?: PageRequest): Promise<HistoryAlbumPageResponse> {
  return invokeCommand("companion_history_albums", request);
}

export function trustedList(): Promise<TrustedListResponse> {
  return invokeCommand("companion_trusted_list");
}

export function trustedRevoke(client_id: string): Promise<unknown> {
  return invokeCommand("companion_trusted_revoke", { client_id });
}

export function pairBegin(request: PairBeginRequest): Promise<PairBeginResponse> {
  return invokeCommand("companion_pair_begin", request);
}

export function pairStatus(): Promise<PairingState> {
  return invokeCommand("companion_pair_status");
}

export function pairCancel(): Promise<PairingState> {
  return invokeCommand("companion_pair_cancel");
}

export function bluetoothControl(action: BluetoothAction): Promise<unknown> {
  return invokeCommand("companion_bt_control", { action });
}