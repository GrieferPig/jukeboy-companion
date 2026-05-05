
<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import ArtworkCover from "../components/ArtworkCover.vue";
import { useCompanionStore } from "../stores/companion";
import { formatDuration, titleizeToken } from "../utils/formatting";

const router = useRouter();
const store = useCompanionStore();

const playback = computed(() => store.snapshotState.playback);
const album = computed(() => store.albumState.album);
const isReady = computed(() => store.connection.connected && store.isAuthenticated);
const canControlPlayback = computed(() => isReady.value && store.hasMountedCartridge);

const trackMeta = computed(() => {
  return [playback.value.track_artist, album.value.name].filter(Boolean).join(" / ") || "Awaiting playback metadata";
});

onMounted(() => {
  void store.refreshDashboard();
  void store.refreshLibrary();
});

function openSettings(): void {
  void router.push({ name: "settings" });
}
</script>

<template>
  <section class="view-shell dashboard-view">
    <v-card class="screen-sheet hero-sheet" color="surface">
      <div class="hero-copy-block">
        <p class="eyebrow">Dashboard</p>
        <h1 class="display-title">Now Playing</h1>
        <p v-if="!store.connection.connected" class="section-copy">
          {{ store.sessionMessage }}
        </p>
        <p v-else-if="!canControlPlayback" class="section-copy" data-testid="dashboard-session-message">
          {{ store.sessionMessage }}
        </p>
        <template v-else>
          <p class="track-title" data-testid="dashboard-track-title">{{ playback.track_title || "No track playing" }}</p>
          <p class="track-meta" data-testid="dashboard-track-meta">{{ trackMeta }}</p>
          <div class="hero-metrics">
            <div>
              <span>Mode</span>
              <strong data-testid="dashboard-mode">{{ titleizeToken(playback.playback_mode) }}</strong>
            </div>
            <div>
              <span>Output</span>
              <strong data-testid="dashboard-output">{{ titleizeToken(playback.output_target) }}</strong>
            </div>
            <div>
              <span>Volume</span>
              <strong data-testid="dashboard-volume">{{ playback.volume_percent ?? 0 }}%</strong>
            </div>
          </div>
        </template>

        <v-btn v-if="!isReady" color="primary" size="large" @click="openSettings">
          Open Settings
        </v-btn>
      </div>

      <ArtworkCover
        :title="album.name || playback.track_title || 'Mounted cartridge'"
        :subtitle="album.artist || playback.track_artist || 'Jukeboy Companion'"
        :seed="`${store.snapshotState.cartridge.checksum ?? 'jukeboy'}-${album.name}`"
        :height="420"
      />
    </v-card>

    <v-card v-if="store.connection.connected && !canControlPlayback" class="screen-sheet" color="surface">
      <p class="eyebrow">Transport</p>
      <h2>Playback Control</h2>
      <p class="section-copy" data-testid="dashboard-transport-empty">
        {{ store.sessionMessage }}
      </p>
    </v-card>
  
    <v-divider class="my-4" />
      
    <v-card class="screen-sheet" color="surface">
      <p class="eyebrow">Library</p>
      <h1 class="display-title">Cartridge Explorer</h1>
      <p class="section-copy">
        {{ store.albumState.album.description || 'The currently mounted cartridge, with direct play-on-select controls and recent history.' }}
      </p>

      <div class="library-hero">
        <ArtworkCover
          :title="store.albumState.album.name || 'Mounted cartridge'"
          :subtitle="store.albumState.album.artist || 'No artist loaded'"
          :seed="`${store.albumState.cartridge.checksum ?? 'library'}-${store.albumState.album.name}`"
          :height="260"
        />

        <div class="library-meta">
          <div>
            <span>Album</span>
            <strong>{{ store.albumState.album.name || 'No cartridge metadata' }}</strong>
          </div>
          <div>
            <span>Artist</span>
            <strong>{{ store.albumState.album.artist || 'Unknown artist' }}</strong>
          </div>
          <div>
            <span>Year</span>
            <strong>{{ store.albumState.album.year ?? '----' }}</strong>
          </div>
          <div>
            <span>Tracks</span>
            <strong data-testid="library-track-count">{{ store.tracksState.track_count || store.albumState.cartridge.track_count || 0 }}</strong>
          </div>
        </div>
      </div>
    </v-card>

    <v-card class="screen-sheet" color="surface">
      <div class="list-header">
        <div>
          <p class="eyebrow">Track List</p>
        </div>
      </div>

      <v-alert
        v-if="store.connection.connected && store.isAuthenticated && !store.hasMountedCartridge"
        type="info"
        variant="tonal"
        color="surface-variant"
        class="mt-4 companion-alert"
        data-testid="dashboard-no-cartridge"
      >
        No cartridge is currently mounted. Recently played albums remain available below, and the track list will populate as soon as media is inserted.
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

    <v-card class="screen-sheet" color="surface">
      <div class="list-header">
        <div>
          <p class="eyebrow">History</p>
          <h2>Recently Played Albums</h2>
        </div>
      </div>

      <v-slide-group show-arrows class="history-strip">
        <v-slide-group-item v-for="album in store.historyAlbumsState.albums" :key="album.checksum">
          <div class="history-card">
            <ArtworkCover
              :title="album.album_name || 'History entry'"
              :subtitle="album.album_artist || 'Unknown artist'"
              :seed="String(album.checksum)"
              :height="172"
            />
            <strong>{{ album.album_name || 'Untitled album' }}</strong>
            <span>{{ album.album_artist || 'Unknown artist' }}</span>
          </div>
        </v-slide-group-item>
      </v-slide-group>
    </v-card>
  
  </section>
