<script setup lang="ts">
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { onBeforeUnmount, onMounted, reactive, ref } from "vue";

type FieldType = "text" | "number" | "password" | "checkbox" | "select";
type FieldValue = string | number | boolean;
type RequestMode = "none" | "optional" | "required";
type RunState = "idle" | "running" | "success" | "error";

interface FieldOption {
  label: string;
  value: string;
}

interface FieldSpec {
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

interface CommandSpec {
  name: string;
  title: string;
  description: string;
  requestMode: RequestMode;
  fields: FieldSpec[];
  buildRequest: (values: Record<string, FieldValue>) => Record<string, unknown> | null;
}

interface CommandSection {
  title: string;
  description: string;
  commands: CommandSpec[];
}

interface CommandRunInfo {
  state: RunState;
  at: string;
}

interface ActivityEntry {
  id: string;
  kind: "command" | "event";
  command: string;
  status: "success" | "error" | "info";
  time: string;
  payload: unknown;
}

type FormState = Record<string, Record<string, FieldValue>>;
type CommandRunState = Record<string, CommandRunInfo>;

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

function commaListValue(value: FieldValue | undefined): string[] | undefined {
  const text = textValue(value);
  if (text === undefined) {
    return undefined;
  }
  const items = text
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
  return items.length > 0 ? items : undefined;
}

function compactRecord(record: Record<string, unknown>): Record<string, unknown> {
  return Object.fromEntries(
    Object.entries(record).filter(([, value]) => value !== undefined),
  );
}

function optionalRequest(record: Record<string, unknown>): Record<string, unknown> | null {
  const clean = compactRecord(record);
  return Object.keys(clean).length > 0 ? clean : null;
}

function requiredRequest(record: Record<string, unknown>): Record<string, unknown> {
  return compactRecord(record);
}

const commandSections: CommandSection[] = [
  {
    title: "Connection",
    description: "Discovery, session control, and low-level health checks.",
    commands: [
      {
        name: "companion_scan",
        title: "Scan",
        description: "Discover BLE candidates that advertise the companion service.",
        requestMode: "optional",
        fields: [
          {
            key: "scan_timeout_secs",
            label: "Scan timeout",
            type: "number",
            defaultValue: "5",
            min: 0,
            step: 0.5,
          },
        ],
        buildRequest: (values) =>
          optionalRequest({
            scan_timeout_secs: numberValue(values.scan_timeout_secs),
          }),
      },
      {
        name: "companion_connect",
        title: "Connect",
        description: "Open a long-lived BLE session using address, name, or auto-discovery.",
        requestMode: "required",
        fields: [
          { key: "address", label: "Address", type: "text", placeholder: "AA:BB:CC:DD:EE:FF" },
          { key: "name", label: "Name", type: "text", placeholder: "ESP_SPP_SERVER" },
          { key: "profile", label: "Profile", type: "text", placeholder: "default" },
          { key: "client_id", label: "Client ID override", type: "text" },
          { key: "app_name", label: "App name override", type: "text" },
          { key: "secret_hex", label: "Secret hex override", type: "password" },
          { key: "timeout_secs", label: "Timeout", type: "number", defaultValue: "10", min: 0, step: 0.5 },
          { key: "scan_timeout_secs", label: "Scan timeout", type: "number", defaultValue: "5", min: 0, step: 0.5 },
        ],
        buildRequest: (values) =>
          requiredRequest({
            address: textValue(values.address),
            name: textValue(values.name),
            profile: textValue(values.profile),
            client_id: textValue(values.client_id),
            app_name: textValue(values.app_name),
            secret_hex: textValue(values.secret_hex),
            timeout_secs: numberValue(values.timeout_secs),
            scan_timeout_secs: numberValue(values.scan_timeout_secs),
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
        fields: [
          { key: "text", label: "Ping text", type: "text", defaultValue: "ping" },
        ],
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
        fields: [
          { key: "client_id", label: "Client ID", type: "text" },
          { key: "app_name", label: "App name", type: "text", defaultValue: "jukeboy-companion-smoke" },
          { key: "secret_hex", label: "Secret hex", type: "password" },
          {
            key: "sequence",
            label: "Button sequence",
            type: "text",
            placeholder: "main1, main2, misc1, misc3",
            help: "Comma-separated button names. Leave blank to let the backend generate one.",
          },
          { key: "wait", label: "Wait for completion", type: "checkbox", defaultValue: true },
          { key: "wait_timeout_secs", label: "Wait timeout", type: "number", defaultValue: "120", min: 0, step: 1 },
        ],
        buildRequest: (values) =>
          requiredRequest({
            client_id: textValue(values.client_id),
            app_name: textValue(values.app_name),
            secret_hex: textValue(values.secret_hex),
            sequence: commaListValue(values.sequence),
            wait: checkboxValue(values.wait),
            wait_timeout_secs: numberValue(values.wait_timeout_secs),
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
        name: "companion_auth",
        title: "Auth",
        description: "Authenticate using stored or explicit credentials.",
        requestMode: "optional",
        fields: [
          { key: "client_id", label: "Client ID", type: "text" },
          { key: "app_name", label: "App name", type: "text" },
          { key: "secret_hex", label: "Secret hex", type: "password" },
        ],
        buildRequest: (values) =>
          optionalRequest({
            client_id: textValue(values.client_id),
            app_name: textValue(values.app_name),
            secret_hex: textValue(values.secret_hex),
          }),
      },
      {
        name: "companion_trusted_list",
        title: "Trusted List",
        description: "List currently trusted clients on the device.",
        requestMode: "none",
        fields: [],
        buildRequest: () => null,
      },
      {
        name: "companion_trusted_revoke",
        title: "Trusted Revoke",
        description: "Remove a trusted client by client ID.",
        requestMode: "required",
        fields: [
          { key: "client_id", label: "Client ID", type: "text", placeholder: "uuid or stored id" },
        ],
        buildRequest: (values) =>
          requiredRequest({
            client_id: textValue(values.client_id),
          }),
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
        fields: [
          { key: "slot", label: "Slot", type: "number", defaultValue: "0", min: 0, step: 1 },
        ],
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
        fields: [
          { key: "enabled", label: "Enabled", type: "checkbox", defaultValue: true },
        ],
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

function buildInitialFormState(): FormState {
  return Object.fromEntries(
    allCommands.map((command) => [
      command.name,
      Object.fromEntries(
        command.fields.map((field) => [field.key, defaultFieldValue(field)]),
      ),
    ]),
  );
}

function buildInitialRunState(): CommandRunState {
  return Object.fromEntries(
    allCommands.map((command) => [
      command.name,
      { state: "idle", at: "" },
    ]),
  );
}

const formState = reactive(buildInitialFormState()) as FormState;
const commandRuns = reactive(buildInitialRunState()) as CommandRunState;
const latestCommandResult = ref<ActivityEntry | null>(null);
const eventLog = ref<ActivityEntry[]>([]);
const busyCommand = ref<string | null>(null);
const eventListenerStatus = ref("starting");

let eventStopper: (() => void) | null = null;
let entryCounter = 0;

function nextEntryId(): string {
  entryCounter += 1;
  return `entry-${entryCounter}`;
}

function timestamp(): string {
  return new Date().toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

function formatPayload(payload: unknown): string {
  if (typeof payload === "string") {
    return payload;
  }
  try {
    return JSON.stringify(payload, null, 2) ?? "";
  } catch {
    return String(payload);
  }
}

function isFieldVisible(command: CommandSpec, field: FieldSpec): boolean {
  if (command.name === "companion_playback_control") {
    const action = textValue(formState[command.name].action) ?? "next";
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
    const action = textValue(formState[command.name].action) ?? "token";
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

function clearEvents(): void {
  eventLog.value = [];
}

async function runCommand(command: CommandSpec): Promise<void> {
  const request = command.buildRequest(formState[command.name]);
  busyCommand.value = command.name;
  commandRuns[command.name] = { state: "running", at: timestamp() };

  try {
    const response =
      command.requestMode === "none"
        ? await invoke(command.name)
        : await invoke(command.name, { request });

    latestCommandResult.value = {
      id: nextEntryId(),
      kind: "command",
      command: command.name,
      status: "success",
      time: timestamp(),
      payload: {
        request,
        response,
      },
    };
    commandRuns[command.name] = { state: "success", at: timestamp() };
  } catch (error) {
    latestCommandResult.value = {
      id: nextEntryId(),
      kind: "command",
      command: command.name,
      status: "error",
      time: timestamp(),
      payload: {
        request,
        error: error instanceof Error ? error.message : String(error),
      },
    };
    commandRuns[command.name] = { state: "error", at: timestamp() };
  } finally {
    busyCommand.value = null;
  }
}

onMounted(async () => {
  try {
    eventStopper = await listen("companion://frame", (event) => {
      const entry: ActivityEntry = {
        id: nextEntryId(),
        kind: "event",
        command: "companion://frame",
        status: "info",
        time: timestamp(),
        payload: event.payload,
      };
      eventLog.value = [
        entry,
        ...eventLog.value,
      ].slice(0, 24);
    });
    eventListenerStatus.value = "armed";
  } catch (error) {
    eventListenerStatus.value = error instanceof Error ? error.message : String(error);
  }
});

onBeforeUnmount(() => {
  eventStopper?.();
});
</script>

<template>
  <main class="app-shell">
    <section class="panel hero-panel">
      <div class="hero-copy">
        <p class="eyebrow">Jukeboy Companion</p>
        <h1>Backend Smoke Console</h1>
        <p>
          This page is a thin manual test rig for every Tauri-exposed companion command.
          It is intentionally direct: fill a payload, fire the command, inspect the response,
          and watch unsolicited protocol frames stream in on the side.
        </p>
      </div>

      <div class="hero-stats">
        <div class="stat-card">
          <span>Event stream</span>
          <strong>{{ eventListenerStatus }}</strong>
        </div>
        <div class="stat-card">
          <span>Last command</span>
          <strong>{{ latestCommandResult?.command ?? "none yet" }}</strong>
        </div>
        <div class="stat-card">
          <span>Recent events</span>
          <strong>{{ eventLog.length }}</strong>
        </div>
      </div>
    </section>

    <section class="layout-grid">
      <div class="section-stack">
        <section
          v-for="(section, sectionIndex) in commandSections"
          :key="section.title"
          class="panel section-panel"
        >
          <header class="section-header">
            <div>
              <p class="eyebrow">{{ section.title }}</p>
              <h2>{{ section.description }}</h2>
            </div>
          </header>

          <div class="command-grid">
            <article
              v-for="(command, commandIndex) in section.commands"
              :key="command.name"
              class="command-card"
              :style="{ '--card-index': String(sectionIndex * 8 + commandIndex) }"
            >
              <div class="card-header">
                <div>
                  <h3>{{ command.title }}</h3>
                  <p>{{ command.description }}</p>
                </div>
                <span class="status-pill" :class="`is-${commandRuns[command.name].state}`">
                  {{ commandRuns[command.name].state }}
                </span>
              </div>

              <form class="command-form" @submit.prevent="runCommand(command)">
                <div v-if="command.fields.length > 0" class="field-grid">
                  <div
                    v-for="field in command.fields"
                    v-show="isFieldVisible(command, field)"
                    :key="field.key"
                    class="field"
                    :class="{ 'is-checkbox': field.type === 'checkbox' }"
                  >
                    <template v-if="field.type === 'checkbox'">
                      <label class="checkbox-label" :for="`${command.name}-${field.key}`">
                        <input
                          :id="`${command.name}-${field.key}`"
                          v-model="formState[command.name][field.key]"
                          type="checkbox"
                        />
                        <span>{{ field.label }}</span>
                      </label>
                    </template>

                    <template v-else>
                      <label :for="`${command.name}-${field.key}`">{{ field.label }}</label>

                      <select
                        v-if="field.type === 'select'"
                        :id="`${command.name}-${field.key}`"
                        v-model="formState[command.name][field.key]"
                      >
                        <option
                          v-for="option in field.options"
                          :key="option.value"
                          :value="option.value"
                        >
                          {{ option.label }}
                        </option>
                      </select>

                      <input
                        v-else
                        :id="`${command.name}-${field.key}`"
                        v-model="formState[command.name][field.key]"
                        :type="field.type"
                        :min="field.min"
                        :step="field.step"
                        :placeholder="field.placeholder"
                        autocomplete="off"
                      />
                    </template>

                    <small v-if="field.help">{{ field.help }}</small>
                  </div>
                </div>

                <p v-else class="empty-request">No request payload required.</p>

                <div class="card-footer">
                  <code>{{ command.name }}</code>
                  <button type="submit" :disabled="busyCommand === command.name">
                    {{ busyCommand === command.name ? "Running..." : "Run" }}
                  </button>
                </div>
              </form>

              <p v-if="commandRuns[command.name].at" class="last-run">
                Last run at {{ commandRuns[command.name].at }}
              </p>
            </article>
          </div>
        </section>
      </div>

      <aside class="sidebar-stack">
        <section class="panel inspector-panel latest-output-panel">
          <div class="sidebar-header">
            <div>
              <p class="eyebrow">Latest Output</p>
              <h2>{{ latestCommandResult?.command ?? "No command fired yet" }}</h2>
            </div>
            <span
              v-if="latestCommandResult"
              class="status-pill"
              :class="`is-${latestCommandResult.status === 'success' ? 'success' : 'error'}`"
            >
              {{ latestCommandResult.status }}
            </span>
          </div>

          <p v-if="latestCommandResult" class="result-time">
            {{ latestCommandResult.time }}
          </p>

          <pre class="payload-view">{{ latestCommandResult ? formatPayload(latestCommandResult.payload) : "Run a command card to see its request and response here." }}</pre>
        </section>

        <section class="panel inspector-panel event-log-panel">
          <div class="sidebar-header">
            <div>
              <p class="eyebrow">Event Log</p>
              <h2>companion://frame</h2>
            </div>
            <button type="button" class="ghost-button" @click="clearEvents">Clear</button>
          </div>

          <div class="event-log-body">
            <div v-if="eventLog.length === 0" class="empty-events">
              No unsolicited frames received yet.
            </div>

            <ul v-else class="event-list">
              <li v-for="entry in eventLog" :key="entry.id" class="event-item">
                <div class="event-meta">
                  <strong>{{ entry.command }}</strong>
                  <span>{{ entry.time }}</span>
                </div>
                <pre class="payload-view is-compact">{{ formatPayload(entry.payload) }}</pre>
              </li>
            </ul>
          </div>
        </section>
      </aside>
    </section>
  </main>
</template>

<style scoped>
.app-shell {
  margin: 0 auto;
  max-width: 1680px;
  padding: 2rem;
}

.panel {
  border: 1px solid var(--line);
  border-radius: 28px;
  background: linear-gradient(180deg, rgba(255, 251, 245, 0.95), rgba(249, 242, 230, 0.96));
  box-shadow: 0 18px 60px rgba(73, 47, 26, 0.11);
}

.hero-panel {
  display: grid;
  gap: 1.5rem;
  grid-template-columns: minmax(0, 1.6fr) minmax(320px, 0.9fr);
  padding: 1.6rem;
  position: relative;
  overflow: hidden;
}

.hero-panel::after {
  content: "";
  position: absolute;
  inset: auto -8% -30% auto;
  width: 280px;
  height: 280px;
  border-radius: 999px;
  background: radial-gradient(circle, rgba(200, 92, 45, 0.2), rgba(200, 92, 45, 0));
  pointer-events: none;
}

.hero-copy h1,
.section-header h2,
.sidebar-header h2,
.card-header h3 {
  font-family: "Space Grotesk", "Aptos Display", "Trebuchet MS", sans-serif;
  letter-spacing: -0.03em;
}

.hero-copy h1 {
  font-size: clamp(2.2rem, 4vw, 4.3rem);
  line-height: 0.92;
  margin: 0.1rem 0 0.8rem;
}

.hero-copy p,
.card-header p,
.section-header h2,
.result-time,
.empty-request,
.empty-events,
.last-run {
  color: var(--muted);
}

.eyebrow {
  margin: 0;
  color: var(--accent);
  font-size: 0.77rem;
  font-weight: 700;
  letter-spacing: 0.18em;
  text-transform: uppercase;
}

.hero-stats {
  display: grid;
  gap: 0.85rem;
}

.stat-card {
  border-radius: 22px;
  background: rgba(255, 255, 255, 0.6);
  border: 1px solid rgba(78, 44, 16, 0.08);
  padding: 1rem 1.1rem;
}

.stat-card span {
  color: var(--muted);
  display: block;
  font-size: 0.82rem;
  margin-bottom: 0.25rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
}

.stat-card strong {
  font-size: 1rem;
  line-height: 1.35;
}

.layout-grid {
  display: grid;
  gap: 1.2rem;
  grid-template-columns: minmax(0, 1fr) var(--sidebar-width);
  margin-top: 1.2rem;
}

.section-stack,
.sidebar-stack {
  display: grid;
  gap: 1.2rem;
}

.section-panel,
.inspector-panel {
  padding: 1.2rem;
}

.section-header,
.sidebar-header {
  align-items: flex-start;
  display: flex;
  gap: 1rem;
  justify-content: space-between;
  margin-bottom: 1rem;
}

.section-header h2,
.sidebar-header h2 {
  font-size: 1.1rem;
  font-weight: 600;
  margin: 0.2rem 0 0;
}

.command-grid {
  display: grid;
  gap: 1rem;
  grid-template-columns: repeat(auto-fit, minmax(290px, 1fr));
}

.command-card {
  animation: rise 0.55s cubic-bezier(0.2, 0.8, 0.2, 1) both;
  animation-delay: calc(var(--card-index, 0) * 40ms);
  background: rgba(255, 255, 255, 0.72);
  border: 1px solid rgba(78, 44, 16, 0.09);
  border-radius: 24px;
  display: grid;
  gap: 0.95rem;
  padding: 1rem;
}

.card-header {
  align-items: flex-start;
  display: flex;
  gap: 0.8rem;
  justify-content: space-between;
}

.card-header h3 {
  font-size: 1.15rem;
  margin: 0;
}

.card-header p {
  font-size: 0.92rem;
  margin: 0.35rem 0 0;
}

.status-pill {
  border-radius: 999px;
  font-size: 0.74rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  padding: 0.35rem 0.55rem;
  text-transform: uppercase;
  white-space: nowrap;
}

.status-pill.is-idle {
  background: rgba(74, 63, 53, 0.08);
  color: #5b5048;
}

.status-pill.is-running {
  background: rgba(211, 126, 68, 0.17);
  color: #8d4219;
}

.status-pill.is-success {
  background: rgba(58, 132, 93, 0.16);
  color: #20563b;
}

.status-pill.is-error {
  background: rgba(180, 63, 48, 0.16);
  color: #852c22;
}

.command-form {
  display: grid;
  gap: 0.9rem;
}

.field-grid {
  display: grid;
  gap: 0.75rem;
  grid-template-columns: repeat(auto-fit, minmax(125px, 1fr));
}

.field {
  display: grid;
  gap: 0.35rem;
}

.field label {
  font-size: 0.84rem;
  font-weight: 600;
}

.field input,
.field select,
.ghost-button,
.card-footer button {
  border-radius: 14px;
  border: 1px solid rgba(61, 37, 18, 0.14);
  font: inherit;
  transition: transform 120ms ease, border-color 120ms ease, box-shadow 120ms ease;
}

.field input,
.field select {
  background: rgba(255, 254, 251, 0.95);
  color: var(--ink);
  min-height: 2.85rem;
  padding: 0.75rem 0.85rem;
}

.field input:focus,
.field select:focus,
.ghost-button:focus,
.card-footer button:focus {
  border-color: rgba(200, 92, 45, 0.55);
  box-shadow: 0 0 0 4px rgba(200, 92, 45, 0.12);
  outline: none;
}

.field small {
  color: var(--muted);
  font-size: 0.75rem;
  line-height: 1.35;
}

.field.is-checkbox {
  align-items: end;
}

.checkbox-label {
  align-items: center;
  display: flex;
  gap: 0.6rem;
  min-height: 2.85rem;
  padding: 0.2rem 0;
}

.checkbox-label input {
  accent-color: var(--accent);
  margin: 0;
  min-height: auto;
  padding: 0;
}

.empty-request,
.empty-events,
.last-run,
.result-time {
  font-size: 0.84rem;
  margin: 0;
}

.card-footer {
  align-items: center;
  display: flex;
  gap: 0.8rem;
  justify-content: space-between;
}

.card-footer code {
  color: var(--muted);
  font-size: 0.78rem;
  word-break: break-all;
}

.card-footer button,
.ghost-button {
  background: var(--ink);
  color: #fff8f1;
  cursor: pointer;
  padding: 0.78rem 1rem;
}

.card-footer button:hover,
.ghost-button:hover {
  transform: translateY(-1px);
}

.card-footer button:disabled {
  cursor: wait;
  opacity: 0.65;
}

.ghost-button {
  background: rgba(61, 37, 18, 0.08);
  color: var(--ink);
  padding: 0.65rem 0.9rem;
}

.sidebar-stack {
  align-self: start;
  display: flex;
  flex-direction: column;
  gap: 1.2rem;
  height: calc(100vh - 2.4rem);
  max-width: var(--sidebar-width);
  min-height: 0;
  min-width: var(--sidebar-width);
  position: sticky;
  top: 1.2rem;
  width: var(--sidebar-width);
}

.inspector-panel {
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.latest-output-panel {
  flex: 0 0 auto;
}

.event-log-panel {
  flex: 1 1 auto;
  min-height: 0;
}

.event-log-body {
  flex: 1 1 auto;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
  padding-right: 0.2rem;
}

.payload-view {
  background: #241813;
  border-radius: 18px;
  color: #fdf3ea;
  font-family: "IBM Plex Mono", "Cascadia Code", "Consolas", monospace;
  font-size: 0.82rem;
  line-height: 1.5;
  margin: 0;
  max-height: 420px;
  overflow: auto;
  padding: 1rem;
  white-space: pre-wrap;
  word-break: break-word;
}

.payload-view.is-compact {
  max-height: 220px;
}

.event-list {
  display: grid;
  gap: 0.8rem;
  list-style: none;
  margin: 0;
  padding: 0;
}

.event-item {
  background: rgba(255, 255, 255, 0.56);
  border: 1px solid rgba(78, 44, 16, 0.09);
  border-radius: 18px;
  padding: 0.8rem;
}

.event-meta {
  align-items: baseline;
  display: flex;
  gap: 0.7rem;
  justify-content: space-between;
  margin-bottom: 0.55rem;
}

.event-meta strong {
  font-size: 0.86rem;
}

.event-meta span {
  color: var(--muted);
  font-size: 0.76rem;
}

@keyframes rise {
  from {
    opacity: 0;
    transform: translateY(18px);
  }

  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@media (max-width: 1180px) {
  .hero-panel,
  .layout-grid {
    grid-template-columns: 1fr;
  }

  .sidebar-stack {
    height: auto;
    max-width: none;
    min-width: 0;
    position: static;
    width: 100%;
  }

  .event-log-panel {
    min-height: 24rem;
  }

  .event-log-body {
    max-height: 55vh;
  }
}

@media (max-width: 720px) {
  .app-shell {
    padding: 1rem;
  }

  .hero-panel,
  .section-panel,
  .inspector-panel {
    padding: 1rem;
  }

  .card-footer,
  .section-header,
  .sidebar-header,
  .card-header {
    align-items: flex-start;
    flex-direction: column;
  }

  .card-footer button,
  .ghost-button {
    width: 100%;
  }
}
</style>

<style>
:root {
  --ink: #2d1c15;
  --muted: #6e5d55;
  --accent: #c85c2d;
  --line: rgba(79, 48, 25, 0.14);
  --sidebar-width: 26rem;
  color: var(--ink);
  font-family: "IBM Plex Sans", "Aptos", "Segoe UI Variable", sans-serif;
  font-size: 16px;
  font-synthesis: none;
  font-weight: 400;
  line-height: 1.5;
  text-rendering: optimizeLegibility;
  -moz-osx-font-smoothing: grayscale;
  -webkit-font-smoothing: antialiased;
  -webkit-text-size-adjust: 100%;
}

html {
  background:
    radial-gradient(circle at top left, rgba(248, 207, 157, 0.72), transparent 30%),
    radial-gradient(circle at right 12%, rgba(210, 112, 68, 0.18), transparent 24%),
    linear-gradient(180deg, #f7f1e8 0%, #efe3d3 100%);
}

body {
  margin: 0;
  min-height: 100vh;
}

button,
input,
select {
  font: inherit;
}

* {
  box-sizing: border-box;
}
</style>