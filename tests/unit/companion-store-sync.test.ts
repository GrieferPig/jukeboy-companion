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
        return () => {};
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

    setActivePinia(createPinia());
    setCompanionTransport({
      async invoke<T>(command: string, request?: CommandRequest): Promise<T> {
        if (command === "companion_connection_status" && disconnected) {
          return { connected: false, device: null } as T;
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
        return baseTransport.listen(handler);
      },
    });

    store = useCompanionStore();
    await store.initialize();
    await nextTick();

    disconnected = true;
    await store.nextTrack();
    await eventually(() => {
      expect(store.connection.connected).toBe(false);
      expect(store.activeIssue?.title).toBe("Connection lost");
      expect(["scanning", "connecting"]).toContain(store.automationState);
    });

    expect(store.connection.connected).toBe(false);
    expect(store.activeIssue?.title).toBe("Connection lost");
    expect(["scanning", "connecting"]).toContain(store.automationState);
  });
});