
<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { useDisplay } from "vuetify";
import CommandPanel from "../components/debug/CommandPanel.vue";
import { useCompanionStore } from "../stores/companion";
import { titleizeToken } from "../utils/formatting";

const store = useCompanionStore();
const display = useDisplay();

// --- Connectivity ---
const wifiDialog = ref(false);
const wifiPassword = ref("");
const selectedSsid = ref("");

const autoreconnectModel = computed({
  get: () => store.snapshotState.wifi.autoreconnect,
  set: (value: boolean) => {
    void store.setWifiAutoreconnect(value);
  },
});

function openWifiDialog(ssid: string): void {
  selectedSsid.value = ssid;
  wifiPassword.value = "";
  wifiDialog.value = true;
}

async function submitWifi(): Promise<void> {
  await store.connectWifiBySsid(selectedSsid.value, wifiPassword.value || undefined);
  wifiDialog.value = false;
}

// --- Trust ---
const lastfmForm = reactive({
  username: "",
  password: "",
});

const selectedScriptName = ref<string | null>(null);

const scriptStatusLabel = computed(() => titleizeToken(store.scriptStatusState.state || "idle"));
const scriptOutputTitle = computed(() =>
  selectedScriptName.value ? `${selectedScriptName.value} output` : "Latest output",
);

watch(() => store.snapshotState.lastfm.username, (value) => { if (!lastfmForm.username) lastfmForm.username = value; }, { immediate: true });

watch(
  () => store.scriptListState.scripts,
  (scripts) => {
    if (scripts.length === 0) {
      selectedScriptName.value = null;
      return;
    }

    if (selectedScriptName.value && scripts.some((script) => script.name === selectedScriptName.value)) {
      return;
    }

    selectedScriptName.value = scripts[0].name;
    void store.loadScriptLog(scripts[0].name, 0, 2048, true);
  },
  { immediate: true },
);

onMounted(() => {
  void store.refreshSettings();
  void store.refreshConnectivity();
});

async function connectFromDiscovery(address: string, name: string): Promise<void> {
  await store.connectToDevice({ address, name });
}

const preferredWifiSlot = computed(() => store.snapshotState.wifi.preferred_slot ?? store.snapshotState.wifi.active_slot ?? 0);

async function connectSavedWifi(): Promise<void> {
  await store.connectWifiBySlot(preferredWifiSlot.value);
}

async function viewScriptLog(name: string): Promise<void> {
  selectedScriptName.value = name;
  await store.loadScriptLog(name);
}

async function runScript(name: string): Promise<void> {
  selectedScriptName.value = name;
  await store.runDeviceScript(name);
}
</script>

