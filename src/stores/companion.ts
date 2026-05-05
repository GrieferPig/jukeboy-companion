import { defineStore } from "pinia";
import { computed, reactive, ref, watch } from "vue";

import {
  bluetoothControl,
  capabilities,
  connect,
  connectionStatus,
  disconnect,
  hello,
  historyAlbums,
  lastfmControl,
  libraryAlbum,
  libraryTracks,
  listenToFrames,
  pairBegin,
  pairCancel,
  pairStatus,
  playbackControl,
  scan,
  snapshot,
  trustedList,
  trustedRevoke,
  wifiAutoreconnect,
  wifiConnect,
  wifiConnectSlot,
  wifiDisconnect,
  wifiScanResults,
  wifiScanStart,
  type CapabilitiesResponse,
  type CompanionEventPayload,
  type ConnectRequest,
  type ConnectionStatus,
  type DiscoveredDevice,
  type HistoryAlbumPageResponse,
  type HelloResponse,
  type LibraryAlbumResponse,
  type OutputTarget,
  type PairBeginResponse,
  type PairBeginRequest,
  type PairingState,
  type PlaybackMode,
  type SnapshotResponse,
  type TrackPageResponse,
  type TrustedListResponse,
  type WifiScanResultsResponse,
  logCompanionConsole,
} from "../services/companion";

const LIBRARY_TRACK_PAGE_SIZE = 8;
const WIFI_SCAN_PAGE_SIZE = 8;
const HISTORY_PAGE_SIZE = 4;
const AUTO_CONNECT_SCAN_TIMEOUT_SECS = 3;
const AUTO_CONNECT_RETRY_MS = 12_000;
const SNAPSHOT_POLL_INTERVAL_MS = 3_000;
const SNAPSHOT_FULL_SYNC_EVERY = 4;

type AutomationState = "idle" | "scanning" | "connecting" | "pairing" | "syncing" | "paused";

interface CompanionIssue {
  action: string;
  title: string;
  detail: string;
  recovery: string;
  background: boolean;
  signature: string;
  time: string;
}

interface ScanDevicesOptions {
  background?: boolean;
  activityKey?: string;
}

interface ConnectToDeviceOptions {
  background?: boolean;
  activityKey?: string;
}

interface PairingActionOptions {
  background?: boolean;
  activityKey?: string;
}

const ACTION_LABELS: Record<string, string> = {
  initialize: "Companion startup",
  autoScan: "Background discovery",
  scanDevices: "Bluetooth scan",
  autoConnectToDevice: "Background connection",
  connectToDevice: "Device connection",
  disconnectDevice: "Disconnect request",
  autoPairing: "Background pairing",
  beginPairing: "Pairing request",
  refreshConnection: "Connection status refresh",
  refreshSnapshot: "Device snapshot sync",
  refreshHello: "Handshake refresh",
  refreshCapabilities: "Capabilities refresh",
  refreshPairing: "Pairing status refresh",
  refreshTrusted: "Trusted client refresh",
  refreshLibraryAlbum: "Album refresh",
  refreshLibraryTracks: "Track list refresh",
  refreshHistory: "History refresh",
  startWifiScan: "Wi-Fi scan",
  connectWifi: "Wi-Fi connect",
  connectWifiSlot: "Wi-Fi slot connect",
  disconnectWifiNetwork: "Wi-Fi disconnect",
  setWifiAutoreconnect: "Wi-Fi auto-reconnect update",
  playbackAction: "Playback command",
  runBluetoothAction: "Bluetooth command",
  setLastfmAuthUrl: "Last.fm auth URL update",
  authenticateLastfm: "Last.fm authentication",
  setLastfmScrobbling: "Last.fm scrobble setting",
  logoutLastfm: "Last.fm logout",
  cancelPairingRequest: "Pairing cancel request",
  revokeTrustedClient: "Trusted client revoke",
};

function createDefaultConnection(): ConnectionStatus {
  return {
    connected: false,
    device: null,
  };
}

function createDefaultPairingState(): PairingState {
  return {
    pairing_pending: false,
    pairing_progress: 0,
    pairing_required: 0,
    pending_client_id: "",
    pending_app_name: "",
    button_sequence: [],
  };
}

function createDefaultSnapshot(): SnapshotResponse {
  return {
    opcode: "snapshot",
    request_id: null,
    generation: null,
    uptime_ms: null,
    auth: {
      authenticated: false,
      client_id: "",
      trusted_client_count: 0,
    },
    pairing: createDefaultPairingState(),
    playback: {
      playing: false,
      paused: false,
      cartridge_checksum: null,
      track_index: null,
      track_count: null,
      position_sec: null,
      started_at: null,
      duration_sec: null,
      volume_percent: null,
      playback_mode: null,
      track_title: "",
      track_artist: "",
      track_file: "",
      output_target: null,
    },
    cartridge: {
      status: null,
      mounted: false,
      checksum: null,
      metadata_version: null,
      track_count: null,
    },
    wifi: {
      state: null,
      internet: false,
      autoreconnect: false,
      active_slot: null,
      preferred_slot: null,
      ip: null,
    },
    lastfm: {
      has_auth_url: false,
      has_token: false,
      has_session: false,
      busy: false,
      scrobbling: false,
      now_playing: false,
      pending_commands: 0,
      pending_scrobbles: 0,
      successful: 0,
      failed: 0,
      auth_url: "",
      username: "",
    },
    history: {
      album_count: 0,
      track_count: 0,
    },
    bluetooth: {
      a2dp_connected: false,
      bonded_count: 0,
    },
  };
}

