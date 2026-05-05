<script setup lang="ts">
import { computed, onMounted } from "vue";
import { storeToRefs } from "pinia";
import { RouterView, useRoute, useRouter } from "vue-router";

import { useCompanionStore } from "../stores/companion";
import PlaybackStrip from "./PlaybackStrip.vue";

const route = useRoute();
const router = useRouter();
const store = useCompanionStore();
const { activeIssue, statusLabel } = storeToRefs(store);
const topbarStatusLabel = computed(() => statusLabel.value.replace(/\s*·\s*live sync active$/i, ""));

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
    <v-app-bar class="shell-app-bar" elevation="0" color="transparent" height="101">
      <div class="view-frame flex-1-1-100 topbar-frame">
        <div class="brand-lockup">
          <span class="status-dot" :class="statusClass" />
          <div class="brand-lockup-text">
            <h1 class="brand-title">Jukeboy Companion</h1>
            <span class="status-text" data-testid="shell-status">{{ topbarStatusLabel }}</span>
          </div>
        </div>

        <v-btn
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
        <v-slide-y-transition>
          <v-alert
            v-if="activeIssue"
            type="warning"
            variant="tonal"
            color="surface-variant"
            closable
            class="shell-issue-banner companion-alert"
            @click:close="store.dismissIssue()"
          >
            <template #title>
              {{ activeIssue.title }}
            </template>
            <div>{{ activeIssue.detail }}</div>
            <div v-if="activeIssue.recovery" class="shell-issue-banner__recovery">
              {{ activeIssue.recovery }}
            </div>
          </v-alert>
        </v-slide-y-transition>

        <RouterView v-slot="{ Component, route: current }">
          <v-fade-transition mode="out-in">
            <component :is="Component" :key="current.fullPath" />
          </v-fade-transition>
        </RouterView>
      </div>
    </v-main>

    <PlaybackStrip />
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
  min-height: 101px;
}

:deep(.shell-app-bar .v-toolbar__content) {
  min-height: 101px !important;
  height: 101px !important;
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
  font-size: 1.2rem;
  font-weight: 800;
  letter-spacing: -0.04em;
  margin: 0;
  line-height: 1;
}

.status-text {
  font-size: 0.75rem;
  color: rgba(255, 255, 255, 0.6);
  text-transform: uppercase;
  letter-spacing: 0.1em;
}

.toggle-button {
  font-weight: 700;
  letter-spacing: 0.05em;
}

.shell-issue-banner {
  margin-bottom: 1rem;
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.shell-issue-banner__recovery {
  margin-top: 0.45rem;
  color: rgba(255, 255, 255, 0.88);
}

@media (max-width: 720px) {
  .shell-app-bar {
    height: 122px !important;
  }

  .shell-app-bar::before {
    inset: 0 0 -30px 0;
  }

  .topbar-frame {
    min-height: 122px;
    padding-top: calc(env(safe-area-inset-top) + 0.95rem);
    padding-bottom: 0.55rem;
    align-items: flex-start;
  }

  :deep(.shell-app-bar .v-toolbar__content) {
    min-height: 122px !important;
    height: 122px !important;
    align-items: flex-start;
  }

  .brand-lockup {
    align-items: flex-start;
  }

  .brand-lockup-text {
    padding-top: 0.1rem;
  }

  .brand-title {
    font-size: 1.08rem;
  }

  .status-text {
    font-size: 0.68rem;
  }

  .toggle-button {
    align-self: flex-start;
    min-width: 0;
    margin-top: 0.2rem;
    padding-inline: 0.25rem !important;
  }
}
</style>
