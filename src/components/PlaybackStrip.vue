<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useDisplay } from "vuetify";
import type { OutputTarget, PlaybackMode } from "../services/companion";
import { useCompanionStore } from "../stores/companion";
import { clampNumber, formatDuration } from "../utils/formatting";
import ArtworkCover from "./ArtworkCover.vue";

const store = useCompanionStore();
const display = useDisplay();

const playback = computed(() => store.snapshotState.playback);
const album = computed(() => store.albumState.album);
const isReady = computed(() => store.connection.connected && store.isAuthenticated);
const canControlPlayback = computed(() => isReady.value && store.hasMountedCartridge);
const stripTitle = computed(() => {
  if (!store.connection.connected) {
    return "Not connected";
  }
  if (!store.isAuthenticated) {
    return "Awaiting authentication";
  }
  if (!store.hasMountedCartridge) {
    return "No cartridge inserted";
  }
  return playback.value.track_title || "No track playing";
});
const stripSubtitle = computed(() => {
  if (!store.connection.connected) {
    return "Connect a Jukeboy to load playback state.";
  }
  if (!store.isAuthenticated) {
    return "Pair or authenticate in Settings to unlock playback controls.";
  }
  if (!store.hasMountedCartridge) {
    return "Recently played albums remain visible until media is inserted.";
  }
  return playback.value.track_artist || album.value.artist || "Unknown artist";
});
const isMobileStrip = computed(() => display.width.value <= 720);
const playIconSize = computed(() => (isMobileStrip.value ? 20 : 28));

const duration = computed(() => playback.value.duration_sec ?? 0);
const progressDraft = ref(0);
const volumeDraft = ref(50);
const volumeMenu = ref(false);
const outputMenu = ref(false);
const isSeeking = ref(false);
const isAdjustingVolume = ref(false);
const titleLineRef = ref<HTMLElement | null>(null);
const artistLineRef = ref<HTMLElement | null>(null);
const titleOverflow = ref(false);
const artistOverflow = ref(false);

const playbackModeOrder: PlaybackMode[] = ["sequential", "shuffle", "single_repeat"];
const titleMarqueeDuration = computed(() => marqueeDuration(stripTitle.value));
const artistMarqueeDuration = computed(() => marqueeDuration(stripSubtitle.value));

let marqueeObserver: ResizeObserver | null = null;

const modeButtonIcon = computed(() => {
  switch (playback.value.playback_mode) {
    case "shuffle":
      return "mdi-shuffle";
    case "single_repeat":
      return "mdi-repeat-once";
    default:
      return "mdi-repeat";
  }
});

const modeButtonColor = computed(() => (playback.value.playback_mode === "sequential" ? "grey-lighten-1" : "primary"));

const outputButtonIcon = computed(() => (playback.value.output_target === "bluetooth" ? "mdi-speaker-wireless" : "mdi-speaker"));

const outputButtonColor = computed(() => (playback.value.output_target === "bluetooth" ? "primary" : "grey-lighten-1"));

watch(() => playback.value.position_sec, (value) => {
  if (!isSeeking.value) progressDraft.value = value ?? 0;
}, { immediate: true });

watch(() => playback.value.volume_percent, (value) => {
  if (!isAdjustingVolume.value) volumeDraft.value = value ?? 50;
}, { immediate: true });

watch([stripTitle, stripSubtitle, isMobileStrip], async () => {
  await nextTick();
  updateOverflowState();
});

onMounted(async () => {
  await nextTick();
  updateOverflowState();

  if (typeof ResizeObserver === "undefined") {
    return;
  }

  marqueeObserver = new ResizeObserver(() => {
    updateOverflowState();
  });

  if (titleLineRef.value) {
    marqueeObserver.observe(titleLineRef.value);
  }
  if (artistLineRef.value) {
    marqueeObserver.observe(artistLineRef.value);
  }
});

onBeforeUnmount(() => {
  marqueeObserver?.disconnect();
});

function marqueeDuration(text: string): string {
  return `${Math.max(8, Math.ceil(text.length * 0.35))}s`;
}

