<script setup lang="ts">
import { computed } from "vue";
import { useDisplay } from "vuetify";
import type { DiscoveredDevice } from "../services/companion";
import { useCompanionStore } from "../stores/companion";

const store = useCompanionStore();
const display = useDisplay();

const showGate = computed(() => !store.isConnected);
const isBusy = computed(() => Boolean(
  store.activity.initialize
  || store.activity.scanDevices
  || store.activity.autoScan
  || store.activity.connectToDevice
  || store.activity.autoConnectToDevice,
));

const gateTitle = computed(() => {
  if (store.autoConnectPaused) {
    return "Connection Paused";
  }
  if (store.automationState === "connecting") {
    return "Connecting To Jukeboy";
  }
  if (store.activity.initialize) {
    return "Starting Companion";
  }
  return "Find Your Jukeboy";
});

const gateMessage = computed(() => {
  if (store.autoConnectPaused) {
    return "Search is paused after a manual disconnect.";
  }
  if (store.activeIssue) {
    return store.activeIssue.detail;
  }
  if (store.automationState === "connecting") {
    return "A nearby Jukeboy was found. Connecting now.";
  }
  if (store.automationState === "scanning") {
    return "Searching for a nearby Jukeboy.";
  }
  return "Searching will continue until a device is connected.";
});

const discoveredDevices = computed(() => store.discoveredDevices);

function deviceName(device: DiscoveredDevice): string {
  return device.name || device.address || "Jukeboy";
}

function scanOrResume(): void {
  if (store.autoConnectPaused) {
    store.resumeAutoConnect();
    return;
  }
  void store.scanDevices();
}

function connectDevice(device: DiscoveredDevice): void {
  void store.connectToDevice({ address: device.address, name: device.name });
}
</script>

<template>
  <v-dialog
    :model-value="showGate"
    persistent
    no-click-animation
    :fullscreen="display.smAndDown.value"
    :max-width="display.mdAndUp.value ? 520 : undefined"
    scrim="rgba(0, 0, 0, 0.78)"
    class="connection-gate-dialog"
  >
    <v-card class="connection-gate screen-sheet" color="surface" data-testid="connection-gate">
      <div class="connection-gate__header">
        <div class="connection-gate__signal" :class="{ 'is-paused': store.autoConnectPaused }">
          <v-progress-circular v-if="!store.autoConnectPaused" indeterminate size="28" width="3" color="primary" />
          <v-icon v-else icon="mdi-pause" size="28" />
        </div>

        <div>
          <p class="eyebrow">{{ store.automationLabel }}</p>
          <h2>{{ gateTitle }}</h2>
        </div>
      </div>

      <p class="connection-gate__copy" data-testid="connection-gate-message">{{ gateMessage }}</p>

      <v-alert
        v-if="store.activeIssue"
        type="warning"
        variant="tonal"
        color="surface-variant"
        class="connection-gate__issue companion-alert"
        data-testid="connection-gate-issue"
      >
        <template #title>{{ store.activeIssue.title }}</template>
        {{ store.activeIssue.recovery }}
      </v-alert>

      <div class="connection-gate__actions">
        <v-btn
          color="primary"
          size="large"
          :loading="isBusy"
          data-testid="connection-gate-scan"
          @click="scanOrResume"
        >
          <v-icon :icon="store.autoConnectPaused ? 'mdi-play' : 'mdi-radar'" class="mr-2" />
          {{ store.autoConnectPaused ? 'Resume Search' : 'Scan Now' }}
        </v-btn>
      </div>

      <v-list v-if="discoveredDevices.length" class="connection-gate__devices">
        <v-list-item v-for="device in discoveredDevices" :key="device.address" lines="one" class="connection-gate__device">
          <template #prepend>
            <v-icon icon="mdi-speaker-wireless" />
          </template>
          <v-list-item-title>{{ deviceName(device) }}</v-list-item-title>
          <template #append>
            <v-btn
              size="small"
              color="primary"
              :loading="Boolean(store.activity.connectToDevice || store.activity.autoConnectToDevice)"
              data-testid="connection-gate-connect-device"
              @click="connectDevice(device)"
            >
              Connect
            </v-btn>
          </template>
        </v-list-item>
      </v-list>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.connection-gate {
  display: grid;
  gap: 1rem;
  border-radius: 8px !important;
}

.connection-gate__header {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 0.9rem;
  align-items: center;
}

.connection-gate__header h2 {
  margin: 0;
  font-size: clamp(1.7rem, 4vw, 2.35rem);
  font-weight: 800;
  line-height: 1;
}

.connection-gate__signal {
  display: grid;
  place-items: center;
  width: 3.2rem;
  height: 3.2rem;
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: rgba(255, 255, 255, 0.05);
}

.connection-gate__signal.is-paused {
  color: rgba(255, 255, 255, 0.76);
}

.connection-gate__copy {
  margin: 0;
  color: rgba(255, 255, 255, 0.74);
  line-height: 1.45;
}

.connection-gate__issue {
  margin: 0;
}

.connection-gate__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.65rem;
}

.connection-gate__devices {
  display: grid;
  gap: 0.5rem;
  padding: 0;
  background: transparent;
}

.connection-gate__device {
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.03);
}

@media (max-width: 720px) {
  .connection-gate {
    min-height: 100%;
    align-content: center;
    border-radius: 0 !important;
  }
}
</style>