function createDefaultHello(): HelloResponse {
  return {
    opcode: "hello",
    request_id: null,
    authenticated: false,
    client_id: "",
    trusted_client_count: 0,
    app_name: "",
    protocol_version: null,
  };
}

function createDefaultCapabilities(): CapabilitiesResponse {
  return {
    ...createDefaultHello(),
    max_frame: null,
    mtu: null,
    max_payload: null,
    feature_bits: null,
    pairing: createDefaultPairingState(),
  };
}

function createDefaultLibraryAlbum(): LibraryAlbumResponse {
  return {
    opcode: "library_album",
    request_id: null,
    cartridge: createDefaultSnapshot().cartridge,
    album: {
      name: "",
      artist: "",
      description: "",
      year: null,
      duration_sec: null,
      genre: "",
    },
  };
}

function createDefaultTracks(): TrackPageResponse {
  return {
    opcode: "library_tracks",
    request_id: null,
    offset: 0,
    track_count: 0,
    returned_count: 0,
    tracks: [],
  };
}

function createDefaultWifiScan(): WifiScanResultsResponse {
  return {
    opcode: "wifi_scan_results",
    request_id: null,
    offset: 0,
    total_count: 0,
    returned_count: 0,
    results: [],
  };
}

function createDefaultHistoryAlbums(): HistoryAlbumPageResponse {
  return {
    opcode: "history_albums",
    request_id: null,
    offset: 0,
    album_count: 0,
    returned_count: 0,
    albums: [],
  };
}

