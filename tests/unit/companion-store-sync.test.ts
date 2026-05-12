import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { nextTick } from "vue";

import {
  type CommandRequest,
  type CompanionEventPayload,
  type CompanionTransport,
  type PlaybackControlRequest,
  setCompanionTransport,
} from "../../src/services/companion";
import { createBrowserMockTransport } from "../../src/services/mockCompanion";
import { useCompanionStore } from "../../src/stores/companion";

function flushMicrotasks(): Promise<void> {
  return new Promise((resolve) => queueMicrotask(resolve));
}

async function eventually(assertion: () => void, attempts = 20): Promise<void> {
  let lastError: unknown;

  for (let attempt = 0; attempt < attempts; attempt += 1) {
    try {
      assertion();
      return;
    } catch (error) {
      lastError = error;
      await flushMicrotasks();
    }
  }

  throw lastError;
}

describe("companion store mock state sync", () => {
  let store: ReturnType<typeof useCompanionStore>;
  const playbackRequests: PlaybackControlRequest[] = [];

  function createObservedTransport(transport: CompanionTransport): CompanionTransport {
    return {
      invoke<T>(command: string, request?: CommandRequest): Promise<T> {
        if (command === "companion_playback_control" && request) {
          playbackRequests.push({ ...(request as PlaybackControlRequest) });
        }

        return transport.invoke<T>(command, request);
      },
      listen(handler: (payload: CompanionEventPayload) => void) {
        return transport.listen(handler);
      },
    };
  }

  beforeEach(async () => {
    setActivePinia(createPinia());
    playbackRequests.length = 0;

    setCompanionTransport(createObservedTransport(createBrowserMockTransport()));

    store = useCompanionStore();
    await store.initialize();
    await nextTick();
  });

  afterEach(() => {
    store.stopFrameListener();
    setCompanionTransport(null);
    vi.useRealTimers();
  });

  it("applies playback, wifi, lastfm, bluetooth, and pairing changes without a timer refresh", async () => {
    expect(store.connection.connected).toBe(true);
    expect(store.connection.device?.name).toBe("MOCK_JUKEBOY");
    expect(store.albumState.album.artwork_data_url?.startsWith("data:image/")).toBe(true);

    const generationBefore = store.snapshotState.generation ?? 0;

    await store.nextTrack();
    await flushMicrotasks();
    expect(store.snapshotState.playback.track_title).toBe("Immediate Event");
    expect(store.frameLog[0].payload.frame_type).toBe("event");

    await store.disconnectWifiNetwork();
    await flushMicrotasks();
    expect(store.snapshotState.wifi.state).toBe("disconnected");
    expect(store.snapshotState.wifi.internet).toBe(false);

    await store.setLastfmScrobbling(false);
    await flushMicrotasks();
    expect(store.snapshotState.lastfm.scrobbling).toBe(false);

    await store.runBluetoothAction("connect-last");
    await flushMicrotasks();
    expect(store.snapshotState.bluetooth.a2dp_connected).toBe(true);
    expect(store.snapshotState.playback.output_target).toBe("bluetooth");

    await store.beginPairing({ wait: true, wait_timeout_secs: 120 });
    await flushMicrotasks();
    expect(store.pairingState.pairing_pending).toBe(true);
    expect(store.snapshotState.pairing.pairing_pending).toBe(true);
    expect((store.snapshotState.generation ?? 0) > generationBefore).toBe(true);
  });

  it("rounds seek and volume values before sending playback control commands", async () => {
    await store.seekTo(93.78152647660264);
    await flushMicrotasks();

    await store.setVolume(61.6);
    await flushMicrotasks();

    expect(playbackRequests).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ action: "seek", value: 94 }),
        expect.objectContaining({ action: "volume", value: 62 }),
      ]),
    );
    expect(store.snapshotState.playback.position_sec).toBe(94);
    expect(store.snapshotState.playback.volume_percent).toBe(62);
  });

  it("loads the full track catalog page by page once and only clears it when the cartridge is removed", async () => {
    store.stopFrameListener();

    const catalog = Array.from({ length: 12 }, (_, index) => ({
      track_index: index,
      title: `Paged Track ${index + 1}`,
      artist: "Paged Artist",
      duration_sec: 180 + index,
      file_num: index + 1,
    }));
    let cartridgeMounted = true;
    let libraryTrackCalls = 0;

    setActivePinia(createPinia());
    setCompanionTransport({
      async invoke<T>(command: string, request?: CommandRequest): Promise<T> {
        if (command === "companion_connection_status") {
          return {
            connected: true,
            device: { address: "MO:CK:BE:EF:00:01", name: "MOCK_JUKEBOY", profile: "mock" },
          } as T;
        }

        if (command === "companion_hello") {
          return {
            opcode: "hello",
            request_id: null,
            authenticated: true,
            client_id: "mock-client",
            trusted_client_count: 1,
            app_name: "jukeboy-companion-mock",
            protocol_version: 1,
          } as T;
        }

        if (command === "companion_capabilities") {
          return {
            opcode: "capabilities",
            request_id: null,
            authenticated: true,
            client_id: "mock-client",
            trusted_client_count: 1,
            app_name: "jukeboy-companion-mock",
            protocol_version: 1,
            max_frame: 2048,
            mtu: 512,
            max_payload: 2036,
            feature_bits: 65535,
            pairing: {
              pairing_pending: false,
              pairing_progress: 0,
              pairing_required: 4,
              pending_client_id: "",
              pending_app_name: "",
              button_sequence: [],
            },
          } as T;
        }

        if (command === "companion_snapshot") {
          return {
            opcode: "snapshot",
            request_id: null,
            generation: 1,
            uptime_ms: 100,
            auth: { authenticated: true, client_id: "mock-client", trusted_client_count: 1 },
            pairing: {
              pairing_pending: false,
              pairing_progress: 0,
              pairing_required: 4,
              pending_client_id: "",
              pending_app_name: "",
              button_sequence: [],
            },
            playback: {
              playing: true,
              paused: false,
              cartridge_checksum: cartridgeMounted ? 0x4a554b45 : null,
              track_index: 0,
              track_count: cartridgeMounted ? catalog.length : 0,
              position_sec: 10,
              started_at: 1777744000,
              duration_sec: cartridgeMounted ? catalog[0]?.duration_sec ?? null : null,
              volume_percent: 62,
              playback_mode: "sequential",
              track_title: cartridgeMounted ? catalog[0]?.title ?? "" : "",
              track_artist: cartridgeMounted ? catalog[0]?.artist ?? "" : "",
              track_file: cartridgeMounted ? "001.jbt" : "",
              output_target: "i2s",
            },
            cartridge: {
              status: cartridgeMounted ? "ready" : null,
              mounted: cartridgeMounted,
              checksum: cartridgeMounted ? 0x4a554b45 : null,
              metadata_version: cartridgeMounted ? 1 : null,
              track_count: cartridgeMounted ? catalog.length : 0,
            },
            wifi: {
              state: "connected",
              internet: true,
              autoreconnect: true,
              active_slot: 0,
              preferred_slot: 0,
              ip: "192.168.4.42",
            },
            lastfm: {
              has_auth_url: true,
              has_token: true,
              has_session: true,
              busy: false,
              scrobbling: true,
              now_playing: true,
              pending_commands: 0,
              pending_scrobbles: 1,
              successful: 12,
              failed: 0,
              auth_url: "https://ws.audioscrobbler.com/2.0",
              username: "mock-listener",
            },
            history: { album_count: 1, track_count: cartridgeMounted ? catalog.length : 0 },
            bluetooth: { a2dp_connected: false, bonded_count: 0 },
          } as T;
        }

        if (command === "companion_library_album") {
          return {
            opcode: "library_album",
            request_id: null,
            cartridge: {
              status: cartridgeMounted ? "ready" : null,
              mounted: cartridgeMounted,
              checksum: cartridgeMounted ? 0x4a554b45 : null,
              metadata_version: cartridgeMounted ? 1 : null,
              track_count: cartridgeMounted ? catalog.length : 0,
            },
            album: {
              name: "Paged Album",
              artist: "Paged Artist",
              description: "Paged track catalog",
              year: 2026,
              duration_sec: cartridgeMounted ? 2400 : null,
              genre: "Diagnostics",
              artwork_data_url: null,
            },
          } as T;
        }

        if (command === "companion_library_tracks") {
          libraryTrackCalls += 1;
          const record = request as Record<string, number> | undefined;
          const offset = record?.offset ?? 0;
          const count = record?.count ?? 8;
          const tracks = cartridgeMounted ? catalog.slice(offset, offset + count) : [];
          return {
            opcode: "library_track_page",
            request_id: null,
            offset,
            track_count: cartridgeMounted ? catalog.length : 0,
            returned_count: tracks.length,
            tracks,
          } as T;
        }

        if (command === "companion_history_albums") {
          return {
            opcode: "history_album_page",
            request_id: null,
            offset: 0,
            album_count: 1,
            returned_count: 1,
            albums: [],
          } as T;
        }

        if (command === "companion_pair_status") {
          return {
            opcode: "pair_status",
            request_id: null,
            pairing_pending: false,
            pairing_progress: 0,
            pairing_required: 4,
            pending_client_id: "",
            pending_app_name: "",
            button_sequence: [],
          } as T;
        }

        if (command === "companion_trusted_list") {
          return {
            opcode: "trusted_list",
            request_id: null,
            trusted_count: 1,
            clients: [{ client_id: "mock-client", app_name: "jukeboy-companion", created_at: 1777744000 }],
          } as T;
        }

        throw new Error(`unexpected command: ${command}`);
      },
      async listen() {
        return () => { };
      },
    });

    store = useCompanionStore();
    await store.initialize();
    await nextTick();

    expect(store.hasMountedCartridge).toBe(true);
    expect(store.tracksState.tracks).toHaveLength(12);
    expect(libraryTrackCalls).toBe(2);

    await store.refreshDashboard();
    await nextTick();

    expect(store.tracksState.tracks).toHaveLength(12);
    expect(libraryTrackCalls).toBe(2);

    cartridgeMounted = false;
    await store.refreshDashboard();
    await nextTick();

    expect(store.hasMountedCartridge).toBe(false);
    expect(store.tracksState.tracks).toHaveLength(0);
  });

  it("starts pairing automatically when a connected device is not authenticated", async () => {
    store.stopFrameListener();

    let pairBeginCalls = 0;
    const baseTransport = createBrowserMockTransport();
    setActivePinia(createPinia());
    setCompanionTransport({
      async invoke<T>(command: string, request?: CommandRequest): Promise<T> {
        if (command === "companion_pair_begin") {
          pairBeginCalls += 1;
          return {
            pairing_pending: true,
            pairing_progress: 0,
            pairing_required: 4,
            pending_client_id: "mock-client",
            pending_app_name: "jukeboy-companion",
            button_sequence: ["main1", "main2", "misc1", "misc3"],
          } as T;
        }

        if (command === "companion_pair_status") {
          return {
            pairing_pending: pairBeginCalls > 0,
            pairing_progress: 0,
            pairing_required: 4,
            pending_client_id: pairBeginCalls > 0 ? "mock-client" : "",
            pending_app_name: pairBeginCalls > 0 ? "jukeboy-companion" : "",
            button_sequence: pairBeginCalls > 0 ? ["main1", "main2", "misc1", "misc3"] : [],
          } as T;
        }

        const response = await baseTransport.invoke<any>(command, request);
        if (command === "companion_hello") {
          return {
            ...response,
            authenticated: false,
            client_id: "",
            trusted_client_count: 0,
          } as T;
        }

        if (command === "companion_capabilities") {
          return {
            ...response,
            authenticated: false,
            client_id: "",
            trusted_client_count: 0,
            pairing: {
              pairing_pending: false,
              pairing_progress: 0,
              pairing_required: 4,
              pending_client_id: "",
              pending_app_name: "",
              button_sequence: [],
            },
          } as T;
        }

        if (command === "companion_snapshot") {
          return {
            ...response,
            auth: {
              ...response.auth,
              authenticated: false,
              client_id: "",
              trusted_client_count: 0,
            },
            pairing: {
              ...response.pairing,
              pairing_pending: false,
              pairing_progress: 0,
              pairing_required: 4,
              pending_client_id: "",
              pending_app_name: "",
              button_sequence: [],
            },
          } as T;
        }

        return response as T;
      },
      listen(handler: (payload: CompanionEventPayload) => void) {
        return baseTransport.listen((payload) => {
          if (payload.opcode === "snapshot") {
            handler({
              ...payload,
              auth: {
                ...(payload.auth as Record<string, unknown> | undefined),
                authenticated: false,
                client_id: "",
                trusted_client_count: 0,
              },
              pairing: {
                ...(payload.pairing as Record<string, unknown> | undefined),
                pairing_pending: false,
                pairing_progress: 0,
                pairing_required: 4,
                pending_client_id: "",
                pending_app_name: "",
                button_sequence: [],
              },
            });
            return;
          }

          handler(payload);
        });
      },
    });

    store = useCompanionStore();
    await store.initialize();
    await nextTick();
    await eventually(() => {
      expect(store.connection.connected).toBe(true);
      expect(pairBeginCalls).toBe(1);
      expect(store.pairingState.pairing_pending).toBe(true);
    });

    expect(store.connection.connected).toBe(true);
  });

  it("refreshes snapshots automatically on a background timer", async () => {
    store.stopFrameListener();

    vi.useFakeTimers();

    let snapshotCalls = 0;
    const baseTransport = createBrowserMockTransport();
    setActivePinia(createPinia());
    setCompanionTransport({
      invoke<T>(command: string, request?: CommandRequest): Promise<T> {
        if (command === "companion_snapshot") {
          snapshotCalls += 1;
        }
        return baseTransport.invoke<T>(command, request);
      },
      listen(handler: (payload: CompanionEventPayload) => void) {
        return baseTransport.listen(handler);
      },
    });

    store = useCompanionStore();
    await store.initialize();
    await nextTick();

    const baseline = snapshotCalls;
    await vi.advanceTimersByTimeAsync(3_200);

    expect(store.connection.connected).toBe(true);
    expect(snapshotCalls).toBeGreaterThan(baseline);
  });

  it("surfaces recovery notices when auto-connect resumes and reconnects", async () => {
    await store.disconnectDevice();
    await flushMicrotasks();

    expect(store.connection.connected).toBe(false);
    expect(store.autoConnectPaused).toBe(true);
    expect(store.notifications).toHaveLength(0);

    store.resumeAutoConnect();

    await eventually(() => {
      expect(store.connection.connected).toBe(true);
      expect(store.notifications.some((entry) => entry.title === "Auto-connect resumed")).toBe(true);
      expect(store.notifications.some((entry) => entry.title === "Companion connected")).toBe(true);
    });
  });

  it("normalizes background discovery errors into a clear user-facing cause", async () => {
    store.stopFrameListener();

    setActivePinia(createPinia());
    setCompanionTransport({
      async invoke<T>(command: string): Promise<T> {
        if (command === "companion_connection_status") {
          return { connected: false, device: null } as T;
        }

        if (command === "companion_scan") {
          throw new Error("no Bluetooth adapter is available");
        }

        throw new Error(`unexpected command: ${command}`);
      },
      async listen() {
        return () => { };
      },
    });

    store = useCompanionStore();
    await store.initialize();
    await nextTick();
    await flushMicrotasks();

    expect(store.activeIssue?.title).toBe("Bluetooth unavailable");
    expect(store.activeIssue?.detail).toContain("No Bluetooth adapter");
    expect(store.activeIssue?.recovery).toContain("Enable Bluetooth");
  });

  it("drops the frontend session and resumes auto-reconnect when the BLE link closes", async () => {
    store.stopFrameListener();

    const baseTransport = createBrowserMockTransport();
    let disconnected = false;
    let frameHandler: ((payload: CompanionEventPayload) => void) | null = null;

    setActivePinia(createPinia());
    setCompanionTransport({
      async invoke<T>(command: string, request?: CommandRequest): Promise<T> {
        if (command === "companion_connection_status" && disconnected) {
          return { connected: false, device: null } as T;
        }

        if (command === "companion_scan" && disconnected) {
          return [] as T;
        }

        if (command === "companion_connect" && disconnected) {
          throw new Error("No Jukeboy companion BLE device found");
        }

        if (
          disconnected
          && (command === "companion_playback_control" || command === "companion_snapshot")
        ) {
          throw new Error("BLE device is not connected");
        }

        return baseTransport.invoke<T>(command, request);
      },
      listen(handler: (payload: CompanionEventPayload) => void) {
        frameHandler = handler;
        return baseTransport.listen(handler);
      },
    });

    store = useCompanionStore();
    await store.initialize();
    await nextTick();

    disconnected = true;
    frameHandler?.({
      opcode: "connection_status",
      frame_type: "event",
      event: "link_disconnected",
      connected: false,
      device: null,
      connection: {
        connected: false,
        device: null,
      },
    });

    await eventually(() => {
      expect(store.connection.connected).toBe(false);
      expect(["scanning", "connecting"]).toContain(store.automationState);
    });

    expect(store.connection.connected).toBe(false);
    expect(store.autoConnectPaused).toBe(false);
    expect(store.snapshotState.cartridge.mounted).toBe(false);
    expect(["scanning", "connecting"]).toContain(store.automationState);
  });
});