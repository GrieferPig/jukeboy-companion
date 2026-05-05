export type FieldType = "text" | "number" | "password" | "checkbox" | "select";
export type FieldValue = string | number | boolean;
export type RequestMode = "none" | "optional" | "required";
export type RunState = "idle" | "running" | "success" | "error";

export interface FieldOption {
  label: string;
  value: string;
}

export interface FieldSpec {
  key: string;
  label: string;
  type: FieldType;
  placeholder?: string;
  help?: string;
  options?: FieldOption[];
  defaultValue?: FieldValue;
  min?: number;
  step?: number;
}

export interface CommandSpec {
  name: string;
  title: string;
  description: string;
  requestMode: RequestMode;
  fields: FieldSpec[];
  buildRequest: (values: Record<string, FieldValue>) => Record<string, unknown> | null;
}

export interface CommandSection {
  title: string;
  description: string;
  commands: CommandSpec[];
}

export interface CommandRunInfo {
  state: RunState;
  at: string;
}

export type FormState = Record<string, Record<string, FieldValue>>;
export type CommandRunState = Record<string, CommandRunInfo>;

function textValue(value: FieldValue | undefined): string | undefined {
  if (typeof value !== "string") {
    return undefined;
  }
  const trimmed = value.trim();
  return trimmed === "" ? undefined : trimmed;
}

function numberValue(value: FieldValue | undefined): number | undefined {
  if (typeof value === "number") {
    return Number.isFinite(value) ? value : undefined;
  }
  const text = textValue(value);
  if (text === undefined) {
    return undefined;
  }
  const parsed = Number(text);
  return Number.isFinite(parsed) ? parsed : undefined;
}

function checkboxValue(value: FieldValue | undefined): boolean {
  return typeof value === "boolean" ? value : value === "true";
}

function compactRecord(record: Record<string, unknown>): Record<string, unknown> {
  return Object.fromEntries(Object.entries(record).filter(([, value]) => value !== undefined));
}

function optionalRequest(record: Record<string, unknown>): Record<string, unknown> | null {
  const clean = compactRecord(record);
  return Object.keys(clean).length > 0 ? clean : null;
}

function requiredRequest(record: Record<string, unknown>): Record<string, unknown> {
  return compactRecord(record);
}