function measureTextWidth(element: HTMLElement | null, text: string): number {
  if (!element) {
    return 0;
  }

  const styles = window.getComputedStyle(element);
  const canvas = document.createElement("canvas");
  const context = canvas.getContext("2d");
  if (!context) {
    return 0;
  }

  context.font = `${styles.fontStyle} ${styles.fontVariant} ${styles.fontWeight} ${styles.fontSize} ${styles.fontFamily}`;
  const letterSpacing = Number.parseFloat(styles.letterSpacing);
  const spacing = Number.isFinite(letterSpacing) ? Math.max(text.length - 1, 0) * letterSpacing : 0;
  return context.measureText(text).width + spacing;
}

function hasOverflow(element: HTMLElement | null, text: string): boolean {
  if (!element || !text) {
    return false;
  }
  return measureTextWidth(element, text) - element.clientWidth > 2;
}

function updateOverflowState(): void {
  titleOverflow.value = hasOverflow(titleLineRef.value, stripTitle.value);
  artistOverflow.value = hasOverflow(artistLineRef.value, stripSubtitle.value);
}

async function commitSeek() {
  isSeeking.value = false;
  await store.seekTo(clampNumber(progressDraft.value, 0, duration.value));
}

async function commitVolume() {
  isAdjustingVolume.value = false;
  await store.setVolume(clampNumber(volumeDraft.value, 0, 100));
  if (isMobileStrip.value) {
    volumeMenu.value = false;
  }
}

function togglePlay() {
  void store.playPause();
}

function cyclePlaybackMode() {
  const currentIndex = playbackModeOrder.indexOf(playback.value.playback_mode ?? "sequential");
  const nextMode = playbackModeOrder[(currentIndex + 1) % playbackModeOrder.length] ?? "sequential";
  void store.setPlaybackMode(nextMode);
}

function setOutputTarget(outputTarget: OutputTarget) {
  outputMenu.value = false;
  void store.setOutputTarget(outputTarget);
}
</script>