function createDefaultTrustedList(): TrustedListResponse {
  return {
    opcode: "trusted_list",
    request_id: null,
    trusted_count: 0,
    clients: [],
  };
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function actionLabel(key: string): string {
  return ACTION_LABELS[key] ?? key;
}

function nowTimestamp(): string {
  return new Date().toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

function integerCommandValue(value: number, minimum = 0): number {
  if (!Number.isFinite(value)) {
    return minimum;
  }

  return Math.max(minimum, Math.round(value));
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function mergeRecord<T extends Record<string, unknown>>(current: T, patch: unknown): T {
  if (!isRecord(patch)) {
    return current;
  }
  return {
    ...current,
    ...patch,
  } as T;
}

async function runSequentially(steps: Array<() => Promise<void>>): Promise<void> {
  for (const step of steps) {
    await step();
  }
}

function snapshotPatchFromEvent(payload: CompanionEventPayload): Partial<SnapshotResponse> | null {
  const hasSnapshotField = ["auth", "pairing", "playback", "cartridge", "wifi", "lastfm", "history", "bluetooth"]
    .some((key) => isRecord(payload[key]));

  if (!hasSnapshotField && typeof payload.generation !== "number" && typeof payload.uptime_ms !== "number") {
    return null;
  }

  return payload as Partial<SnapshotResponse>;
}

function pairingStateFromUnknown(value: unknown): PairingState | null {
  if (!isRecord(value)) {
    return null;
  }

  const pairStatus = isRecord(value.pair_status) ? value.pair_status : value;
  if (typeof pairStatus.pairing_pending !== "boolean") {
    return null;
  }

  return {
    pairing_pending: Boolean(pairStatus.pairing_pending),
    pairing_progress: Number(pairStatus.pairing_progress ?? 0),
    pairing_required: Number(pairStatus.pairing_required ?? 0),
    pending_client_id: String(pairStatus.pending_client_id ?? ""),
    pending_app_name: String(pairStatus.pending_app_name ?? ""),
    button_sequence: Array.isArray(pairStatus.button_sequence)
      ? pairStatus.button_sequence.map((entry) => String(entry))
      : [],
  };
}

function isConnectionDroppedError(error: unknown): boolean {
  const normalized = errorMessage(error).toLowerCase();

  return normalized.includes("ble device is not connected")
    || normalized.includes("object has been closed")
    || normalized.includes("notification stream ended")
    || normalized.includes("ble session is not connected")
    || normalized.includes("ble device disconnected")
    || (normalized.includes("ble session") && normalized.includes("not available"));
}

function companionIssueFor(action: string, error: unknown, background = false): CompanionIssue {
  const raw = errorMessage(error);
  const normalized = raw.toLowerCase();
  let title = `${actionLabel(action)} failed`;
  let detail = raw;
  let recovery = background
    ? "The app will keep retrying this step in the background."
    : "Retry the action after checking the companion connection state.";

  if (normalized.includes("no bluetooth adapter")) {
    title = "Bluetooth unavailable";
    detail = "No Bluetooth adapter is currently available, so the app cannot discover or connect to a Jukeboy.";
    recovery = "Enable Bluetooth or attach a compatible adapter. Background discovery will resume automatically afterward.";
  } else if (normalized.includes("no jukeboy companion ble device found")) {
    title = "Jukeboy not found";
    detail = "Bluetooth discovery completed, but no compatible Jukeboy was advertising the companion service.";
    recovery = "Power on the device and keep it nearby. Background discovery will retry automatically.";
  } else if (normalized.includes("requested profile has no credentials")) {
    title = "Pairing required";
    detail = "The device is reachable, but there are no saved credentials for it yet.";
    recovery = "The app will start pairing in the background unless pairing has been paused.";
  } else if (
    normalized.includes("ble device is not connected")
    || normalized.includes("object has been closed")
    || normalized.includes("ble session is not connected")
    || normalized.includes("ble device disconnected")
    || (normalized.includes("ble session") && normalized.includes("not available"))
  ) {
    title = "Connection lost";
    detail = "The active BLE link dropped before the request finished.";
    recovery = "Keep the device awake and nearby. The app will rescan and reconnect automatically unless auto-connect is paused.";
  } else if (normalized.includes("operation timed out")) {
    title = "Device timed out";
    detail = "The device did not respond before the request timed out.";
    recovery = "Keep the device awake and nearby. Snapshot sync and connection attempts will retry automatically.";
  } else if (normalized.includes("authentication secret must be 32 bytes") || normalized.includes("authentication challenge nonce was invalid")) {
    title = "Saved credentials are invalid";
    detail = raw;
    recovery = "Start pairing again to replace the saved credentials for this device.";
  } else if (normalized.includes("btle plug error") || normalized.includes("android ble bridge error")) {
    title = "Bluetooth stack error";
    detail = raw;
    recovery = "Check system Bluetooth permissions and adapter state, then keep the app open to retry automatically.";
  } else if (normalized.includes("companion api error")) {
    title = "Device rejected the request";
    detail = raw;
    recovery = "Check pairing, authentication, and current device state before retrying the action.";
  }

  return {
    action,
    title,
    detail,
    recovery,
    background,
    signature: `${background ? "bg" : "fg"}:${title}:${detail}:${recovery}`,
    time: nowTimestamp(),
  };
}

export interface FrameLogEntry {
  id: string;
  command: string;
  time: string;
  payload: CompanionEventPayload;
}

interface PerformOptions<T> {
  background?: boolean;
  assign?: (value: T) => void;
}

export const useCompanionStore = defineStore("companion", () => {
  const initialized = ref(false);
  const frameListenerState = ref("idle");
  const lastError = ref<string | null>(null);
  const activeIssue = ref<CompanionIssue | null>(null);
  const automationState = ref<AutomationState>("idle");
  const automationMessage = ref("Background discovery will start once the companion app is ready.");
  const autoConnectPaused = ref(false);
  const autoPairPaused = ref(false);

  const activity = reactive<Record<string, boolean>>({
    initialize: false,
  });
  const errors = reactive<Record<string, string>>({});

  const connection = ref(createDefaultConnection());
  const snapshotState = ref(createDefaultSnapshot());
  const helloState = ref(createDefaultHello());
  const capabilitiesState = ref(createDefaultCapabilities());
  const pairingState = ref(createDefaultPairingState());
  const albumState = ref(createDefaultLibraryAlbum());
  const tracksState = ref(createDefaultTracks());
  const wifiScanState = ref(createDefaultWifiScan());
  const historyAlbumsState = ref(createDefaultHistoryAlbums());
  const trustedClientsState = ref(createDefaultTrustedList());
  const discoveredDevices = ref<DiscoveredDevice[]>([]);
  const frameLog = ref<FrameLogEntry[]>([]);
  const heartbeat = ref<CompanionEventPayload | null>(null);

  let frameStopper: (() => void) | null = null;
  let frameCounter = 0;
  let reconnectTimer: ReturnType<typeof setInterval> | null = null;
  let snapshotTimer: ReturnType<typeof setInterval> | null = null;
  let snapshotPollCounter = 0;
  let autoConnectPending = false;
  let autoPairPending = false;
  let snapshotPollPending = false;

  const isConnected = computed(() => connection.value.connected);
  const isAuthenticated = computed(() => {
    return (
      snapshotState.value.auth.authenticated
      || capabilitiesState.value.authenticated
      || helloState.value.authenticated
    );
  });
  const hasMountedCartridge = computed(() => {
    return snapshotState.value.cartridge.mounted
      || albumState.value.cartridge.mounted
      || tracksState.value.tracks.length > 0
      || (tracksState.value.track_count ?? 0) > 0;
  });
  const automationLabel = computed(() => {
    switch (automationState.value) {
      case "scanning":
        return "Background discovery";
      case "connecting":
        return "Connecting";
      case "pairing":
        return "Pairing in background";
      case "syncing":
        return "Live sync active";
      case "paused":
        return "Auto-connect paused";
      default:
        return connection.value.connected ? "Connected" : "Idle";
    }
  });
  const sessionPhase = computed(() => {
    if (!connection.value.connected) {
      return "disconnected";
    }
    if (!isAuthenticated.value) {
      return "awaiting-auth";
    }
    if (!hasMountedCartridge.value) {
      return "no-cartridge";
    }
    return "ready";
  });
  const sessionMessage = computed(() => {
    if (!connection.value.connected) {
      if (autoConnectPaused.value) {
        return "The companion session is offline because auto-connect is paused after a manual disconnect. Resume auto-connect in Settings when you want the app to reconnect on its own again.";
      }
      return "The companion session is offline, but background discovery is already scanning for a nearby Jukeboy and will connect automatically when one appears.";
    }
    if (!isAuthenticated.value) {
      if (pairingState.value.pairing_pending || autoPairPending) {
        return "Connected, but pairing is already running in the background. Open Settings if you need to watch the generated button sequence or pairing progress.";
      }
      return "Connected, but authentication is still pending. Saved credentials are being retried automatically, and pairing will start in the background if the device has never been trusted before.";
    }
    if (!hasMountedCartridge.value) {
      return "Connected, but no cartridge is inserted. Recently played albums are still available below, while the track list and current track controls wait for media.";
    }
    return "";
  });
  const statusLabel = computed(() => {
    if (activity.initialize) {
      return "Booting companion";
    }
    if (!connection.value.connected) {
      if (autoConnectPaused.value) {
        return "Disconnected · auto-connect paused";
      }
      if (automationState.value === "connecting") {
        return "Connecting in background";
      }
      if (automationState.value === "scanning") {
        return "Searching for Jukeboy";
      }
      return "Disconnected · background discovery armed";
    }
    if (connection.value.connected) {
      if (pairingState.value.pairing_pending || autoPairPending) {
        return `Connected to ${connection.value.device?.name ?? "device"} · pairing in background`;
      }
      if (!isAuthenticated.value) {
        return `Connected to ${connection.value.device?.name ?? "device"} · waiting for auth`;
      }
      if (automationState.value === "syncing") {
        return `Connected to ${connection.value.device?.name ?? "device"} · live sync active`;
      }
      if (!hasMountedCartridge.value) {
        return `Connected to ${connection.value.device?.name ?? "device"} · no cartridge`;
      }
      return `Connected to ${connection.value.device?.name ?? "device"}`;
    }
    return "Disconnected";
  });

  function syncAuthState(auth: Partial<SnapshotResponse["auth"]>): void {
    snapshotState.value = {
      ...snapshotState.value,
      auth: {
        ...snapshotState.value.auth,
        ...auth,
      },
    };
  }

  function clearErrors(): void {
    for (const key of Object.keys(errors)) {
      delete errors[key];
    }
    activeIssue.value = null;
    lastError.value = null;
  }

  function stopFrameListener(): void {
    frameStopper?.();
    frameStopper = null;
    stopReconnectLoop();
    stopSnapshotLoop();
    frameListenerState.value = "stopped";
    initialized.value = false;
    automationState.value = "idle";
    automationMessage.value = "Background discovery is idle.";
  }

  function clearFrameLog(): void {
    frameLog.value = [];
  }

  function resetSessionState(): void {
    snapshotState.value = createDefaultSnapshot();
    helloState.value = createDefaultHello();
    capabilitiesState.value = createDefaultCapabilities();
    pairingState.value = createDefaultPairingState();
    albumState.value = createDefaultLibraryAlbum();
    tracksState.value = createDefaultTracks();
    wifiScanState.value = createDefaultWifiScan();
    historyAlbumsState.value = createDefaultHistoryAlbums();
    trustedClientsState.value = createDefaultTrustedList();
    heartbeat.value = null;
  }

  function handleConnectionDropped(): void {
    connection.value = createDefaultConnection();
    resetSessionState();
    syncAutomationState();
  }

  function setAutomation(nextState: AutomationState, message: string): void {
    automationState.value = nextState;
    automationMessage.value = message;
  }

  function dismissIssue(): void {
    activeIssue.value = null;
    lastError.value = null;
  }

  function clearIssueForAction(action: string): void {
    if (activeIssue.value?.action === action) {
      activeIssue.value = null;
      lastError.value = null;
    }
  }

  function reportIssue(action: string, error: unknown, background = false): void {
    const issue = companionIssueFor(action, error, background);
    if (activeIssue.value?.signature !== issue.signature) {
      activeIssue.value = issue;
    }
    lastError.value = issue.detail;
  }

  function stopReconnectLoop(): void {
    if (reconnectTimer) {
      clearInterval(reconnectTimer);
      reconnectTimer = null;
    }
    autoConnectPending = false;
  }

  function stopSnapshotLoop(): void {
    if (snapshotTimer) {
      clearInterval(snapshotTimer);
      snapshotTimer = null;
    }
    snapshotPollPending = false;
    snapshotPollCounter = 0;
  }

  async function runSnapshotPollingTick(): Promise<void> {
    if (!initialized.value || !connection.value.connected || snapshotPollPending) {
      return;
    }

    snapshotPollPending = true;

    try {
      await refreshSnapshot(true);
      snapshotPollCounter += 1;

      if (!isAuthenticated.value) {
        await runSequentially([
          () => refreshHello(true),
          () => refreshCapabilities(true),
          () => refreshPairing(true),
        ]);
      }

      if (snapshotPollCounter % SNAPSHOT_FULL_SYNC_EVERY === 0) {
        await runSequentially([
          () => refreshHello(true),
          () => refreshCapabilities(true),
          () => refreshLibraryAlbum(true),
          () => refreshLibraryTracks(0, LIBRARY_TRACK_PAGE_SIZE, true),
          () => refreshHistory(0, HISTORY_PAGE_SIZE, true),
          () => refreshPairing(true),
          () => refreshTrusted(true),
        ]);
      }

      if (connection.value.connected && !isAuthenticated.value && !pairingState.value.pairing_pending && !autoPairPaused.value) {
        await ensureBackgroundPairing();
      }
    } finally {
      snapshotPollPending = false;
    }
  }

  async function ensureBackgroundPairing(): Promise<void> {
    if (
      autoPairPending
      || autoPairPaused.value
      || !connection.value.connected
      || isAuthenticated.value
      || pairingState.value.pairing_pending
    ) {
      return;
    }

    autoPairPending = true;
    setAutomation("pairing", "Connected. Pairing is starting automatically in the background.");

    try {
      await beginPairing({ wait: false, wait_timeout_secs: 0 }, { background: true, activityKey: "autoPairing" });
    } finally {
      autoPairPending = false;
    }
  }

  async function attemptAutoConnect(): Promise<void> {
    if (
      autoConnectPending
      || autoConnectPaused.value
      || !initialized.value
      || connection.value.connected
      || activity.connectToDevice
      || activity.disconnectDevice
      || activity.autoConnectToDevice
    ) {
      return;
    }

    autoConnectPending = true;
    setAutomation("scanning", "Scanning for a nearby Jukeboy in the background.");

    try {
      const devices = await scanDevices(AUTO_CONNECT_SCAN_TIMEOUT_SECS, {
        background: true,
        activityKey: "autoScan",
      });

      if (
        devices.length === 0
        || !initialized.value
        || autoConnectPaused.value
        || connection.value.connected
      ) {
        return;
      }

      const [candidate] = devices;
      setAutomation("connecting", `Found ${candidate.name || candidate.address}. Connecting in the background.`);
      await connectToDevice(
        {
          address: candidate.address,
          name: candidate.name,
        },
        {
          background: true,
          activityKey: "autoConnectToDevice",
        },
      );
    } finally {
      autoConnectPending = false;
    }
  }

  function syncAutomationState(): void {
    if (!initialized.value) {
      stopReconnectLoop();
      stopSnapshotLoop();
      if (!activity.initialize) {
        setAutomation("idle", "Background discovery is idle.");
      }
      return;
    }

    if (!connection.value.connected) {
      stopSnapshotLoop();
      if (autoConnectPaused.value) {
        stopReconnectLoop();
        setAutomation("paused", "Auto-connect is paused after a manual disconnect.");
        return;
      }

      setAutomation(
        autoConnectPending ? "connecting" : "scanning",
        autoConnectPending
          ? "Connecting to a discovered Jukeboy in the background."
          : "Scanning for a nearby Jukeboy in the background.",
      );

      if (!reconnectTimer) {
        reconnectTimer = setInterval(() => {
          void attemptAutoConnect();
        }, AUTO_CONNECT_RETRY_MS);
        void attemptAutoConnect();
      }
      return;
    }

    stopReconnectLoop();
    if (!snapshotTimer) {
      snapshotTimer = setInterval(() => {
        void runSnapshotPollingTick();
      }, SNAPSHOT_POLL_INTERVAL_MS);
    }

    if (pairingState.value.pairing_pending || autoPairPending) {
      setAutomation("pairing", "Connected. Pairing is active in the background.");
      return;
    }

    if (!isAuthenticated.value) {
      setAutomation(
        "syncing",
        autoPairPaused.value
          ? "Connected. Waiting for authentication to resume."
          : "Connected. Saved credentials are being checked in the background, and pairing will start automatically only if the device still is not authenticated afterward.",
      );
      return;
    }

    setAutomation("syncing", "Connected. Device snapshots are refreshing automatically.");
  }

  function resumeAutoConnect(): void {
    autoConnectPaused.value = false;
    autoPairPaused.value = false;
    clearIssueForAction("disconnectDevice");
    syncAutomationState();
    void attemptAutoConnect();
  }

  async function perform<T>(
    key: string,
    operation: () => Promise<T>,
    options: PerformOptions<T> = {},
  ): Promise<T | null> {
    activity[key] = true;
    delete errors[key];
    logCompanionConsole("action", `${key}:start`);

    try {
      const value = await operation();
      options.assign?.(value);
      clearIssueForAction(key);
      if (!options.background) {
        lastError.value = null;
      }
      logCompanionConsole("action", `${key}:success`, value);
      return value;
    } catch (error) {
      const issue = companionIssueFor(key, error, options.background);
      errors[key] = issue.detail;
      reportIssue(key, error, options.background);
      if (key !== "refreshConnection" && isConnectionDroppedError(error)) {
        handleConnectionDropped();
      }
      logCompanionConsole("action", `${key}:error`, issue);
      return null;
    } finally {
      activity[key] = false;
    }
  }

  async function refreshConnection(background = false): Promise<void> {
    const status = await perform("refreshConnection", () => connectionStatus(), {
      background,
      assign: (value) => {
        connection.value = value;
      },
    });

    if (!status?.connected) {
      resetSessionState();
    }
  }

  async function refreshSnapshot(background = false): Promise<void> {
    if (!connection.value.connected) {
      return;
    }

    const previousChecksum = snapshotState.value.cartridge.checksum;
    const nextSnapshot = await perform("refreshSnapshot", () => snapshot(), {
      background,
      assign: (value) => {
        snapshotState.value = value;
        pairingState.value = value.pairing;
      },
    });

    if (nextSnapshot && previousChecksum !== nextSnapshot.cartridge.checksum) {
      void refreshLibrary(true);
    }
  }

  async function refreshHello(background = false): Promise<void> {
    if (!connection.value.connected) {
      return;
    }

    await perform("refreshHello", () => hello(), {
      background,
      assign: (value) => {
        helloState.value = value;
        syncAuthState({
          authenticated: value.authenticated,
          client_id: value.client_id,
          trusted_client_count: value.trusted_client_count,
        });
      },
    });
  }

  async function refreshCapabilities(background = false): Promise<void> {
    if (!connection.value.connected) {
      return;
    }

    await perform("refreshCapabilities", () => capabilities(), {
      background,
      assign: (value) => {
        capabilitiesState.value = value;
        pairingState.value = mergeRecord(pairingState.value, value.pairing);
        snapshotState.value = {
          ...snapshotState.value,
          pairing: mergeRecord(snapshotState.value.pairing, value.pairing),
        };
        syncAuthState({
          authenticated: value.authenticated,
          client_id: value.client_id,
          trusted_client_count: value.trusted_client_count,
        });
      },
    });
  }

  async function refreshPairing(background = false): Promise<void> {
    if (!connection.value.connected) {
      return;
    }

    await perform("refreshPairing", () => pairStatus(), {
      background,
      assign: (value) => {
        pairingState.value = value;
      },
    });
  }

  async function refreshTrusted(background = false): Promise<void> {
    if (!connection.value.connected) {
      return;
    }

    await perform("refreshTrusted", () => trustedList(), {
      background,
      assign: (value) => {
        trustedClientsState.value = value;
      },
    });
  }

  async function refreshLibraryAlbum(background = false): Promise<void> {
    if (!connection.value.connected) {
      return;
    }

    await perform("refreshLibraryAlbum", () => libraryAlbum(), {
      background,
      assign: (value) => {
        albumState.value = value;
      },
    });
  }

  async function refreshLibraryTracks(offset = 0, count = LIBRARY_TRACK_PAGE_SIZE, background = false): Promise<void> {
    if (!connection.value.connected) {
      return;
    }

    await perform("refreshLibraryTracks", () => libraryTracks({ offset, count }), {
      background,
      assign: (value) => {
        tracksState.value = value;
      },
    });
  }

  async function refreshWifiScan(offset = 0, count = WIFI_SCAN_PAGE_SIZE, background = false): Promise<void> {
    if (!connection.value.connected) {
      return;
    }

    await perform("refreshWifiScan", () => wifiScanResults({ offset, count }), {
      background,
      assign: (value) => {
        wifiScanState.value = value;
      },
    });
  }

  async function refreshHistory(offset = 0, count = HISTORY_PAGE_SIZE, background = false): Promise<void> {
    if (!connection.value.connected) {
      return;
    }

    await perform("refreshHistory", () => historyAlbums({ offset, count }), {
      background,
      assign: (value) => {
        historyAlbumsState.value = value;
      },
    });
  }

  async function refreshDashboard(background = false): Promise<void> {
    await runSequentially([
      () => refreshSnapshot(background),
      () => refreshLibraryAlbum(background),
      () => refreshLibraryTracks(0, LIBRARY_TRACK_PAGE_SIZE, background),
    ]);
  }

  async function refreshLibrary(background = false): Promise<void> {
    await runSequentially([
      () => refreshLibraryAlbum(background),
      () => refreshLibraryTracks(0, LIBRARY_TRACK_PAGE_SIZE, background),
      () => refreshHistory(0, HISTORY_PAGE_SIZE, background),
    ]);
  }

  async function refreshConnectivity(background = false): Promise<void> {
    await runSequentially([
      () => refreshSnapshot(background),
      () => refreshWifiScan(0, WIFI_SCAN_PAGE_SIZE, background),
    ]);
  }

  async function refreshSettings(background = false): Promise<void> {
    await runSequentially([
      () => refreshSnapshot(background),
      () => refreshHello(background),
      () => refreshCapabilities(background),
      () => refreshPairing(background),
      () => refreshTrusted(background),
    ]);
  }

  async function refreshAll(background = false): Promise<void> {
    if (!connection.value.connected) {
      resetSessionState();
      return;
    }

    await runSequentially([
      () => refreshSnapshot(background),
      () => refreshHello(background),
      () => refreshCapabilities(background),
      () => refreshLibraryAlbum(background),
      () => refreshLibraryTracks(0, LIBRARY_TRACK_PAGE_SIZE, background),
      () => refreshHistory(0, HISTORY_PAGE_SIZE, background),
      () => refreshPairing(background),
      () => refreshTrusted(background),
    ]);
  }

  function appendFrame(payload: CompanionEventPayload): void {
    frameCounter += 1;
    frameLog.value = [
      {
        id: `frame-${frameCounter}`,
        command: typeof payload.opcode === "string" ? payload.opcode : "companion://frame",
        time: nowTimestamp(),
        payload,
      },
      ...frameLog.value,
    ].slice(0, 32);
  }

  function applyPairingEvent(payload: CompanionEventPayload): boolean {
    if (typeof payload.pairing_pending !== "boolean") {
      return false;
    }

    pairingState.value = {
      pairing_pending: Boolean(payload.pairing_pending),
      pairing_progress: Number(payload.pairing_progress ?? 0),
      pairing_required: Number(payload.pairing_required ?? 0),
      pending_client_id: String(payload.pending_client_id ?? ""),
      pending_app_name: String(payload.pending_app_name ?? ""),
      button_sequence: Array.isArray(payload.button_sequence)
        ? payload.button_sequence.map((entry) => String(entry))
        : [],
    };
    snapshotState.value = {
      ...snapshotState.value,
      pairing: pairingState.value,
    };
    return true;
  }

  function applySnapshotEvent(payload: CompanionEventPayload): boolean {
    const patch = snapshotPatchFromEvent(payload);
    if (!patch) {
      return false;
    }

    const nextSnapshot = {
      ...snapshotState.value,
      generation: typeof patch.generation === "number" ? patch.generation : snapshotState.value.generation,
      uptime_ms: typeof patch.uptime_ms === "number" ? patch.uptime_ms : snapshotState.value.uptime_ms,
      auth: mergeRecord(snapshotState.value.auth, patch.auth),
      pairing: mergeRecord(snapshotState.value.pairing, patch.pairing),
      playback: mergeRecord(snapshotState.value.playback, patch.playback),
      cartridge: mergeRecord(snapshotState.value.cartridge, patch.cartridge),
      wifi: mergeRecord(snapshotState.value.wifi, patch.wifi),
      lastfm: mergeRecord(snapshotState.value.lastfm, patch.lastfm),
      history: mergeRecord(snapshotState.value.history, patch.history),
      bluetooth: mergeRecord(snapshotState.value.bluetooth, patch.bluetooth),
    };

    const previousChecksum = snapshotState.value.cartridge.checksum;
    snapshotState.value = nextSnapshot;
    pairingState.value = nextSnapshot.pairing;

    if (previousChecksum !== nextSnapshot.cartridge.checksum) {
      void refreshLibrary(true);
    }

    return true;
  }

  function handleFrame(payload: CompanionEventPayload): void {
    appendFrame(payload);
    if (payload.frame_type === "heartbeat") {
      heartbeat.value = payload;
      if (typeof payload.generation === "number" || typeof payload.uptime_ms === "number") {
        snapshotState.value = {
          ...snapshotState.value,
          generation: typeof payload.generation === "number" ? payload.generation : snapshotState.value.generation,
          uptime_ms: typeof payload.uptime_ms === "number" ? payload.uptime_ms : snapshotState.value.uptime_ms,
          auth: {
            ...snapshotState.value.auth,
            authenticated:
              typeof payload.authenticated === "boolean"
                ? payload.authenticated
                : snapshotState.value.auth.authenticated,
          },
        };
      }
      return;
    }

    if (applySnapshotEvent(payload) || applyPairingEvent(payload)) {
      return;
    }

    void refreshSnapshot(true);
  }

  async function initialize(): Promise<void> {
    if (initialized.value) {
      return;
    }

    activity.initialize = true;
    lastError.value = null;

    try {
      await refreshConnection(true);
      frameStopper = await listenToFrames(handleFrame);
      frameListenerState.value = "armed";
      initialized.value = true;
      if (connection.value.connected) {
        await refreshAll(true);
        if (!isAuthenticated.value && !pairingState.value.pairing_pending && !autoPairPaused.value) {
          await ensureBackgroundPairing();
        }
      } else {
        await attemptAutoConnect();
      }
    } catch (error) {
      const issue = companionIssueFor("initialize", error, false);
      frameListenerState.value = issue.detail;
      reportIssue("initialize", error, false);
    } finally {
      activity.initialize = false;
      syncAutomationState();
    }
  }

  async function scanDevices(scanTimeoutSecs = 5, options: ScanDevicesOptions = {}): Promise<DiscoveredDevice[]> {
    const devices = await perform(options.activityKey ?? "scanDevices", () => scan({ scan_timeout_secs: scanTimeoutSecs }), {
      background: options.background,
      assign: (value) => {
        discoveredDevices.value = value;
      },
    });
    return devices ?? [];
  }

  async function connectToDevice(request: ConnectRequest, options: ConnectToDeviceOptions = {}): Promise<boolean> {
    autoConnectPaused.value = false;
    autoPairPaused.value = false;

    const status = await perform(options.activityKey ?? "connectToDevice", () => connect(request), {
      background: options.background,
    });
    if (!status) {
      syncAutomationState();
      return false;
    }

    resetSessionState();
    clearErrors();
    lastError.value = null;
    connection.value = status;
    await refreshAll(options.background);

    if (connection.value.connected && !isAuthenticated.value && !pairingState.value.pairing_pending && !autoPairPaused.value) {
      await ensureBackgroundPairing();
    }

    syncAutomationState();
    return true;
  }

  async function disconnectDevice(): Promise<void> {
    const status = await perform("disconnectDevice", () => disconnect());
    if (!status) {
      return;
    }

    connection.value = status;
    resetSessionState();
    autoConnectPaused.value = true;
    autoPairPaused.value = true;
    syncAutomationState();
  }

  async function runPlaybackAction(action: Parameters<typeof playbackControl>[0]): Promise<void> {
    if (!connection.value.connected) {
      return;
    }

    await perform("playbackAction", () => playbackControl(action));
    await refreshSnapshot(true);
  }

  async function playPause(): Promise<void> {
    snapshotState.value.playback.playing = !snapshotState.value.playback.playing;
    snapshotState.value.playback.paused = !snapshotState.value.playback.paused;
    await runPlaybackAction({ action: "pause" });
  }

  async function previousTrack(): Promise<void> {
    await runPlaybackAction({ action: "prev" });
  }

  async function nextTrack(): Promise<void> {
    await runPlaybackAction({ action: "next" });
  }

  async function setVolume(value: number): Promise<void> {
    const normalizedValue = integerCommandValue(value);
    snapshotState.value.playback.volume_percent = normalizedValue;
    await runPlaybackAction({ action: "volume", value: normalizedValue });
  }

  async function seekTo(value: number): Promise<void> {
    const normalizedValue = integerCommandValue(value);
    snapshotState.value.playback.position_sec = normalizedValue;
    await runPlaybackAction({ action: "seek", value: normalizedValue });
  }

  async function setPlaybackMode(mode: PlaybackMode): Promise<void> {
    snapshotState.value.playback.playback_mode = mode;
    await runPlaybackAction({ action: "mode", mode });
  }

  async function setOutputTarget(output_target: OutputTarget): Promise<void> {
    snapshotState.value.playback.output_target = output_target;
    await runPlaybackAction({ action: "output", output_target });
  }

  async function playTrack(trackIndex: number): Promise<void> {
    const normalizedIndex = integerCommandValue(trackIndex);
    snapshotState.value.playback.track_index = normalizedIndex;
    await runPlaybackAction({ action: "play_index", value: normalizedIndex });
  }

  async function startWifiScan(): Promise<void> {
    if (!connection.value.connected) {
      return;
    }

    const response = await perform("startWifiScan", () => wifiScanStart());
    if (response === null) {
      return;
    }
    await refreshWifiScan(0, WIFI_SCAN_PAGE_SIZE, true);
  }

  async function connectWifiBySsid(ssid: string, password?: string): Promise<void> {
    await perform("connectWifi", () => wifiConnect({ ssid, password }));
    await refreshSnapshot(true);
  }

  async function connectWifiBySlot(slot: number): Promise<void> {
    await perform("connectWifiSlot", () => wifiConnectSlot(slot));
    await refreshSnapshot(true);
  }

  async function disconnectWifiNetwork(): Promise<void> {
    await perform("disconnectWifiNetwork", () => wifiDisconnect());
    await refreshSnapshot(true);
  }

  async function setWifiAutoreconnect(enabled: boolean): Promise<void> {
    snapshotState.value.wifi.autoreconnect = enabled;
    await perform("setWifiAutoreconnect", () => wifiAutoreconnect(enabled));
    await refreshSnapshot(true);
  }

  async function runBluetoothAction(action: "connect-last" | "pair-best" | "disconnect"): Promise<void> {
    await perform("runBluetoothAction", () => bluetoothControl(action));
    await refreshSnapshot(true);
  }

  async function setLastfmAuthUrl(url: string): Promise<void> {
    await perform("setLastfmAuthUrl", () => lastfmControl({ action: "set_auth_url", url }));
    await refreshSnapshot(true);
  }

  async function authenticateLastfm(username: string, password: string): Promise<void> {
    await perform("authenticateLastfm", () => lastfmControl({ action: "auth", username, password }));
    await refreshSnapshot(true);
  }

  async function setLastfmScrobbling(enabled: boolean): Promise<void> {
    snapshotState.value.lastfm.scrobbling = enabled;
    await perform("setLastfmScrobbling", () => lastfmControl({ action: "scrobble", enabled }));
    await refreshSnapshot(true);
  }

  async function logoutLastfm(): Promise<void> {
    await perform("logoutLastfm", () => lastfmControl({ action: "logout" }));
    await refreshSnapshot(true);
  }

  async function beginPairing(request: PairBeginRequest, options: PairingActionOptions = {}): Promise<void> {
    autoPairPaused.value = false;

    const result = await perform<PairBeginResponse>(options.activityKey ?? "beginPairing", () => pairBegin(request), {
      background: options.background,
    });
    if (!result) {
      return;
    }

    const normalizedPairing = pairingStateFromUnknown(result);
    if (normalizedPairing) {
      pairingState.value = normalizedPairing;
    }

    await refreshTrusted(true);
    await refreshSnapshot(true);
    await refreshPairing(true);
  }

  async function cancelPairingRequest(): Promise<void> {
    const result = await perform("cancelPairingRequest", () => pairCancel());
    if (result) {
      const normalizedPairing = pairingStateFromUnknown(result);
      if (normalizedPairing) {
        pairingState.value = normalizedPairing;
      }
      autoPairPaused.value = true;
      await refreshSnapshot(true);
      await refreshPairing(true);
    }
  }

  async function revokeTrustedClient(clientId: string): Promise<void> {
    await perform("revokeTrustedClient", () => trustedRevoke(clientId));
    await refreshTrusted(true);
    await refreshSnapshot(true);
  }

  async function syncAfterDebugCommand(command: string): Promise<void> {
    if (command === "companion_scan") {
      return;
    }

    await refreshConnection(true);
    if (!connection.value.connected) {
      resetSessionState();
      return;
    }

    if (command === "companion_wifi_scan_start" || command === "companion_wifi_scan_results") {
      await refreshWifiScan(0, WIFI_SCAN_PAGE_SIZE, true);
      return;
    }

    await runSequentially([
      () => refreshSnapshot(true),
      () => refreshLibraryAlbum(true),
      () => refreshLibraryTracks(0, LIBRARY_TRACK_PAGE_SIZE, true),
      () => refreshHistory(0, HISTORY_PAGE_SIZE, true),
      () => refreshPairing(true),
      () => refreshTrusted(true),
      () => refreshHello(true),
      () => refreshCapabilities(true),
    ]);
  }

  watch(
    [
      initialized,
      () => connection.value.connected,
      isAuthenticated,
      () => pairingState.value.pairing_pending,
      autoConnectPaused,
      autoPairPaused,
    ],
    () => {
      syncAutomationState();
    },
    { immediate: true },
  );

  return {
    activity,
    activeIssue,
    albumState,
    automationLabel,
    automationMessage,
    automationState,
    autoConnectPaused,
    capabilitiesState,
    clearErrors,
    clearFrameLog,
    connectToDevice,
    connection,
    discoveredDevices,
    disconnectDevice,
    dismissIssue,
    errors,
    frameListenerState,
    frameLog,
    heartbeat,
    historyAlbumsState,
    helloState,
    initialize,
    initialized,
    isAuthenticated,
    isConnected,
    hasMountedCartridge,
    lastError,
    pairingState,
    playPause,
    playTrack,
    previousTrack,
    nextTrack,
    refreshAll,
    refreshConnectivity,
    refreshDashboard,
    refreshHistory,
    refreshLibrary,
    refreshLibraryTracks,
    refreshSettings,
    refreshWifiScan,
    scanDevices,
    setLastfmAuthUrl,
    setLastfmScrobbling,
    setOutputTarget,
    setPlaybackMode,
    setVolume,
    sessionMessage,
    sessionPhase,
    seekTo,
    snapshotState,
    startWifiScan,
    statusLabel,
    stopFrameListener,
    syncAfterDebugCommand,
    tracksState,
    trustedClientsState,
    wifiScanState,
    beginPairing,
    cancelPairingRequest,
    revokeTrustedClient,
    authenticateLastfm,
    logoutLastfm,
    connectWifiBySsid,
    connectWifiBySlot,
    disconnectWifiNetwork,
    setWifiAutoreconnect,
    runBluetoothAction,
    resumeAutoConnect,
  };
});