</template>
<style scoped>

.dashboard-view {
  gap: 1.4rem;
}

.hero-copy-block {
  display: grid;
  gap: 1rem;
  align-content: start;
}

.track-title {
  margin: 0;
  font-size: clamp(2rem, 5vw, 4rem);
  font-weight: 800;
  letter-spacing: -0.06em;
  line-height: 0.95;
}

.track-meta {
  margin: 0;
  color: rgba(255, 255, 255, 0.72);
  font-size: 1.1rem;
}

.hero-metrics {
  display: grid;
  gap: 0.75rem;
  grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
}

.hero-metrics div {
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 20px;
  padding: 0.85rem 1rem;
  background: rgba(255, 255, 255, 0.03);
}

.hero-metrics span {
  display: block;
  font-size: 0.74rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  color: rgba(255, 255, 255, 0.56);
}


.library-view {
  gap: 1.4rem;
}

.library-hero {
  display: grid;
  gap: 1rem;
  margin-top: 1rem;
}

.library-meta {
  display: grid;
  gap: 0.75rem;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
}

.library-meta div,
.history-card {
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 20px;
  padding: 0.9rem;
  background: rgba(255, 255, 255, 0.03);
}

.library-meta span,
.history-card span {
  display: block;
  color: rgba(255, 255, 255, 0.58);
  font-size: 0.76rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
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

.track-list {
  margin-top: 1rem;
}

.track-item {
  margin-bottom: 0.7rem;
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
  min-width: 3.2ch;
  margin-right: 0.65rem;
  font-size: 1.05rem;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  letter-spacing: 0.02em;
  color: rgba(255, 255, 255, 0.72);
}

.history-strip {
  margin-top: 1rem;
}

.history-card {
  width: 220px;
  display: grid;
  gap: 0.7rem;
  margin-right: 0.85rem;
}

.history-card strong {
  line-height: 1.1;
}

@media (min-width: 960px) {
  .library-hero {
    grid-template-columns: minmax(260px, 320px) minmax(0, 1fr);
    align-items: start;
  }
}

.my-4 { margin-top: 1rem; margin-bottom: 1rem; }
</style>