<template>
  <v-footer app class="playback-strip" elevation="8">
    <!-- Left: Track Info -->
    <div class="strip-left">
      <ArtworkCover
        v-if="playback.track_title || album.name"
        :title="album.name || playback.track_title || 'Mounted cartridge'"
        :subtitle="album.artist || playback.track_artist || 'Jukeboy Companion'"
        :seed="`${store.snapshotState.cartridge.checksum ?? 'jukeboy'}-${album.name}`"
        :height="56"
        class="strip-artwork"
      />
      <div v-else class="placeholder-art" />
      <div class="strip-meta">
        <div
          ref="titleLineRef"
          :class="['strip-title', 'strip-line', { 'strip-line--marquee': titleOverflow }]"
          data-testid="strip-title"
        >
          <div v-if="titleOverflow" class="strip-line__track" :style="{ '--marquee-duration': titleMarqueeDuration }">
            <span class="strip-line__label">{{ stripTitle }}</span>
            <span class="strip-line__label" aria-hidden="true">{{ stripTitle }}</span>
          </div>
          <span v-else class="strip-line__label">{{ stripTitle }}</span>
        </div>
        <div
          ref="artistLineRef"
          :class="['strip-artist', 'strip-line', { 'strip-line--marquee': artistOverflow }]"
          data-testid="strip-artist"
        >
          <div v-if="artistOverflow" class="strip-line__track" :style="{ '--marquee-duration': artistMarqueeDuration }">
            <span class="strip-line__label">{{ stripSubtitle }}</span>
            <span class="strip-line__label" aria-hidden="true">{{ stripSubtitle }}</span>
          </div>
          <span v-else class="strip-line__label">{{ stripSubtitle }}</span>
        </div>
      </div>
    </div>

    <!-- Center: Transport -->
    <div class="strip-center">
      <div class="strip-actions">
        <div class="transport-group transport-group--left">
          <v-btn
            :disabled="!canControlPlayback"
            icon
            variant="plain"
            class="action-btn"
            color="grey-lighten-1"
            data-testid="strip-prev"
            @click="store.previousTrack()"
          >
            <v-icon icon="mdi-skip-previous" size="24" />
          </v-btn>
        </div>

        <v-btn :disabled="!canControlPlayback" icon color="white" class="play-btn" data-testid="strip-play" @click="togglePlay">
          <v-icon :icon="playback.playing ? 'mdi-pause' : 'mdi-play'" color="black" :size="playIconSize" />
        </v-btn>

        <div class="transport-group transport-group--right">
          <v-btn
            :disabled="!canControlPlayback"
            icon
            variant="plain"
            class="action-btn"
            color="grey-lighten-1"
            data-testid="strip-next"
            @click="store.nextTrack()"
          >
            <v-icon icon="mdi-skip-next" size="24" />
          </v-btn>
          <v-btn
            :disabled="!canControlPlayback"
            icon
            variant="plain"
            class="action-btn"
            :color="modeButtonColor"
            data-testid="strip-repeat"
            @click="cyclePlaybackMode"
          >
            <v-icon :icon="modeButtonIcon" size="20" />
          </v-btn>

          <template v-if="isMobileStrip">
            <v-menu
              v-model="outputMenu"
              location="top end"
              :offset="14"
              transition="fade-transition"
            >
              <template #activator="{ props }">
                <v-btn
                  v-bind="props"
                  :disabled="!canControlPlayback"
                  icon
                  variant="plain"
                  class="action-btn"
                  :color="outputButtonColor"
                  data-testid="strip-output-menu"
                >
                  <v-icon :icon="outputButtonIcon" size="20" />
                </v-btn>
              </template>

              <v-card class="output-popout" color="surface">
                <div class="output-popout__label">Playback output</div>
                <div class="output-popout__actions">
                  <v-btn
                    block
                    :variant="playback.output_target === 'i2s' ? 'flat' : 'outlined'"
                    :color="playback.output_target === 'i2s' ? 'primary' : undefined"
                    data-testid="strip-output-i2s"
                    @click="setOutputTarget('i2s')"
                  >
                    I2S
                  </v-btn>
                  <v-btn
                    block
                    :variant="playback.output_target === 'bluetooth' ? 'flat' : 'outlined'"
                    :color="playback.output_target === 'bluetooth' ? 'primary' : undefined"
                    data-testid="strip-output-bluetooth"
                    @click="setOutputTarget('bluetooth')"
                  >
                    Bluetooth
                  </v-btn>
                </div>
              </v-card>
            </v-menu>
          </template>

          <template v-if="isMobileStrip">
            <v-menu
              v-model="volumeMenu"
              location="top end"
              :offset="14"
              :close-on-content-click="false"
              transition="fade-transition"
            >
              <template #activator="{ props }">
                <v-btn
                  v-bind="props"
                  :disabled="!canControlPlayback"
                  icon
                  variant="plain"
                  class="volume-trigger"
                >
                  <v-icon icon="mdi-volume-high" color="grey-lighten-1" size="20" />
                </v-btn>
              </template>

              <v-card class="volume-popout" color="surface">
                <div class="volume-popout__label">Volume</div>
                <v-slider
                  :disabled="!canControlPlayback"
                  v-model="volumeDraft"
                  :max="100"
                  color="white"
                  track-color="grey-darken-2"
                  class="volume-slider volume-slider--popout"
                  hide-details
                  thumb-size="12"
                  @start="isAdjustingVolume = true"
                  @end="commitVolume"
                />
              </v-card>
            </v-menu>
          </template>
        </div>
      </div>
      <div class="strip-progress">
        <span class="time-text">{{ canControlPlayback ? formatDuration(progressDraft) : "--:--" }}</span>
        <v-slider
          :disabled="!canControlPlayback"
          v-model="progressDraft"
          :max="Math.max(duration, 1)"
          color="white"
          track-color="grey-darken-2"
          class="progress-slider"
          hide-details
          thumb-size="12"
          @start="isSeeking = true"
          @end="commitSeek"
        />
        <span class="time-text">{{ canControlPlayback ? formatDuration(duration) : "--:--" }}</span>
      </div>
    </div>

    <!-- Right: Volume/Misc -->
    <div v-if="!isMobileStrip" class="strip-right">
      <v-menu
        v-model="outputMenu"
        location="top end"
        :offset="14"
        transition="fade-transition"
      >
        <template #activator="{ props }">
          <v-btn
            v-bind="props"
            :disabled="!canControlPlayback"
            icon
            variant="plain"
            class="output-trigger"
            :color="outputButtonColor"
            data-testid="strip-output-menu"
          >
            <v-icon :icon="outputButtonIcon" size="20" />
          </v-btn>
        </template>

        <v-card class="output-popout" color="surface">
          <div class="output-popout__label">Playback output</div>
          <div class="output-popout__actions">
            <v-btn
              block
              :variant="playback.output_target === 'i2s' ? 'flat' : 'outlined'"
              :color="playback.output_target === 'i2s' ? 'primary' : undefined"
              data-testid="strip-output-i2s"
              @click="setOutputTarget('i2s')"
            >
              I2S
            </v-btn>
            <v-btn
              block
              :variant="playback.output_target === 'bluetooth' ? 'flat' : 'outlined'"
              :color="playback.output_target === 'bluetooth' ? 'primary' : undefined"
              data-testid="strip-output-bluetooth"
              @click="setOutputTarget('bluetooth')"
            >
              Bluetooth
            </v-btn>
          </div>
        </v-card>
      </v-menu>

      <template>
        <v-icon icon="mdi-volume-high" color="grey-lighten-1" size="20" />
        <v-slider
          :disabled="!canControlPlayback"
          v-model="volumeDraft"
          :max="100"
          color="white"
          track-color="grey-darken-2"
          class="volume-slider"
          hide-details
          thumb-size="12"
          @start="isAdjustingVolume = true"
          @end="commitVolume"
        />
      </template>
    </div>
  </v-footer>
