<script setup lang="ts">
import { computed } from "vue";
import { state } from "../core/store";

const hasPhoto = computed(() => !!state.settings.wallpaper);

const photoStyle = computed(() => ({
  backgroundImage: `url("${state.settings.wallpaper}")`,
  filter: `blur(${state.settings.wallpaperBlur}px) saturate(115%)`,
  transform: "scale(1.12)",
}));

const veilStyle = computed(() => ({
  opacity: state.settings.wallpaperDim,
}));
</script>

<template>
  <div class="wallpaper" aria-hidden="true">
    <!-- 用户设定的亚克力背景图：放大 + 模糊，往后再压一层暗色面纱 -->
    <div v-if="hasPhoto" class="photo" :style="photoStyle"></div>
    <div v-if="hasPhoto" class="veil" :style="veilStyle"></div>

    <!-- 没有背景图时，用会呼吸的光斑撑住画面 -->
    <template v-else>
      <div class="blob b1"></div>
      <div class="blob b2"></div>
      <div class="blob b3"></div>
      <div class="blob b4"></div>
    </template>

    <div class="grain"></div>
  </div>
</template>

<style scoped>
.wallpaper {
  position: absolute;
  inset: 0;
  z-index: 0;
  overflow: hidden;
  background: radial-gradient(120% 100% at 18% 0%, #171a2b 0%, var(--bg-0) 55%, #05070c 100%);
}
html[data-theme="light"] .wallpaper {
  background: radial-gradient(120% 100% at 18% 0%, #dfe6f7 0%, var(--bg-0) 60%, #e9edf6 100%);
}

.photo {
  position: absolute;
  inset: -6%;
  background-size: cover;
  background-position: center;
  will-change: filter, transform;
}

.veil {
  position: absolute;
  inset: 0;
  background: #05070c;
  transition: opacity 0.4s;
}
html[data-theme="light"] .veil {
  background: #f3f6fc;
}

.blob {
  position: absolute;
  border-radius: 50%;
  filter: blur(70px);
  opacity: 0.42;
  will-change: transform;
}
.b1 {
  width: 56vw;
  height: 56vw;
  left: -14vw;
  top: -18vw;
  background: radial-gradient(circle at 40% 40%, #7c3aed, transparent 68%);
  animation: drift-a 26s var(--ease-smooth) infinite;
}
.b2 {
  width: 48vw;
  height: 48vw;
  right: -12vw;
  top: -8vw;
  background: radial-gradient(circle at 50% 50%, #db2777, transparent 70%);
  opacity: 0.34;
  animation: drift-b 32s var(--ease-smooth) infinite;
}
.b3 {
  width: 44vw;
  height: 44vw;
  left: 22vw;
  bottom: -24vw;
  background: radial-gradient(circle at 50% 50%, #0d9488, transparent 70%);
  opacity: 0.30;
  animation: drift-a 38s var(--ease-smooth) infinite reverse;
}
.b4 {
  width: 30vw;
  height: 30vw;
  right: 12vw;
  bottom: -12vw;
  background: radial-gradient(circle at 50% 50%, #2563eb, transparent 72%);
  opacity: 0.24;
  animation: drift-b 30s var(--ease-smooth) infinite reverse;
}

.grain {
  position: absolute;
  inset: 0;
  opacity: 0.35;
  mix-blend-mode: overlay;
  pointer-events: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='160' height='160'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.8' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='160' height='160' filter='url(%23n)' opacity='0.5'/%3E%3C/svg%3E");
}
</style>