<template>
  <section class="view-shell settings-view">
    <div class="settings-section-lede">
      <p class="eyebrow">Settings</p>
      <h1 class="display-title">Trust And Control</h1>
      <p class="section-copy">
        Connection, account, network, and audio controls live here.
      </p>
    </div>

    <div class="settings-grid">
      <v-card class="screen-sheet" color="surface">
        <div class="settings-header">
          <div>
            <p class="eyebrow">Companion Session</p>
            <h2>Background discovery and session control</h2>
          </div>
          <v-chip color="primary">{{ store.statusLabel }}</v-chip>
        </div>

        <v-alert
          type="info"
          variant="tonal"
          color="surface-variant"
          class="settings-automation-alert companion-alert"
          data-testid="settings-automation-status"
        >
          <template #title>
            {{ store.automationLabel }}
          </template>
          {{ store.automationMessage }}
        </v-alert>

        <div class="settings-actions">
          <v-btn color="primary" :loading="store.activity.scanDevices" data-testid="settings-scan" @click="store.scanDevices()">Scan Now</v-btn>
          <v-btn
            v-if="store.autoConnectPaused"
            variant="outlined"
            color="primary"
            data-testid="settings-resume-auto-connect"
            @click="store.resumeAutoConnect()"
          >
            Resume Auto Connect
          </v-btn>
          <v-btn variant="text" color="primary" data-testid="settings-disconnect" @click="store.disconnectDevice()">Disconnect Companion</v-btn>
        </div>

        <v-list class="device-list">
          <v-list-item v-for="device in store.discoveredDevices" :key="device.address" lines="two" class="device-item">
            <v-list-item-title>{{ device.name || device.address }}</v-list-item-title>
            <v-list-item-subtitle v-if="!device.name">{{ device.address }}</v-list-item-subtitle>
            <template #append>
              <v-btn size="small" color="primary" data-testid="settings-connect-discovered" @click="connectFromDiscovery(device.address, device.name)">
                Connect
              </v-btn>
            </template>
          </v-list-item>
        </v-list>
      </v-card>

      <v-card class="screen-sheet" color="surface">
        <div class="settings-header">
          <div>
            <p class="eyebrow">Last.fm</p>
            <h2>Credentials and scrobbling</h2>
          </div>
          <v-switch
            :model-value="store.snapshotState.lastfm.scrobbling"
            label="Scrobbling"
            data-testid="lastfm-scrobbling"
            @update:model-value="store.setLastfmScrobbling(Boolean($event))"
          />
        </div>

        <div class="settings-subsection">
          <div class="settings-subsection__title">
            <p class="eyebrow">Account</p>
            <h3>Link a Last.fm account</h3>
          </div>

          <div class="settings-form-grid">
            <v-text-field v-model="lastfmForm.username" label="Username" />
            <v-text-field v-model="lastfmForm.password" label="Password" type="password" />
          </div>

          <div class="settings-actions">
            <v-btn variant="outlined" data-testid="lastfm-authenticate" @click="store.authenticateLastfm(lastfmForm.username, lastfmForm.password)">Authenticate</v-btn>
            <v-btn variant="text" color="primary" data-testid="lastfm-logout" @click="store.logoutLastfm()">Logout</v-btn>
          </div>
        </div>
      </v-card>
    </div>

    <v-card v-if="store.pairingState.pairing_pending" class="screen-sheet" color="surface">
      <div class="settings-header">
        <div>
          <p class="eyebrow">Pairing</p>
          <h2>Confirm the button sequence</h2>
        </div>
        <v-chip color="primary" data-testid="pairing-progress">{{ store.pairingState.pairing_progress }} / {{ store.pairingState.pairing_required }}</v-chip>
      </div>

      <v-alert type="info" variant="tonal" color="surface-variant" class="settings-automation-alert companion-alert">
        <template #title>
          Pairing in progress
        </template>
        Press the buttons shown below on the Jukeboy.
      </v-alert>

      <div class="pairing-summary">
        <div>
          <span>Sequence</span>
          <strong data-testid="pairing-sequence">{{ store.pairingState.button_sequence.join(' -> ') }}</strong>
        </div>
      </div>
    </v-card>

    <v-card class="screen-sheet" color="surface" data-testid="settings-scripts-section">
      <div class="settings-header">
        <div>
          <p class="eyebrow">Scripts</p>
          <h2>Safe maintenance tasks</h2>
        </div>
        <v-chip color="primary" data-testid="scripts-status">{{ scriptStatusLabel }}</v-chip>
      </div>

      <p class="section-copy">
        Run named companion scripts without exposing the raw protocol surface. Output is kept here so maintenance tasks stay readable.
      </p>

      <div class="settings-actions">
        <v-btn
          variant="outlined"
          color="primary"
          :loading="store.activity.refreshScriptList || store.activity.refreshScriptStatus"
          data-testid="scripts-refresh"
          @click="store.refreshScripts()"
        >
          Refresh Scripts
        </v-btn>
        <v-btn
          variant="text"
          color="primary"
          :disabled="!selectedScriptName"
          data-testid="scripts-refresh-output"
          @click="selectedScriptName ? store.loadScriptLog(selectedScriptName) : undefined"
        >
          Refresh Output
        </v-btn>
      </div>

      <div v-if="store.scriptListState.scripts.length === 0" class="settings-empty-state">
        No device scripts were reported by the companion.
      </div>

      <div v-else class="scripts-layout">
        <v-list class="scripts-list">
          <v-list-item
            v-for="(script, index) in store.scriptListState.scripts"
            :key="script.name"
            lines="two"
            class="script-item"
            :data-testid="`script-item-${index}`"
          >
            <v-list-item-title>{{ script.name }}</v-list-item-title>
            <v-list-item-subtitle>
              {{ titleizeToken(script.kind ?? "script") }}
              <span v-if="script.last_run"> • {{ script.last_run }}</span>
            </v-list-item-subtitle>

            <template #append>
              <div class="script-actions">
                <v-btn size="small" variant="text" color="primary" :data-testid="`script-view-${index}`" @click.stop="viewScriptLog(script.name)">
                  View Output
                </v-btn>
                <v-btn size="small" color="primary" :loading="store.activity.runDeviceScript" :data-testid="`script-run-${index}`" @click.stop="runScript(script.name)">
                  Run
                </v-btn>
              </div>
            </template>
          </v-list-item>
        </v-list>

        <div class="script-log-panel">
          <div class="settings-subsection__title">
            <p class="eyebrow">Latest Output</p>
            <h3>{{ scriptOutputTitle }}</h3>
          </div>
          <p class="section-copy">
            {{ store.scriptStatusState.message || "Select a script to inspect its latest output." }}
          </p>
          <pre class="script-log" data-testid="script-log-output">{{ store.scriptLogState.output || "No output loaded yet." }}</pre>
        </div>
      </div>
    </v-card>

    <div class="settings-section-lede settings-section-lede--secondary">
      <p class="eyebrow">Connectivity</p>
      <h2 class="section-title">Network And Audio</h2>
      <p class="section-copy">
        Manage Wi-Fi handoff, stored slots, and Bluetooth audio without leaving the monotone control surface.
      </p>
    </div>

    <div class="connectivity-grid">
      <v-card class="screen-sheet" color="surface">
        <div class="connectivity-header">
          <div>
            <p class="eyebrow">Wi-Fi</p>
            <h2>Network state</h2>
          </div>
          <v-btn color="primary" size="large" :loading="store.activity.startWifiScan" data-testid="wifi-scan" @click="store.startWifiScan()">
            Scan Networks
          </v-btn>
        </div>

        <div class="wifi-summary">
          <div>
            <span>State</span>
            <strong data-testid="wifi-state">{{ titleizeToken(store.snapshotState.wifi.state) }}</strong>
          </div>
          <div>
            <span>Internet</span>
            <strong data-testid="wifi-internet">{{ store.snapshotState.wifi.internet ? 'Online' : 'Offline' }}</strong>
          </div>
        </div>

        <v-switch v-model="autoreconnectModel" label="Auto-reconnect" class="wifi-switch" data-testid="wifi-autoreconnect" />

        <div class="slot-row">
          <v-btn variant="outlined" data-testid="wifi-connect" @click="connectSavedWifi">Connect Saved Network</v-btn>
          <v-btn variant="text" color="primary" data-testid="wifi-disconnect" @click="store.disconnectWifiNetwork()">Disconnect Wi-Fi</v-btn>
        </div>

        <v-list class="wifi-list">
          <v-list-item
            v-for="network in store.wifiScanState.results"
            :key="network.ssid"
            lines="one"
            class="wifi-item"
            @click="openWifiDialog(network.ssid)"
          >
            <v-list-item-title>{{ network.ssid }}</v-list-item-title>
            <template #append>
              <v-icon icon="mdi-lock-outline" v-if="network.authmode !== 0" />
            </template>
          </v-list-item>
        </v-list>
      </v-card>

      <v-card class="screen-sheet" color="surface">
        <div class="connectivity-header">
          <div>
            <p class="eyebrow">Bluetooth</p>
            <h2>Audio output</h2>
          </div>
          <v-chip color="primary" data-testid="bt-status">{{ store.snapshotState.bluetooth.a2dp_connected ? 'A2DP connected' : 'Idle' }}</v-chip>
        </div>

        <div class="bt-actions">
          <v-btn color="primary" size="large" data-testid="bt-connect-last" @click="store.runBluetoothAction('connect-last')">Connect Last</v-btn>
          <v-btn variant="outlined" size="large" data-testid="bt-pair-best" @click="store.runBluetoothAction('pair-best')">Pair Best Candidate</v-btn>
          <v-btn variant="text" size="large" color="primary" data-testid="bt-disconnect" @click="store.runBluetoothAction('disconnect')">Disconnect</v-btn>
        </div>
      </v-card>
    </div>

    <v-expansion-panels class="settings-debug-panels" variant="accordion">
      <v-expansion-panel class="screen-sheet" color="surface" data-testid="settings-debug-panel">
        <v-expansion-panel-title>
          <div class="settings-header settings-header--panel">
            <div>
              <p class="eyebrow">Debug &amp; Maintenance</p>
              <h2>Advanced diagnostics</h2>
            </div>
            <v-chip size="small" color="surface-variant">Hidden by default</v-chip>
          </div>
        </v-expansion-panel-title>

        <v-expansion-panel-text>
          <p class="section-copy settings-debug-copy">
            Raw protocol checks, maintenance commands, and low-level event inspection stay behind this panel so the main settings flow remains non-technical.
          </p>
          <CommandPanel />
        </v-expansion-panel-text>
      </v-expansion-panel>
    </v-expansion-panels>

    <v-dialog
      v-model="wifiDialog"
      :fullscreen="display.smAndDown.value"
      :max-width="display.mdAndUp.value ? 520 : undefined"
      scrollable
    >
      <v-card class="screen-sheet" color="surface">
        <p class="eyebrow">Wi-Fi Connect</p>
        <h2 class="dialog-title">{{ selectedSsid }}</h2>
        <p class="section-copy">Enter the password and hand off the device to the selected network.</p>
        <v-text-field v-model="wifiPassword" label="Password" type="password" />
        <div class="dialog-actions">
          <v-btn variant="text" @click="wifiDialog = false">Cancel</v-btn>
          <v-btn color="primary" @click="submitWifi">Connect</v-btn>
        </div>
      </v-card>
    </v-dialog>
  
  </section>