</template>

<style scoped>
.playback-strip {
  background-color: #000000 !important;
  color: white;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  height: 108px;
  border-top: 1px solid rgba(255,255,255,0.1);
  box-sizing: border-box;
}

.strip-left {
  display: flex;
  flex: 1;
  align-items: center;
  min-width: 0;
  gap: 14px;
}

.strip-artwork {
  border-radius: 4px;
  overflow: hidden;
  width: 56px;
  height: 56px;
  flex-shrink: 0;
}

.placeholder-art {
  width: 56px;
  height: 56px;
  border-radius: 4px;
  background-color: #282828;
  flex-shrink: 0;
}

.strip-meta {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  overflow: hidden;
}

.strip-title {
  font-size: 0.875rem;
  font-weight: 500;
  color: white;
}

.strip-artist {
  font-size: 0.6875rem;
  color: #b3b3b3;
}

.strip-line {
  position: relative;
  display: block;
  width: 100%;
  min-width: 0;
  overflow: hidden;
  white-space: nowrap;
}

.strip-line__label {
  display: inline-block;
  white-space: nowrap;
}

.strip-line:not(.strip-line--marquee) .strip-line__label {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
}

.strip-line__track {
  display: inline-flex;
  align-items: center;
  gap: 1.5rem;
  width: max-content;
}

.strip-line--marquee .strip-line__track {
  animation: strip-marquee var(--marquee-duration, 10s) linear infinite;
}

@keyframes strip-marquee {
  from {
    transform: translateX(0);
  }
  to {
    transform: translateX(calc(-50% - 0.75rem));
  }
}

.strip-center {
  display: flex;
  flex: 2;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-width: 0;
  max-width: 722px;
}

.strip-actions {
  display: grid;
  grid-template-columns: auto auto auto;
  align-items: center;
  justify-content: center;
  column-gap: 14px;
  margin-bottom: 4px;
}

.transport-group {
  display: flex;
  align-items: center;
  gap: 12px;
}

.action-btn {
  width: 40px !important;
  height: 40px !important;
  min-width: 40px !important;
  min-height: 40px !important;
  flex: 0 0 40px;
}

.play-btn {
  background-color: white !important;
  width: 56px !important;
  height: 56px !important;
  min-width: 56px !important;
  min-height: 56px !important;
  flex: 0 0 56px;
  border-radius: 50% !important;
  padding: 0 !important;
}

.play-btn:hover {
  transform: scale(1.05);
}

