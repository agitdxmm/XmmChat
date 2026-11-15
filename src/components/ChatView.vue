<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { state } from "../core/store";
import Bubble from "./Bubble.vue";
import Composer from "./Composer.vue";

const emit = defineEmits<{
  (e: "call", video: boolean): void;
  (e: "assist"): void;
  (e: "terminal"): void;
  (e: "settings"): void;
}>();

const scroller = ref<HTMLDivElement | null>(null);

function toBottom(smooth = false) {
  const el = scroller.value;
  if (!el) return;
  el.scrollTo({ top: el.scrollHeight, behavior: smooth ? "smooth" : "auto" });
}

watch(
  () => state.messages.length,
  () => nextTick(() => toBottom(true))
);

watch(
  () => state.typing,
  (v) => {
    if (v) nextTick(() => toBottom(true));
  }
);

const initial = () => (state.peer.name || "?").slice(0, 1);
</script>

<template>
  <div class="chat">
    <header class="chead">
      <div class="who">
        <div class="av">{{ initial() }}</div>
        <div class="txt">
          <div class="nm">
            {{ state.peer.name || "对方" }}
            <span class="plat">{{ state.peer.platform }}</span>
          </div>
          <div class="st">
            <template v-if="state.typing"><i class="d"></i>正在输入…</template>
            <template v-else-if="state.granted.length">
              <i class="d g"></i>已授权：{{ state.granted.map((s) => (s === "screen" ? "屏幕" : "指令")).join(" · ") }}
            </template>
            <template v-else><i class="d"></i>点对点直连 · 端到端</template>
          </div>
        </div>
      </div>

      <div class="acts">
        <button class="act" title="语音通话" @click="emit('call', false)">
          <svg viewBox="0 0 20 20">
            <path d="M4.2 3.4h3l1.3 3.4-1.9 1.3a10 10 0 0 0 5.3 5.3l1.3-1.9 3.4 1.3v3a1.6 1.6 0 0 1-1.7 1.6C9 16.6 3.4 11 2.6 5.1A1.6 1.6 0 0 1 4.2 3.4Z" stroke="currentColor" fill="none" stroke-width="1.4" stroke-linejoin="round" />
          </svg>
        </button>
        <button class="act" title="视频通话" @click="emit('call', true)">
          <svg viewBox="0 0 20 20">
            <rect x="2.4" y="5.4" width="10.4" height="9.2" rx="2.2" stroke="currentColor" fill="none" stroke-width="1.4" />
            <path d="M12.8 9.2 17.4 6.6v6.8l-4.6-2.6" stroke="currentColor" fill="none" stroke-width="1.4" stroke-linejoin="round" />
          </svg>
        </button>
        <button class="act" title="协助与远程" @click="emit('assist')">
          <svg viewBox="0 0 20 20">
            <rect x="2.6" y="3.6" width="14.8" height="10" rx="1.8" stroke="currentColor" fill="none" stroke-width="1.4" />
            <path d="M7 17h6M10 13.6V17" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" />
            <path d="M7.4 8.6h5.2M10 6.2v4.8" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
          </svg>
        </button>
        <button class="act" title="远程终端" @click="emit('terminal')">
          <svg viewBox="0 0 20 20">
            <rect x="2.4" y="3.6" width="15.2" height="12.8" rx="2" stroke="currentColor" fill="none" stroke-width="1.4" />
            <path d="M6 8.2 8.4 10.4 6 12.6M10.6 13h3.4" stroke="currentColor" stroke-width="1.3" fill="none" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>
        <div class="sep"></div>
        <button class="act" title="聊天设置" @click="emit('settings')">
          <svg viewBox="0 0 20 20">
            <circle cx="10" cy="10" r="2.4" stroke="currentColor" fill="none" stroke-width="1.4" />
            <path d="M10 2.6v1.6M10 15.8v1.6M2.6 10h1.6M15.8 10h1.6M4.9 4.9l1.1 1.1M14 14l1.1 1.1M15.1 4.9 14 6M6 14l-1.1 1.1" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" />
          </svg>
        </button>
      </div>
    </header>

    <div ref="scroller" class="stream">
      <div v-if="!state.messages.length" class="empty">
        <div class="ehint">连接已建立，说点什么吧</div>
        <div class="esub">可以发文字、拖文件进来、录一段语音，或者点右上角发起通话</div>
      </div>

      <Bubble v-for="m in state.messages" :key="m.id" :m="m" />

      <div v-if="state.typing" class="typing">
        <span></span><span></span><span></span>
      </div>
    </div>

    <Composer />
  </div>
</template>

<style scoped>
.chat {
  position: relative;
  z-index: 10;
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.chead {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 10px 16px;
  background: rgba(12, 15, 23, 0.34);
  backdrop-filter: blur(30px) saturate(150%);
  -webkit-backdrop-filter: blur(30px) saturate(150%);
  border-bottom: 1px solid var(--line-1);
}
html[data-theme="light"] .chead {
  background: rgba(250, 251, 254, 0.5);
}

.who {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}
.av {
  width: 36px;
  height: 36px;
  flex: 0 0 36px;
  display: grid;
  place-items: center;
  border-radius: 12px;
  font-weight: 700;
  color: #0d1018;
  background: linear-gradient(135deg, var(--hue-c), var(--hue-a));
  box-shadow: 0 4px 14px rgba(94, 234, 212, 0.28);
}
.txt {
  min-width: 0;
}
.nm {
  display: flex;
  align-items: baseline;
  gap: 7px;
  font-size: 13.5px;
  font-weight: 620;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.plat {
  font-size: 9.5px;
  font-weight: 400;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--ink-4);
}
.st {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 10.5px;
  color: var(--ink-3);
}
.d {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: #4ade80;
  box-shadow: 0 0 7px rgba(74, 222, 128, 0.7);
}
.d.g {
  background: var(--hue-c);
  box-shadow: 0 0 7px rgba(94, 234, 212, 0.8);
}

.acts {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 2px;
}
.act {
  width: 34px;
  height: 34px;
  display: grid;
  place-items: center;
  border-radius: 10px;
  color: var(--ink-2);
  transition: background 0.16s, color 0.16s, transform 0.16s var(--ease-spring);
}
.act svg {
  width: 18px;
  height: 18px;
}
.act:hover {
  background: rgba(255, 255, 255, 0.09);
  color: var(--ink-1);
  transform: translateY(-1px);
}
.act:active {
  transform: scale(0.92);
}
.sep {
  width: 1px;
  height: 18px;
  margin: 0 5px;
  background: var(--line-1);
}

.stream {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 14px 0 10px;
  scroll-behavior: smooth;
}

.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  height: 100%;
  min-height: 220px;
  text-align: center;
  animation: fade-up 0.6s var(--ease-out) both;
}
.ehint {
  font-size: 13px;
  color: var(--ink-2);
}
.esub {
  font-size: 11.5px;
  color: var(--ink-4);
  max-width: 360px;
  line-height: 1.6;
}

.typing {
  display: flex;
  gap: 4px;
  padding: 6px 22px 10px;
}
.typing span {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--ink-3);
  animation: dots 1.3s infinite;
}
.typing span:nth-child(2) {
  animation-delay: 0.16s;
}
.typing span:nth-child(3) {
  animation-delay: 0.32s;
}
</style>
