<script setup lang="ts">
import { computed } from "vue";

import { buildArtworkDataUrl } from "../utils/artwork";

const props = withDefaults(
  defineProps<{
    title: string;
    subtitle?: string;
    seed?: string;
    height?: number | string;
  }>(),
  {
    subtitle: "",
    seed: "",
    height: 320,
  },
);

const artworkUrl = computed(() => {
  return buildArtworkDataUrl(props.seed || props.title, props.title, props.subtitle);
});

const resolvedHeight = computed(() => {
  return typeof props.height === "number" ? `${props.height}px` : props.height;
});
</script>

<template>
  <v-sheet class="artwork-cover" color="surface-variant">
    <v-img :src="artworkUrl" cover class="artwork-cover__image" :style="{ height: resolvedHeight }" />
  </v-sheet>
</template>

<style scoped>
.artwork-cover {
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.artwork-cover__image {
  width: 100%;
}
</style>