</template>
<style scoped>

.settings-view {
  gap: 1rem;
}

.settings-section-lede {
  display: grid;
  gap: 0.35rem;
  max-width: 64rem;
}

.settings-section-lede .display-title,
.section-title {
  margin: 0;
}

.settings-section-lede--secondary {
  padding-top: 0.25rem;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
}

.section-title {
  font-size: clamp(1.5rem, 3.5vw, 2.4rem);
  font-weight: 800;
  letter-spacing: -0.05em;
}

.settings-grid,
.connectivity-grid {
  display: grid;
  gap: 1rem;
}

.settings-header {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 1rem;
  align-items: flex-end;
}

.settings-header h2 {
  margin: 0;
  font-size: 1.2rem;
}

.settings-actions,
.settings-form-grid,
.pairing-summary {
  margin-top: 0.75rem;
}

.settings-automation-alert {
  margin-top: 0.75rem;
}

.settings-actions {
  display: flex;
  gap: 0.6rem;
  flex-wrap: wrap;
}

.settings-subsection {
  margin-top: 0.75rem;
}

.settings-subsection + .settings-subsection {
  padding-top: 0.75rem;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
}

.settings-subsection__title h3 {
  margin: 0;
  font-size: 1rem;
}

.settings-empty-state {
  margin-top: 0.75rem;
  color: rgba(255, 255, 255, 0.68);
}

