<script setup lang="ts">
import { computed, onMounted } from "vue";
import { storeToRefs } from "pinia";
import { RouterView, useRoute, useRouter } from "vue-router";

import { useCompanionStore } from "../stores/companion";
import ConnectionGateDialog from "./ConnectionGateDialog.vue";
import PlaybackStrip from "./PlaybackStrip.vue";

const route = useRoute();
const router = useRouter();
const store = useCompanionStore();
const { notifications, statusLabel } = storeToRefs(store);
const topbarStatusLabel = computed(() => statusLabel.value);

const statusClass = computed(() => (store.connection.connected ? "is-active" : "is-idle"));

onMounted(() => {
  void store.initialize();
});

function toggleView() {
  if (route.name === "settings") {
    router.push({ name: "dashboard" });
  } else {
    router.push({ name: "settings" });
  }
}
</script>

<template>
  <v-app class="jukeboy-app">
    <v-app-bar class="shell-app-bar" elevation="0" color="transparent" height="92">
      <div class="view-frame flex-1-1-100 topbar-frame">
        <div class="brand-lockup">
          <span class="status-dot" :class="statusClass" />
          <div class="brand-lockup-text">
            <h1 class="brand-title">Jukeboy Companion</h1>
            <span class="status-text" data-testid="shell-status">{{ topbarStatusLabel }}</span>
          </div>
        </div>

        <v-btn
          v-if="store.isConnected"
          color="primary"
          variant="text"
          size="large"
          class="toggle-button"
          data-testid="shell-toggle-view"
          @click="toggleView"
        >
          <v-icon :icon="route.name === 'settings' ? 'mdi-play-circle-outline' : 'mdi-cog-outline'" size="28" class="mr-2" />
          <span>{{ route.name === 'settings' ? 'Dashboard' : 'Settings' }}</span>
        </v-btn>
      </div>
    </v-app-bar>

    <v-main class="shell-main">
      <div class="view-frame">
        <div v-if="notifications.length" class="shell-notification-stack" data-testid="shell-notification-stack">
          <v-alert
            v-for="notification in notifications"
            :key="notification.id"
            :type="notification.level === 'recovery' ? 'success' : 'warning'"
            variant="tonal"
            :color="notification.level === 'recovery' ? 'primary' : 'surface-variant'"
            closable
            class="shell-notification companion-alert"
            :data-testid="`shell-notification-${notification.level}`"
            @click:close="store.dismissNotification(notification.id)"
          >
            <template #title>
              <div class="shell-notification__title-row">
                <span>{{ notification.title }}</span>
                <span class="shell-notification__time">{{ notification.time }}</span>
              </div>
            </template>
            <div>{{ notification.detail }}</div>
            <div v-if="notification.recovery" class="shell-notification__recovery">
              {{ notification.recovery }}
            </div>
          </v-alert>
        </div>

        <RouterView v-if="store.isConnected" v-slot="{ Component, route: current }">
          <v-fade-transition mode="out-in">
            <component :is="Component" :key="current.fullPath" />
          </v-fade-transition>
        </RouterView>
      </div>
    </v-main>

    <PlaybackStrip v-if="store.isConnected" />
    <ConnectionGateDialog />
  </v-app>
</template>

<style scoped>
.shell-app-bar {
  position: relative;
  isolation: isolate;
  background: transparent !important;
  overflow: visible !important;
}

.shell-app-bar::before {
  content: "";
  position: absolute;
  inset: 0 0 -26px 0;
  background:
    linear-gradient(
      180deg,
      rgba(0, 0, 0, 0.98) 0%,
      rgba(0, 0, 0, 0.92) 42%,
      rgba(0, 0, 0, 0.72) 70%,
      rgba(0, 0, 0, 0.28) 88%,
      rgba(0, 0, 0, 0) 100%
    );
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  mask-image: linear-gradient(
    180deg,
    rgba(0, 0, 0, 1) 0%,
    rgba(0, 0, 0, 1) 54%,
    rgba(0, 0, 0, 0.82) 74%,
    rgba(0, 0, 0, 0.34) 90%,
    rgba(0, 0, 0, 0) 100%
  );
  -webkit-mask-image: linear-gradient(
    180deg,
    rgba(0, 0, 0, 1) 0%,
    rgba(0, 0, 0, 1) 54%,
    rgba(0, 0, 0, 0.82) 74%,
    rgba(0, 0, 0, 0.34) 90%,
    rgba(0, 0, 0, 0) 100%
  );
  pointer-events: none;
  z-index: 0;
}

.topbar-frame {
  display: flex;
  justify-content: space-between;
  align-items: center;
  position: relative;
  z-index: 1;
  min-height: 92px;
  gap: 1rem;
  padding-top: 0.45rem;
  padding-bottom: 0.1rem;
}

:deep(.shell-app-bar .v-toolbar__content) {
  min-height: 92px !important;
  height: 92px !important;
  padding-top: 0;
  padding-bottom: 0;
}

.brand-lockup {
  display: flex;
  align-items: center;
  gap: 0.9rem;
}

.brand-lockup-text {
  display: flex;
  flex-direction: column;
}

.brand-title {
  font-size: 1.08rem;
  font-weight: 800;
  letter-spacing: -0.04em;
  margin: 0;
  line-height: 1;
}

.status-text {
  font-size: 0.68rem;
  color: rgba(255, 255, 255, 0.6);
  text-transform: uppercase;
  letter-spacing: 0.1em;
}

.toggle-button {
  font-weight: 700;
  letter-spacing: 0.04em;
  padding-inline: 0.8rem !important;
}

.shell-notification-stack {
  display: grid;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
}

.shell-notification {
  margin-bottom: 0;
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.shell-notification__title-row {
  display: flex;
  justify-content: space-between;
  gap: 0.75rem;
  align-items: baseline;
}

.shell-notification__time {
  font-size: 0.74rem;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: rgba(255, 255, 255, 0.6);
}

.shell-notification__recovery {
  margin-top: 0.45rem;
  color: rgba(255, 255, 255, 0.88);
}

@media (max-width: 720px) {
  .shell-app-bar {
    height: 106px !important;
  }

  .shell-app-bar::before {
    inset: 0 0 -30px 0;
  }

  .topbar-frame {
    min-height: 106px;
    padding-top: calc(env(safe-area-inset-top) + 0.65rem);
    padding-bottom: 0.4rem;
    align-items: flex-start;
  }

  :deep(.shell-app-bar .v-toolbar__content) {
    min-height: 106px !important;
    height: 106px !important;
    align-items: flex-start;
  }

  .brand-lockup {
    align-items: flex-start;
  }

  .brand-lockup-text {
    padding-top: 0.1rem;
  }

  .brand-title {
    font-size: 0.98rem;
  }

  .status-text {
    font-size: 0.62rem;
  }

  .toggle-button {
    align-self: flex-start;
    min-width: 0;
    margin-top: 0.15rem;
    padding-inline: 0.2rem !important;
  }
}
</style>
