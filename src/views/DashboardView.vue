
<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useDisplay } from "vuetify";
import ArtworkCover from "../components/ArtworkCover.vue";
import { useCompanionStore } from "../stores/companion";
import { formatDuration } from "../utils/formatting";

const store = useCompanionStore();
const display = useDisplay();

const playback = computed(() => store.snapshotState.playback);
const album = computed(() => store.albumState.album);
const isReady = computed(() => store.connection.connected && store.isAuthenticated);
const canControlPlayback = computed(() => isReady.value && store.hasMountedCartridge);
const trackCount = computed(() => store.tracksState.track_count || store.albumState.cartridge.track_count || 0);

const trackMeta = computed(() => {
  return [playback.value.track_artist, album.value.name].filter(Boolean).join(" / ") || "Awaiting playback metadata";
});

const heroLabel = computed(() => {
  if (!store.connection.connected) {
    return "Disconnected";
  }
  if (!canControlPlayback.value) {
    return "Getting Ready";
  }
  return "Now Playing";
});

const heroTitle = computed(() => {
  if (!store.connection.connected) {
    return "Session Offline";
  }
  if (!store.isAuthenticated) {
    return "Authenticate Companion";
  }
  if (!store.hasMountedCartridge) {
    return "Insert A Cartridge";
  }
  return playback.value.track_title || "No track playing";
});

const heroSubtitle = computed(() => {
  if (!canControlPlayback.value) {
    return store.sessionMessage;
  }
  return trackMeta.value;
});

const heroArtworkHeight = computed(() => (display.width.value >= 760 ? 260 : 190));

onMounted(() => {
  void store.refreshDashboard();
});
</script>

<template>
  <section class="view-shell dashboard-view">
    <v-card class="screen-sheet hero-sheet dashboard-hero" color="surface">
      <div class="hero-copy-block">
        <p class="eyebrow">Dashboard</p>
        <p class="hero-kicker">{{ heroLabel }}</p>
        <h1 class="track-title" data-testid="dashboard-track-title">{{ heroTitle }}</h1>
        <p class="track-meta" data-testid="dashboard-track-meta">{{ heroSubtitle }}</p>
      </div>

      <div class="hero-art-stage">
        <ArtworkCover
          :title="album.name || playback.track_title || 'Mounted cartridge'"
          :subtitle="album.artist || playback.track_artist || 'Jukeboy Companion'"
          :src="album.artwork_data_url"
          :height="heroArtworkHeight"
        />
      </div>
    </v-card>

    <v-card class="screen-sheet" color="surface">
      <div class="list-header list-header--compact">
        <div>
          <p class="eyebrow">Track List</p>
          <h2 class="panel-title">Play from the cartridge queue</h2>
        </div>
        <strong class="track-list-count" data-testid="library-track-count">{{ trackCount }} loaded</strong>
      </div>

      <v-alert
        v-if="store.connection.connected && store.isAuthenticated && !store.hasMountedCartridge"
        type="info"
        variant="tonal"
        color="surface-variant"
        class="mt-4 companion-alert"
        data-testid="dashboard-no-cartridge"
      >
        No cartridge is currently mounted. The track list will populate as soon as media is inserted.
      </v-alert>

      <v-alert
        v-else-if="store.errors.refreshLibraryTracks"
        type="warning"
        variant="tonal"
        color="surface-variant"
        class="mt-4 companion-alert"
        data-testid="dashboard-track-error"
      >
        {{ store.errors.refreshLibraryTracks }}
      </v-alert>

      <v-alert
        v-else-if="store.connection.connected && store.isAuthenticated && store.hasMountedCartridge && store.tracksState.tracks.length === 0"
        type="info"
        variant="tonal"
        color="surface-variant"
        class="mt-4 companion-alert"
        data-testid="dashboard-track-empty"
      >
        No tracks were returned for the mounted cartridge yet. The library will refresh automatically as the device reports updated media state.
      </v-alert>

      <v-list v-if="store.hasMountedCartridge" lines="two" class="track-list">
        <v-list-item
          v-for="track in store.tracksState.tracks"
          :key="track.track_index"
          class="track-item"
          :data-testid="`track-item-${track.track_index}`"
          @click="store.playTrack(track.track_index)"
        >
          <template #prepend>
            <div class="track-index">{{ track.track_index + 1 }}</div>
          </template>
          <v-list-item-title>{{ track.title || 'Untitled track' }}</v-list-item-title>
          <v-list-item-subtitle>
            {{ track.artist || 'Unknown artist' }} · {{ formatDuration(track.duration_sec) }}
          </v-list-item-subtitle>
          <template #append>
            <v-icon icon="mdi-play-circle-outline" />
          </template>
        </v-list-item>
      </v-list>
    </v-card>

  </section>
</template>
<style scoped>

.dashboard-view {
  gap: 1rem;
}

.dashboard-hero {
  overflow: hidden;
}

.hero-copy-block {
  display: grid;
  gap: 0.75rem;
  align-content: start;
}

.hero-kicker,
.track-meta {
  margin: 0;
}

.hero-kicker {
  color: rgba(255, 255, 255, 0.64);
  font-size: 0.78rem;
  font-weight: 700;
  letter-spacing: 0.14em;
  text-transform: uppercase;
}

.track-title {
  margin: 0;
  font-size: clamp(2rem, 4.6vw, 3.5rem);
  font-weight: 800;
  letter-spacing: -0.06em;
  line-height: 0.95;
  max-width: 10ch;
}

.track-meta {
  color: rgba(255, 255, 255, 0.72);
  font-size: 1rem;
}

.track-list-count {
  display: block;
  color: rgba(255, 255, 255, 0.58);
  font-size: 0.72rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
}

.hero-art-stage {
  display: grid;
  align-self: start;
}

.panel-title {
  margin: 0;
  font-size: 1.45rem;
  font-weight: 800;
  letter-spacing: -0.04em;
}


.list-header {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  align-items: flex-end;
  gap: 1rem;
}

.list-header h2 {
  margin: 0;
  font-size: 1.2rem;
}

.list-header--compact {
  align-items: center;
}

.track-list-count {
  font-weight: 700;
  text-align: right;
}

.track-list {
  margin-top: 0.75rem;
}

.track-item {
  margin-bottom: 0.5rem;
  border: 1px solid rgba(255, 255, 255, 0.06);
  transition: background 160ms ease, transform 160ms ease;
}

.track-item:hover {
  background: rgba(255, 255, 255, 0.05);
  transform: translateY(-1px);
}

.track-index {
  display: inline-flex;
  align-items: center;
  justify-content: flex-end;
  min-width: 2.8ch;
  margin-right: 0.55rem;
  font-size: 0.95rem;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  letter-spacing: 0.02em;
  color: rgba(255, 255, 255, 0.72);
}

@media (max-width: 759px) {
  .track-title {
    max-width: none;
  }

  .panel-title {
    font-size: 1.2rem;
  }
}

</style>