.settings-form-grid,
.pairing-summary {
  display: grid;
  gap: 0.65rem;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
}

.pairing-summary div {
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 18px;
  padding: 0.75rem;
  background: rgba(255, 255, 255, 0.03);
}

.pairing-summary span {
  display: block;
  color: rgba(255, 255, 255, 0.56);
  font-size: 0.74rem;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

.device-list {
  margin-top: 0.75rem;
}

.device-item {
  margin-bottom: 0.5rem;
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.scripts-layout {
  display: grid;
  gap: 1rem;
  margin-top: 0.75rem;
}

.scripts-list {
  margin: 0;
  padding: 0;
}

.script-item {
  margin-bottom: 0.5rem;
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.script-actions {
  display: flex;
  gap: 0.35rem;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.script-log-panel {
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 20px;
  padding: 1rem;
  background: rgba(255, 255, 255, 0.03);
}

.script-log {
  margin: 0;
  min-height: 11rem;
  max-height: 20rem;
  overflow: auto;
  padding: 0.95rem;
  border-radius: 18px;
  background: rgba(0, 0, 0, 0.32);
  color: #f0f0f0;
  font-family: "Cascadia Code", "Consolas", monospace;
  font-size: 0.84rem;
  line-height: 1.5;
  white-space: pre-wrap;
}

.hardware-grid {
  margin-top: 1rem;
}


.connectivity-view {
  gap: 1.4rem;
}

.connectivity-header {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  align-items: flex-end;
  gap: 1rem;
}

.connectivity-header h2,
.dialog-title {
  margin: 0;
  font-size: 1.2rem;
}

.wifi-summary {
  display: grid;
  gap: 0.65rem;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  margin-top: 0.75rem;
}

.wifi-summary div {
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 18px;
  padding: 0.75rem;
  background: rgba(255, 255, 255, 0.03);
}

.wifi-summary span {
  display: block;
  color: rgba(255, 255, 255, 0.56);
  font-size: 0.74rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
}

.wifi-switch,
.slot-row,
.bt-actions {
  margin-top: 0.75rem;
}

.slot-row,
.bt-actions,
.dialog-actions {
  display: flex;
  gap: 0.6rem;
  flex-wrap: wrap;
}

.wifi-list {
  margin-top: 0.75rem;
}

.wifi-item {
  margin-bottom: 0.5rem;
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.dialog-actions {
  justify-content: flex-end;
  margin-top: 0.75rem;
}

.settings-debug-panels {
  margin-top: 0.25rem;
}

.settings-header--panel {
  width: 100%;
  align-items: center;
}

.settings-debug-copy {
  margin-bottom: 1rem;
}

@media (min-width: 960px) {
  .settings-grid {
    grid-template-columns: minmax(0, 1.05fr) minmax(0, 0.95fr);
  }

  .connectivity-grid {
    grid-template-columns: minmax(0, 1.15fr) minmax(320px, 0.85fr);
    align-items: start;
  }

  .scripts-layout {
    grid-template-columns: minmax(0, 1fr) minmax(320px, 0.95fr);
    align-items: start;
  }
}
</style>