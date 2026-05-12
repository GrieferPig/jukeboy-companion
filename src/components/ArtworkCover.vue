<script setup lang="ts">
import { computed, ref, watch } from "vue";

const props = withDefaults(
  defineProps<{
    title: string;
    subtitle?: string;
    src?: string | null;
    height?: number | string;
    radius?: number | string;
  }>(),
  {
    subtitle: "",
    src: null,
    height: 320,
    radius: 24,
  },
);

const imageFailed = ref(false);
const imageLoaded = ref(false);

const resolvedHeight = computed(() => {
  return typeof props.height === "number" ? `${props.height}px` : props.height;
});

const resolvedRadius = computed(() => {
  return typeof props.radius === "number" ? `${props.radius}px` : props.radius;
});

const showImage = computed(() => Boolean(props.src) && !imageFailed.value);

watch(
  () => props.src,
  () => {
    imageFailed.value = false;
    imageLoaded.value = false;
  },
);

function handleLoad(): void {
  imageLoaded.value = true;
}

function handleError(): void {
  imageFailed.value = true;
  imageLoaded.value = false;
}
</script>

<template>
  <div class="artwork-cover" :style="{ height: resolvedHeight, borderRadius: resolvedRadius }">
    <div
      class="artwork-cover__fallback"
      :class="{ 'artwork-cover__fallback--hidden': showImage && imageLoaded }"
    />
    <img
      v-if="showImage"
      :src="props.src ?? undefined"
      :alt="props.title || 'Album artwork'"
      class="artwork-cover__image"
      :class="{ 'artwork-cover__image--ready': imageLoaded }"
      @load="handleLoad"
      @error="handleError"
    >
  </div>
</template>

<style scoped>
.artwork-cover {
  position: relative;
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: #6b7280;
}

.artwork-cover__fallback {
  position: absolute;
  inset: 0;
  background: linear-gradient(180deg, #7a7f87 0%, #61666f 100%);
  transition: opacity 140ms ease;
}

.artwork-cover__fallback--hidden {
  opacity: 0;
}

.artwork-cover__image {
  position: relative;
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
  opacity: 0;
  transition: opacity 140ms ease;
}

.artwork-cover__image--ready {
  opacity: 1;
}
</style>