.strip-progress {
  display: flex;
  align-items: center;
  min-width: 0;
  width: 100%;
  gap: 8px;
}

.time-text {
  font-size: 0.6875rem;
  color: #a7a7a7;
  min-width: 40px;
  text-align: center;
}

.progress-slider {
  flex: 1;
  min-width: 0;
}

:deep(.progress-slider .v-slider-track__thumb) {
  opacity: 0;
  transition: opacity 0.2s;
}

:deep(.progress-slider:hover .v-slider-track__thumb) {
  opacity: 1;
}

.strip-right {
  display: flex;
  flex: 1;
  justify-content: flex-end;
  align-items: center;
  gap: 8px;
  max-width: 250px;
}

.volume-slider {
  max-width: 100px;
}

.volume-trigger {
  width: 40px !important;
  height: 40px !important;
  min-width: 40px !important;
  min-height: 40px !important;
  flex: 0 0 40px;
}

.output-trigger {
  width: 40px !important;
  height: 40px !important;
  min-width: 40px !important;
  min-height: 40px !important;
  flex: 0 0 40px;
}

.volume-popout {
  width: min(220px, calc(100vw - 24px));
  padding: 0.9rem 1rem 0.45rem;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.04), rgba(255, 255, 255, 0.015)),
    #171717 !important;
}

.volume-popout__label {
  margin-bottom: 0.45rem;
  color: rgba(255, 255, 255, 0.62);
  font-size: 0.68rem;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

.output-popout {
  width: min(220px, calc(100vw - 24px));
  padding: 0.9rem 1rem 1rem;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.04), rgba(255, 255, 255, 0.015)),
    #171717 !important;
}

.output-popout__label {
  margin-bottom: 0.7rem;
  color: rgba(255, 255, 255, 0.62);
  font-size: 0.68rem;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

.output-popout__actions {
  display: grid;
  gap: 0.6rem;
}

.volume-slider--popout {
  max-width: none;
}

:deep(.volume-slider .v-slider-track__thumb) {
  opacity: 0;
  transition: opacity 0.2s;
}

:deep(.volume-slider:hover .v-slider-track__thumb) {
  opacity: 1;
}

@media (max-width: 720px) {
  .playback-strip {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    grid-template-areas:
      "left controls"
      "progress progress";
    align-items: center;
    justify-content: stretch;
    padding: 8px 10px 14px;
    min-height: 114px;
    height: auto;
    column-gap: 10px;
    row-gap: 6px;
  }

  .strip-left {
    grid-area: left;
    flex: none;
    width: auto;
    min-width: 0;
    gap: 8px;
  }

  .strip-artwork,
  .placeholder-art {
    width: 40px;
    height: 40px;
  }

  .strip-meta {
    justify-content: center;
    gap: 1px;
  }

  .strip-title {
    font-size: 0.72rem;
    line-height: 1.05;
  }

  .strip-artist {
    display: block;
    font-size: 0.54rem;
    line-height: 1.05;
  }

  .strip-center {
    display: contents;
  }

  .strip-actions {
    grid-area: controls;
    width: auto;
    grid-template-columns: auto auto auto;
    column-gap: 4px;
    margin-bottom: 0;
    justify-self: end;
    align-self: center;
  }

  .transport-group {
    gap: 4px;
  }

  .transport-group--left {
    justify-self: end;
  }

  .transport-group--right {
    justify-self: start;
  }

  .action-btn {
    width: 32px !important;
    height: 32px !important;
    min-width: 32px !important;
    min-height: 32px !important;
    flex-basis: 32px;
  }

  .play-btn {
    width: 34px !important;
    height: 34px !important;
    min-width: 34px !important;
    min-height: 34px !important;
    flex-basis: 34px;
  }

  .time-text {
    min-width: 28px;
    font-size: 0.58rem;
  }

  .strip-progress {
    grid-area: progress;
    width: 100%;
    gap: 4px;
    padding-inline: 0;
  }

  .volume-trigger {
    width: 32px !important;
    height: 32px !important;
    min-width: 32px !important;
    min-height: 32px !important;
    flex-basis: 32px;
  }
}
</style>
