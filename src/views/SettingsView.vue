
<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { useDisplay } from "vuetify";
import CommandPanel from "../components/debug/CommandPanel.vue";
import { useCompanionStore } from "../stores/companion";
import { formatTimestamp, titleizeToken } from "../utils/formatting";

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
  url: "",
  username: "",
  password: "",
});

watch(() => store.snapshotState.lastfm.auth_url, (value) => { if (!lastfmForm.url) lastfmForm.url = value; }, { immediate: true });
watch(() => store.snapshotState.lastfm.username, (value) => { if (!lastfmForm.username) lastfmForm.username = value; }, { immediate: true });

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
</script>

<template>
  <section class="view-shell settings-view">
    
    <v-card class="screen-sheet" color="surface">
      <p class="eyebrow">Settings</p>
      <h1 class="display-title">Trust And Control</h1>
      <p class="section-copy">
        Connection, trust, cloud integration, and the extracted backend smoke panel live here now.
      </p>
    </v-card>

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
          <v-list-item-subtitle>{{ device.address }}</v-list-item-subtitle>
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
          <p class="eyebrow">Settings</p>
          <h3>Endpoint and scrobble preferences</h3>
        </div>

        <div class="settings-form-grid">
          <v-text-field v-model="lastfmForm.url" label="Auth URL" />
        </div>

        <div class="settings-actions">
          <v-btn color="primary" data-testid="lastfm-save-url" @click="store.setLastfmAuthUrl(lastfmForm.url)">Save Auth URL</v-btn>
        </div>
      </div>

      <div class="settings-subsection">
        <div class="settings-subsection__title">
          <p class="eyebrow">Authenticate</p>
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

    <v-card class="screen-sheet" color="surface">
      <div class="settings-header">
        <div>
          <p class="eyebrow">Pairing & Trust</p>
          <h2>Trusted clients and background pairing</h2>
        </div>
        <v-chip color="primary">{{ store.trustedClientsState.trusted_count }} trusted</v-chip>
      </div>

      <v-alert type="info" variant="tonal" color="surface-variant" class="settings-automation-alert companion-alert">
        <template #title>
          {{ store.pairingState.pairing_pending ? 'Pairing in background' : 'Automatic pairing' }}
        </template>
        Saved credentials are retried whenever the companion reconnects. If the device still needs trust, pairing starts automatically and the generated sequence appears below.
      </v-alert>

      <div class="pairing-summary">
        <div>
          <span>Pending</span>
          <strong data-testid="pairing-pending">{{ store.pairingState.pairing_pending ? 'Yes' : 'No' }}</strong>
        </div>
        <div>
          <span>Progress</span>
          <strong data-testid="pairing-progress">{{ store.pairingState.pairing_progress }} / {{ store.pairingState.pairing_required }}</strong>
        </div>
        <div>
          <span>Sequence</span>
          <strong>{{ store.pairingState.button_sequence.join(' -> ') || 'Backend generated' }}</strong>
        </div>
      </div>

      <v-list class="device-list">
        <v-list-item v-for="client in store.trustedClientsState.clients" :key="client.client_id" lines="two" class="device-item">
          <v-list-item-title>{{ client.app_name || 'Unnamed client' }}</v-list-item-title>
          <v-list-item-subtitle>{{ client.client_id }} · {{ formatTimestamp(client.created_at) }}</v-list-item-subtitle>
          <template #append>
            <v-btn size="small" variant="outlined" color="primary" @click="store.revokeTrustedClient(client.client_id)">
              Revoke
            </v-btn>
          </template>
        </v-list-item>
      </v-list>
    </v-card>
  
    <v-divider class="my-4" />
    
    <v-card class="screen-sheet" color="surface">
      <p class="eyebrow">Connectivity</p>
      <h1 class="display-title">Network And Audio</h1>
      <p class="section-copy">
        Manage Wi-Fi handoff, stored slots, and Bluetooth audio without leaving the monotone control surface.
      </p>
    </v-card>

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
        <div>
          <span>IP</span>
          <strong>{{ store.snapshotState.wifi.ip || 'No address' }}</strong>
        </div>
      </div>

      <v-switch v-model="autoreconnectModel" label="Auto-reconnect" class="wifi-switch" data-testid="wifi-autoreconnect" />

      <div class="slot-row">
        <v-btn variant="outlined" data-testid="wifi-connect" @click="connectSavedWifi">Connect</v-btn>
        <v-chip variant="outlined" size="small">Saved slot {{ preferredWifiSlot }}</v-chip>
        <v-btn variant="text" color="primary" data-testid="wifi-disconnect" @click="store.disconnectWifiNetwork()">Disconnect Wi-Fi</v-btn>
      </div>

      <v-list class="wifi-list">
        <v-list-item
          v-for="network in store.wifiScanState.results"
          :key="network.ssid"
          lines="two"
          class="wifi-item"
          @click="openWifiDialog(network.ssid)"
        >
          <v-list-item-title>{{ network.ssid }}</v-list-item-title>
          <v-list-item-subtitle>
            RSSI {{ network.rssi ?? 'n/a' }} · Channel {{ network.channel ?? 'n/a' }}
          </v-list-item-subtitle>
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

      <div class="wifi-summary">
        <div>
          <span>Bonded</span>
          <strong>{{ store.snapshotState.bluetooth.bonded_count }}</strong>
        </div>
        <div>
          <span>Playback Output</span>
          <strong data-testid="bt-output">{{ titleizeToken(store.snapshotState.playback.output_target) }}</strong>
        </div>
      </div>

      <div class="bt-actions">
        <v-btn color="primary" size="large" data-testid="bt-connect-last" @click="store.runBluetoothAction('connect-last')">Connect Last</v-btn>
        <v-btn variant="outlined" size="large" data-testid="bt-pair-best" @click="store.runBluetoothAction('pair-best')">Pair Best Candidate</v-btn>
        <v-btn variant="text" size="large" color="primary" data-testid="bt-disconnect" @click="store.runBluetoothAction('disconnect')">Disconnect</v-btn>
      </div>
    </v-card>

    <v-card class="screen-sheet" color="surface">
      <p class="eyebrow">Hardware Info</p>
      <h2 class="info-title">Protocol and session facts</h2>
      <div class="pairing-summary hardware-grid">
        <div>
          <span>Connected App</span>
          <strong>{{ store.helloState.app_name || 'Unknown' }}</strong>
        </div>
        <div>
          <span>Protocol Version</span>
          <strong>{{ store.capabilitiesState.protocol_version ?? store.helloState.protocol_version ?? 'n/a' }}</strong>
        </div>
        <div>
          <span>MTU</span>
          <strong>{{ store.capabilitiesState.mtu ?? 'n/a' }}</strong>
        </div>
        <div>
          <span>Generation</span>
          <strong data-testid="hardware-generation">{{ store.snapshotState.generation ?? 'n/a' }}</strong>
        </div>
        <div>
          <span>Uptime</span>
          <strong>{{ store.snapshotState.uptime_ms ?? 0 }} ms</strong>
        </div>
      </div>
    </v-card>

    <v-expansion-panels variant="accordion">
      <v-expansion-panel>
        <v-expansion-panel-title>
          <div>
            <p class="eyebrow">Debug Menu</p>
            <strong>Smoke test panel</strong>
          </div>
        </v-expansion-panel-title>
        <v-expansion-panel-text>
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
  gap: 1.4rem;
}

