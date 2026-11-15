<script setup lang="ts">
import { onMounted, ref } from "vue";
import { isAndroid, isTauri, winClose, winIsMaxed, winMinimize, winToggleMax } from "../core/bridge";
import { state } from "../core/store";

const maxed = ref(false);
// 安卓上 isTauri 也是 true，但手机不需要窗口标题栏
const isDesktop = ref(isTauri && !isAndroid);

async function refreshMax() {
  maxed.value = await winIsMaxed();
}

async function onMax() {
  await winToggleMax();
  setTimeout(refreshMax, 80);
}

onMounted(() => {
  refreshMax();
  window.addEventListener("resize", refreshMax);
});
</script>

<template>
  <header v-if="isDesktop" class="titlebar" data-tauri-drag-region>
    <div class="brand" data-tauri-drag-region>
      <svg class="mark" viewBox="0 0 32 32" aria-hidden="true">
        <defs>
          <linearGradient id="lg" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0%" stop-color="#a78bfa" />
            <stop offset="55%" stop-color="#f0abfc" />
            <stop offset="100%" stop-color="#5eead4" />
          </linearGradient>
        </defs>
        <path
          d="M16 3.6c6.85 0 12.4 5.1 12.4 11.4 0 6.3-5.55 11.4-12.4 11.4-1.62 0-3.17-.28-4.58-.8L5.4 28.4l1.2-5.3A11.1 11.1 0 0 1 3.6 15C3.6 8.7 9.15 3.6 16 3.6Z"
          fill="url(#lg)"
          opacity="0.92"
        />
        <circle cx="11.6" cy="14.2" r="1.5" fill="#0d1018" opacity="0.85" />
        <circle cx="20.4" cy="14.2" r="1.5" fill="#0d1018" opacity="0.85" />
        <path
          d="M12.4 18.6c1.1.95 2.4 1.42 3.6 1.42s2.5-.47 3.6-1.42"
          stroke="#0d1018"
          stroke-width="1.5"
          stroke-linecap="round"
          fill="none"
          opacity="0.75"
        />
      </svg>
      <span class="name">咪的天</span>
      <span class="latin">XmmChat</span>
    </div>

    <div class="drag" data-tauri-drag-region @dblclick="onMax"></div>

    <div class="status" data-tauri-drag-region>
      <span v-if="state.link.state === 'online'" class="dot online"></span>
      <span v-else class="dot"></span>
      <span class="st">{{ state.link.state === "online" ? state.peer.name || "已连接" : "未连接" }}</span>
    </div>

    <div class="winbtns">
      <button class="wb" title="最小化" @click="winMinimize">
        <svg viewBox="0 0 10 10"><path d="M2 5h6" stroke="currentColor" stroke-width="1.1" /></svg>
      </button>
      <button class="wb" :title="maxed ? '还原' : '最大化'" @click="onMax">
        <svg v-if="!maxed" viewBox="0 0 10 10">
          <rect x="2.2" y="2.2" width="5.6" height="5.6" stroke="currentColor" stroke-width="1.1" fill="none" />
        </svg>
        <svg v-else viewBox="0 0 10 10">
          <rect x="2" y="3" width="5" height="5" stroke="currentColor" stroke-width="1.1" fill="none" />
          <path d="M3.5 3V2h5v5h-1" stroke="currentColor" stroke-width="1.1" fill="none" />
        </svg>
      </button>
      <button class="wb close" title="关闭" @click="winClose">
        <svg viewBox="0 0 10 10"><path d="M2.4 2.4l5.2 5.2M7.6 2.4L2.4 7.6" stroke="currentColor" stroke-width="1.1" /></svg>
      </button>
    </div>
  </header>
</template>

<style scoped>
.titlebar {
  position: relative;
  z-index: 40;
  display: flex;
  align-items: center;
  height: var(--titlebar-h);
  flex: 0 0 var(--titlebar-h);
  padding-left: 14px;
  /* 顶部一层极淡的玻璃，让标题栏和内容有分隔但不割裂 */
  background: linear-gradient(to bottom, rgba(255, 255, 255, 0.05), rgba(255, 255, 255, 0.015));
  backdrop-filter: blur(22px) saturate(150%);
  -webkit-backdrop-filter: blur(22px) saturate(150%);
  border-bottom: 1px solid var(--line-1);
}

.brand {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 0 0 auto;
}

.mark {
  width: 22px;
  height: 22px;
  filter: drop-shadow(0 2px 6px rgba(167, 139, 250, 0.45));
  transition: transform 0.5s var(--ease-spring);
}
.brand:hover .mark {
  transform: rotate(-8deg) scale(1.08);
}

.name {
  font-size: 13px;
  font-weight: 650;
  letter-spacing: 0.02em;
}

.latin {
  font-size: 10px;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--ink-3);
  padding-top: 1px;
}

.drag {
  flex: 1 1 auto;
  height: 100%;
}

.status {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-right: 10px;
  font-size: 11.5px;
  color: var(--ink-2);
  max-width: 220px;
}

.dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--ink-4);
  transition: background 0.3s, box-shadow 0.3s;
}
.dot.online {
  background: #4ade80;
  box-shadow: 0 0 8px rgba(74, 222, 128, 0.75);
  animation: breathe 2.6s var(--ease-smooth) infinite;
}

.st {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.winbtns {
  display: flex;
  height: 100%;
}

.wb {
  width: 46px;
  height: 100%;
  display: grid;
  place-items: center;
  color: var(--ink-2);
  transition: background 0.15s, color 0.15s;
}
.wb svg {
  width: 10px;
  height: 10px;
}
.wb:hover {
  background: rgba(255, 255, 255, 0.09);
  color: var(--ink-1);
}
.wb.close:hover {
  background: #c42b1c;
  color: #fff;
}
.wb:active {
  background: rgba(255, 255, 255, 0.05);
}
html[data-theme="light"] .wb:hover {
  background: rgba(16, 20, 32, 0.07);
}
</style>