export const commandSections: CommandSection[] = [
  {
    title: "Connection",
    description: "Discovery, session control, and low-level health checks.",
    commands: [
      {
        name: "companion_scan",
        title: "Scan",
        description: "Discover BLE candidates that advertise the companion service.",
        requestMode: "none",
        fields: [],
        buildRequest: () => null,
      },
      {
        name: "companion_connect",
        title: "Connect",
        description: "Open a long-lived BLE session using address, name, or auto-discovery.",
        requestMode: "required",
        fields: [
          { key: "address", label: "Address", type: "text", placeholder: "AA:BB:CC:DD:EE:FF" },
          { key: "name", label: "Name", type: "text", placeholder: "ESP_SPP_SERVER" },
        ],
        buildRequest: (values) =>
          requiredRequest({
            address: textValue(values.address),
            name: textValue(values.name),
          }),
      },
      {
        name: "companion_disconnect",
        title: "Disconnect",
        description: "Tear down the active BLE session.",
        requestMode: "none",
        fields: [],
        buildRequest: () => null,
      },
      {
        name: "companion_connection_status",
        title: "Connection Status",
        description: "Read the current session state from the backend manager.",
        requestMode: "none",
        fields: [],
        buildRequest: () => null,
      },
      {
        name: "companion_hello",
        title: "Hello",
        description: "Call the HELLO opcode against the connected device.",
        requestMode: "none",
        fields: [],
        buildRequest: () => null,
      },
      {
        name: "companion_capabilities",
        title: "Capabilities",
        description: "Fetch protocol limits and pairing/auth state.",
        requestMode: "none",
        fields: [],
        buildRequest: () => null,
      },
      {
        name: "companion_ping",
        title: "Ping",
        description: "Round-trip an arbitrary text payload.",
        requestMode: "optional",
        fields: [{ key: "text", label: "Ping text", type: "text", defaultValue: "ping" }],
        buildRequest: (values) =>
          optionalRequest({
            text: textValue(values.text),
          }),
      },
    ],
  },
  {
    title: "Pairing And Trust",
    description: "Pairing workflows, auth proof, and trusted client management.",
    commands: [
      {
        name: "companion_pair_begin",
        title: "Pair Begin",
        description: "Start pairing and optionally wait for auth completion.",
        requestMode: "required",
        fields: [],
        buildRequest: () =>
          requiredRequest({
            wait: true,
            wait_timeout_secs: 120,
          }),
      },
      {
        name: "companion_pair_status",
        title: "Pair Status",
        description: "Read the current pairing state machine.",
        requestMode: "none",
        fields: [],
        buildRequest: () => null,
      },
      {
        name: "companion_pair_cancel",
        title: "Pair Cancel",
        description: "Cancel an active pairing workflow.",
        requestMode: "none",
        fields: [],
        buildRequest: () => null,
      },
      {
        name: "companion_trusted_list",
        title: "Trusted List",
        description: "List currently trusted clients on the device.",
        requestMode: "none",
        fields: [],
        buildRequest: () => null,
      },
    ],
  },
  {
    title: "Playback And Library",
    description: "Snapshot, transport control, and local library browsing.",
    commands: [
      {
        name: "companion_snapshot",
        title: "Snapshot",
        description: "Fetch the merged device state snapshot.",
        requestMode: "none",
        fields: [],
        buildRequest: () => null,
      },
      {
        name: "companion_playback_status",
        title: "Playback Status",
        description: "Read playback-related snapshot fields only.",
        requestMode: "none",
        fields: [],
        buildRequest: () => null,
      },
      {
        name: "companion_playback_control",
        title: "Playback Control",
        description: "Drive transport, seeking, volume, mode, and output target.",
        requestMode: "required",
        fields: [
          {
            key: "action",
            label: "Action",
            type: "select",
            defaultValue: "next",
            options: [
              { label: "Next", value: "next" },
              { label: "Previous", value: "prev" },
              { label: "Pause toggle", value: "pause" },
              { label: "Fast forward", value: "ff" },
              { label: "Rewind", value: "rewind" },
              { label: "Play index", value: "play_index" },
              { label: "Seek seconds", value: "seek" },
              { label: "Set volume", value: "volume" },
              { label: "Set mode", value: "mode" },
              { label: "Set output", value: "output" },
            ],
          },
          { key: "value", label: "Numeric value", type: "number", placeholder: "track, seconds, or percent" },
          {
            key: "mode",
            label: "Playback mode",
            type: "select",
            defaultValue: "sequential",
            options: [
              { label: "Sequential", value: "sequential" },
              { label: "Single repeat", value: "single_repeat" },
              { label: "Shuffle", value: "shuffle" },
            ],
          },
          {
            key: "output_target",
            label: "Output target",
            type: "select",
            defaultValue: "bluetooth",
            options: [
              { label: "Bluetooth", value: "bluetooth" },
              { label: "I2S", value: "i2s" },
            ],
          },
        ],
        buildRequest: (values) => {
          const action = textValue(values.action) ?? "next";
          const request: Record<string, unknown> = { action };
          if (["play_index", "seek", "volume"].includes(action)) {
            request.value = numberValue(values.value);
          }
          if (action === "mode") {
            request.mode = textValue(values.mode);
          }
          if (action === "output") {
            request.output_target = textValue(values.output_target);
          }
          return requiredRequest(request);
        },
      },
      {
        name: "companion_library_album",
        title: "Library Album",
        description: "Read the mounted album metadata.",
        requestMode: "none",
        fields: [],
        buildRequest: () => null,
      },
      {
        name: "companion_library_tracks",
        title: "Library Tracks",
        description: "Page through cartridge tracks.",
        requestMode: "optional",
        fields: [
          { key: "offset", label: "Offset", type: "number", defaultValue: "0", min: 0, step: 1 },
          { key: "count", label: "Count", type: "number", defaultValue: "8", min: 1, step: 1 },
        ],
        buildRequest: (values) =>
          optionalRequest({
            offset: numberValue(values.offset),
            count: numberValue(values.count),
          }),
      },
    ],
  },
  {
    title: "Wi-Fi",
    description: "Status, scan results, connect flows, and auto-reconnect toggling.",
    commands: [
      {
        name: "companion_wifi_status",
        title: "Wi-Fi Status",
        description: "Read Wi-Fi and internet state.",
        requestMode: "none",
        fields: [],
        buildRequest: () => null,
      },
      {
        name: "companion_wifi_scan_start",
        title: "Wi-Fi Scan Start",
        description: "Trigger an access point scan on the device.",
        requestMode: "none",
        fields: [],
        buildRequest: () => null,
      },
      {
        name: "companion_wifi_scan_results",
        title: "Wi-Fi Scan Results",
        description: "Page through returned Wi-Fi scan results.",
        requestMode: "optional",
        fields: [
          { key: "offset", label: "Offset", type: "number", defaultValue: "0", min: 0, step: 1 },
          { key: "count", label: "Count", type: "number", defaultValue: "8", min: 1, step: 1 },
        ],
        buildRequest: (values) =>
          optionalRequest({
            offset: numberValue(values.offset),
            count: numberValue(values.count),
          }),
      },
      {
        name: "companion_wifi_connect",
        title: "Wi-Fi Connect",
        description: "Connect using an SSID and optional password.",
        requestMode: "required",
        fields: [
          { key: "ssid", label: "SSID", type: "text", placeholder: "Network name" },
          { key: "password", label: "Password", type: "password" },
        ],
        buildRequest: (values) =>
          requiredRequest({
            ssid: textValue(values.ssid),
            password: textValue(values.password),
          }),
      },
      {
        name: "companion_wifi_connect_slot",
        title: "Wi-Fi Connect Slot",
        description: "Connect using a stored slot index.",
        requestMode: "required",
        fields: [{ key: "slot", label: "Slot", type: "number", defaultValue: "0", min: 0, step: 1 }],
        buildRequest: (values) =>
          requiredRequest({
            slot: numberValue(values.slot),
          }),
      },
      {
        name: "companion_wifi_disconnect",
        title: "Wi-Fi Disconnect",
        description: "Disconnect the device from Wi-Fi.",
        requestMode: "none",
        fields: [],
        buildRequest: () => null,
      },
      {
        name: "companion_wifi_autoreconnect",
        title: "Wi-Fi Autoreconnect",
        description: "Toggle Wi-Fi auto-reconnect.",
        requestMode: "required",
        fields: [{ key: "enabled", label: "Enabled", type: "checkbox", defaultValue: true }],
        buildRequest: (values) =>
          requiredRequest({
            enabled: checkboxValue(values.enabled),
          }),
      },
    ],
  },
  {
    title: "Last.fm, History, And Bluetooth",
    description: "Cloud auth, play history, and Bluetooth audio control.",
    commands: [
      {
        name: "companion_lastfm_status",
        title: "Last.fm Status",
        description: "Read Last.fm integration state.",
        requestMode: "none",
        fields: [],
        buildRequest: () => null,
      },
      {
        name: "companion_lastfm_control",
        title: "Last.fm Control",
        description: "Drive auth URL, token request, auth, logout, and scrobble flags.",
        requestMode: "required",
        fields: [
          {
            key: "action",
            label: "Action",
            type: "select",
            defaultValue: "token",
            options: [
              { label: "Set auth URL", value: "set_auth_url" },
              { label: "Request token", value: "token" },
              { label: "Auth", value: "auth" },
              { label: "Logout", value: "logout" },
              { label: "Set scrobbling", value: "scrobble" },
              { label: "Set now playing", value: "now_playing" },
            ],
          },
          { key: "url", label: "Auth URL", type: "text", placeholder: "https://..." },
          { key: "username", label: "Username", type: "text" },
          { key: "password", label: "Password", type: "password" },
          { key: "enabled", label: "Enabled", type: "checkbox", defaultValue: true },
        ],
        buildRequest: (values) => {
          const action = textValue(values.action) ?? "token";
          const request: Record<string, unknown> = { action };
          if (action === "set_auth_url") {
            request.url = textValue(values.url);
          }
          if (action === "auth") {
            request.username = textValue(values.username);
            request.password = textValue(values.password);
          }
          if (action === "scrobble" || action === "now_playing") {
            request.enabled = checkboxValue(values.enabled);
          }
          return requiredRequest(request);
        },
      },
      {
        name: "companion_history_summary",
        title: "History Summary",
        description: "Read summary playback history metrics.",
        requestMode: "none",
        fields: [],
        buildRequest: () => null,
      },
      {
        name: "companion_history_albums",
        title: "History Albums",
        description: "Page through historical albums.",
        requestMode: "optional",
        fields: [
          { key: "offset", label: "Offset", type: "number", defaultValue: "0", min: 0, step: 1 },
          { key: "count", label: "Count", type: "number", defaultValue: "4", min: 1, step: 1 },
        ],
        buildRequest: (values) =>
          optionalRequest({
            offset: numberValue(values.offset),
            count: numberValue(values.count),
          }),
      },
      {
        name: "companion_bt_status",
        title: "Bluetooth Status",
        description: "Read Bluetooth audio state.",
        requestMode: "none",
        fields: [],
        buildRequest: () => null,
      },
      {
        name: "companion_bt_control",
        title: "Bluetooth Control",
        description: "Connect last device, pair best candidate, or disconnect.",
        requestMode: "required",
        fields: [
          {
            key: "action",
            label: "Action",
            type: "select",
            defaultValue: "connect-last",
            options: [
              { label: "Connect last", value: "connect-last" },
              { label: "Pair best", value: "pair-best" },
              { label: "Disconnect", value: "disconnect" },
            ],
          },
        ],
        buildRequest: (values) =>
          requiredRequest({
            action: textValue(values.action),
          }),
      },
    ],
  },
];