.settings-header {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 1rem;
  align-items: flex-end;
}

.settings-header h2,
.info-title {
  margin: 0;
  font-size: 1.2rem;
}

.settings-actions,
.settings-form-grid,
.pairing-summary {
  margin-top: 1rem;
}

.settings-automation-alert {
  margin-top: 1rem;
}

.settings-actions {
  display: flex;
  gap: 0.75rem;
  flex-wrap: wrap;
}

.settings-subsection {
  margin-top: 1rem;
}

.settings-subsection + .settings-subsection {
  padding-top: 1rem;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
}

.settings-subsection__title h3 {
  margin: 0;
  font-size: 1rem;
}

.settings-form-grid,
.pairing-summary {
  display: grid;
  gap: 0.75rem;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
}

.pairing-summary div {
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 20px;
  padding: 0.9rem;
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
  margin-top: 1rem;
}

.device-item {
  margin-bottom: 0.7rem;
  border: 1px solid rgba(255, 255, 255, 0.06);
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
  gap: 0.75rem;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  margin-top: 1rem;
}

.wifi-summary div {
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 20px;
  padding: 0.9rem;
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
  margin-top: 1rem;
}

.slot-row,
.bt-actions,
.dialog-actions {
  display: flex;
  gap: 0.75rem;
  flex-wrap: wrap;
}

.wifi-list {
  margin-top: 1rem;
}

.wifi-item {
  margin-bottom: 0.65rem;
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.dialog-actions {
  justify-content: flex-end;
  margin-top: 1rem;
}

.my-4 { margin-top: 1rem; margin-bottom: 1rem; }
</style>