const allCommands = commandSections.flatMap((section) => section.commands);

function defaultFieldValue(field: FieldSpec): FieldValue {
  if (field.defaultValue !== undefined) {
    return field.defaultValue;
  }
  return field.type === "checkbox" ? false : "";
}

export function buildInitialFormState(): FormState {
  return Object.fromEntries(
    allCommands.map((command) => [
      command.name,
      Object.fromEntries(command.fields.map((field) => [field.key, defaultFieldValue(field)])),
    ]),
  );
}

export function buildInitialRunState(): CommandRunState {
  return Object.fromEntries(allCommands.map((command) => [command.name, { state: "idle", at: "" }]));
}

export function isFieldVisible(command: CommandSpec, field: FieldSpec, values: Record<string, FieldValue>): boolean {
  if (command.name === "companion_playback_control") {
    const action = textValue(values.action) ?? "next";
    if (field.key === "value") {
      return ["play_index", "seek", "volume"].includes(action);
    }
    if (field.key === "mode") {
      return action === "mode";
    }
    if (field.key === "output_target") {
      return action === "output";
    }
  }

  if (command.name === "companion_lastfm_control") {
    const action = textValue(values.action) ?? "token";
    if (field.key === "url") {
      return action === "set_auth_url";
    }
    if (field.key === "username" || field.key === "password") {
      return action === "auth";
    }
    if (field.key === "enabled") {
      return action === "scrobble" || action === "now_playing";
    }
  }

  return true;
}