<script setup lang="ts">
import { computed, ref } from "vue";
import { fmtClock, fmtDuration, fmtSize, type Message } from "../core/store";

const props = defineProps<{ m: Message }>();

const mine = computed(() => props.m.from === "me");
const isSys = computed(() => props.m.from === "sys");

const zoom = ref(false);

function openFile(url?: string, name?: string) {
  if (!url) return;
  const a = document.createElement("a");
  a.href = url;
  a.download = name || "download";
  a.click();
}

const fileIcon = computed(() => {
  const n = (props.m.file?.name || "").toLowerCase();
  const mime = props.m.file?.mime || "";
  if (mime.startsWith("image/") || /\.(png|jpe?g|gif|webp|bmp|avif)$/.test(n)) return "image";
  if (mime.startsWith("video/") || /\.(mp4|webm|mov|mkv)$/.test(n)) return "video";
  if (mime.startsWith("audio/") || /\.(mp3|wav|ogg|m4a|flac)$/.test(n)) return "audio";
  if (/\.(zip|7z|rar|tar|gz|xz)$/.test(n)) return "archive";
  if (/\.(pdf|docx?|xlsx?|pptx?|txt|md)$/.test(n)) return "doc";
  if (/\.(exe|msi|apk|dmg)$/.test(n)) return "app";
  return "file";
});

const iconPath: Record<string, string> = {
  image: "M3 15l4.5-5 3.5 4 2.5-3L21 18H3z",
  video: "M3 6h11v12H3zM14 10l5-3v10l-5-3z",
  audio: "M9 18V7l8-2v11M9 18a2.5 2.5 0 1 1-5 0 2.5 2.5 0 0 1 5 0ZM17 16a2.5 2.5 0 1 1-5 0 2.5 2.5 0 0 1 5 0Z",
  archive: "M4 6h16v13H4zM4 6l8-3 8 3M10 10h4v5h-4z",
  doc: "M6 3h8l4 4v14H6zM14 3v4h4M9 12h6M9 16h6",
  app: "M4 5h16v14H4zM4 9h16M8 13h3",
  file: "M6 3h8l4 4v14H6zM14 3v4h4",
};

/* 语音播放 */
const audio = ref<HTMLAudioElement | null>(null);
const playing = ref(false);
const progress = ref(0);

function togglePlay() {
  const src = props.m.voice?.url;
  if (!src) return;
  if (!audio.value) {
    audio.value = new Audio(src);
    audio.value.ontimeupdate = () => {
      const d = audio.value?.duration || props.m.voice?.duration || 1;
      progress.value = (audio.value?.currentTime || 0) / d;
    };
    audio.value.onended = () => {
      playing.value = false;
      progress.value = 0;
    };
  }
  if (playing.value) {
    audio.value.pause();
    playing.value = false;
  } else {
    audio.value.play().catch(() => {});
    playing.value = true;
  }
}

const voiceBars = computed(() => {
  // 用消息 id 生成稳定的「波形」，避免每次重绘乱跳
  const seed = [...(props.m.id || "x")].reduce((a, c) => a + c.charCodeAt(0), 0);
  return Array.from({ length: 26 }, (_, i) => {
    const v = Math.abs(Math.sin(seed * 0.13 + i * 0.9) * Math.cos(i * 0.37 + seed));
    return 26 + Math.round(v * 62);
  });
});
</script>

<template>
  <!-- 系统提示 -->
  <div v-if="isSys" class="sysrow">
    <span class="systext">{{ m.text }}</span>
  </div>

  <!-- 消息 -->
  <div v-else class="row" :class="{ mine }">
    <div class="stack">
      <div
        class="bubble glass glass-edge"
        :class="[m.kind, { mine, pending: m.pending }]"
      >
        <!-- 文本 -->
        <p v-if="m.kind === 'text'" class="body">{{ m.text }}</p>

        <!-- 图片 -->
        <template v-else-if="m.kind === 'image'">
          <div v-if="m.file?.url" class="media">
            <img :src="m.file.url" :alt="m.file.name" @click="zoom = true" />
          </div>
          <div v-else class="filecard">
            <span class="fsize">{{ Math.round((m.file?.progress || 0) * 100) }}%</span>
          </div>
        </template>

        <!-- 视频 -->
        <template v-else-if="m.kind === 'video'">
          <video v-if="m.file?.url" :src="m.file.url" controls class="vid"></video>
          <div v-else class="filecard">
            <span class="fsize">{{ Math.round((m.file?.progress || 0) * 100) }}%</span>
          </div>
        </template>

        <!-- 语音 -->
        <div v-else-if="m.kind === 'voice'" class="voice" @click="togglePlay">
          <button class="vplay" :class="{ on: playing }">
            <svg v-if="!playing" viewBox="0 0 18 18">
              <path d="M6 4.2 13.4 9 6 13.8z" fill="currentColor" />
            </svg>
            <svg v-else viewBox="0 0 18 18">
              <rect x="5.2" y="4" width="2.6" height="10" rx="1" fill="currentColor" />
              <rect x="10.2" y="4" width="2.6" height="10" rx="1" fill="currentColor" />
            </svg>
          </button>
          <div class="wave">
            <i
              v-for="(h, i) in voiceBars"
              :key="i"
              :style="{ height: `${h}%` }"
              :class="{ lit: progress * voiceBars.length > i }"
            ></i>
          </div>
          <span class="dur mono">{{ fmtDuration(m.voice?.duration || 0) }}</span>
        </div>

        <!-- 文件 -->
        <div v-else-if="m.kind === 'file'" class="filecard">
          <div class="ficon">
            <svg viewBox="0 0 24 24">
              <path :d="iconPath[fileIcon]" stroke="currentColor" fill="none" stroke-width="1.5" stroke-linejoin="round" />
            </svg>
          </div>
          <div class="finfo">
            <div class="fname">{{ m.file?.name || "文件" }}</div>
            <div class="fmeta mono">
              {{ fmtSize(m.file?.size || 0) }}
              <template v-if="m.file?.status === 'done'"> · 已保存</template>
              <template v-else> · {{ Math.round((m.file?.progress || 0) * 100) }}%</template>
            </div>
            <div v-if="m.file && m.file.status !== 'done'" class="bar">
              <i :style="{ width: `${(m.file.progress || 0) * 100}%` }"></i>
            </div>
          </div>
          <button v-if="m.file?.url" class="fdl" title="保存" @click="openFile(m.file.url, m.file.name)">
            <svg viewBox="0 0 20 20">
              <path d="M10 3.5v9M6.4 9.2 10 12.8l3.6-3.6M4 16h12" stroke="currentColor" fill="none" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
          </button>
        </div>

        <!-- 通话记录 -->
        <div v-else-if="m.kind === 'call'" class="callcard">
          <svg viewBox="0 0 20 20" class="cicon">
            <path
              d="M4.2 3.4h3l1.3 3.4-1.9 1.3a10 10 0 0 0 5.3 5.3l1.3-1.9 3.4 1.3v3a1.6 1.6 0 0 1-1.7 1.6C9 16.6 3.4 11 2.6 5.1A1.6 1.6 0 0 1 4.2 3.4Z"
              stroke="currentColor"
              fill="none"
              stroke-width="1.4"
              stroke-linejoin="round"
            />
          </svg>
          <div>
            <div class="ctitle">
              {{ m.call?.video ? "视频通话" : "语音通话" }}
              <span v-if="m.call?.missed" class="missed">未接</span>
            </div>
            <div class="csub mono">{{ fmtDuration(m.call?.seconds || 0) }}</div>
          </div>
        </div>

        <!-- 远程终端 -->
        <div v-else-if="m.kind === 'exec'" class="term">
          <div class="termhead">
            <span class="tdot r"></span><span class="tdot y"></span><span class="tdot g"></span>
            <span class="ttitle mono">{{ m.exec?.cmd }}</span>
          </div>
          <pre v-if="m.pending" class="tout">执行中…</pre>
          <pre v-else-if="m.exec?.denied" class="tout denied">{{ m.exec?.stderr || "对端未授权执行" }}</pre>
          <pre v-else class="tout">{{ m.exec?.stdout || m.exec?.stderr || "(无输出)" }}</pre>
          <div v-if="!m.pending && !m.exec?.denied" class="termfoot mono">
            exit {{ m.exec?.code }}
          </div>
        </div>

        <!-- 远程画面 -->
        <div v-else-if="m.kind === 'screen'" class="screenbox">
          <img v-if="m.screen?.url" :src="m.screen.url" alt="远程屏幕" @click="zoom = true" />
        </div>
      </div>

      <div class="meta" :class="{ mine }">
        <span class="t mono">{{ fmtClock(m.ts) }}</span>
        <span v-if="mine && m.pending" class="tick" title="发送中">
          <svg viewBox="0 0 14 14"><circle cx="7" cy="7" r="5" stroke="currentColor" fill="none" stroke-width="1.3" stroke-dasharray="2 2" /></svg>
        </span>
      </div>
    </div>

    <!-- 图片放大 -->
    <transition name="zoomfade">
      <div v-if="zoom" class="zoomlay" @click="zoom = false">
        <img v-if="m.kind === 'image'" :src="m.file?.url" alt="" />
        <img v-else-if="m.kind === 'screen'" :src="m.screen?.url" alt="" />
      </div>
    </transition>
  </div>
</template>

<style scoped>
.sysrow {
  display: flex;
  justify-content: center;
  padding: 6px 0;
  animation: fade-up 0.4s var(--ease-out) both;
}
.systext {
  max-width: 74%;
  padding: 4px 12px;
  font-size: 11px;
  line-height: 1.6;
  text-align: center;
  color: var(--ink-3);
  border-radius: var(--r-pill);
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid var(--line-1);
}

.row {
  display: flex;
  padding: 3px 18px;
  animation: msg-in 0.42s var(--ease-out) both;
}
.row.mine {
  justify-content: flex-end;
}

.stack {
  max-width: min(560px, 74%);
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.bubble {
  position: relative;
  padding: 9px 13px;
  border-radius: var(--r-md);
  font-size: 13.5px;
  line-height: 1.62;
  word-break: break-word;
  overflow-wrap: anywhere;
  transition: transform 0.22s var(--ease-spring), border-color 0.22s, box-shadow 0.22s;
}
.bubble:hover {
  transform: translateY(-1px);
}
.bubble.mine {
  border-bottom-right-radius: 6px;
  background: linear-gradient(
    145deg,
    rgba(167, 139, 250, 0.22),
    rgba(240, 171, 252, 0.13)
  );
  border-color: rgba(196, 181, 253, 0.30);
}
html[data-theme="light"] .bubble.mine {
  background: linear-gradient(145deg, rgba(196, 181, 253, 0.55), rgba(240, 171, 252, 0.38));
  border-color: rgba(139, 92, 246, 0.28);
}
.bubble:not(.mine) {
  border-bottom-left-radius: 6px;
}

/* 不同内容形态的留白 */
.bubble.image,
.bubble.video,
.bubble.screen {
  padding: 5px;
}
.bubble.file,
.bubble.voice,
.bubble.exec,
.bubble.call {
  padding: 8px;
}

.body {
  white-space: pre-wrap;
  user-select: text;
}

.media img,
.screenbox img {
  display: block;
  max-width: 100%;
  max-height: 340px;
  border-radius: 13px;
  cursor: zoom-in;
  transition: filter 0.2s;
}
.media img:hover,
.screenbox img:hover {
  filter: brightness(1.06);
}

.vid {
  display: block;
  max-width: 100%;
  max-height: 340px;
  border-radius: 13px;
}

/* ---- 语音 ---- */
.voice {
  display: flex;
  align-items: center;
  gap: 9px;
  min-width: 190px;
  cursor: pointer;
}
.vplay {
  width: 30px;
  height: 30px;
  flex: 0 0 30px;
  display: grid;
  place-items: center;
  border-radius: 50%;
  color: #12101c;
  background: linear-gradient(135deg, var(--hue-c), var(--hue-a));
  transition: transform 0.2s var(--ease-spring);
}
.vplay svg {
  width: 16px;
  height: 16px;
}
.vplay.on {
  animation: pulse-ring 1.6s infinite;
}
.voice:hover .vplay {
  transform: scale(1.08);
}

.wave {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 2px;
  height: 26px;
}
.wave i {
  flex: 1;
  min-width: 2px;
  border-radius: 2px;
  background: var(--ink-4);
  transition: background 0.2s;
}
.wave i.lit {
  background: var(--hue-a);
}
.dur {
  font-size: 11px;
  color: var(--ink-3);
}

/* ---- 文件 ---- */
.filecard {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 220px;
}
.ficon {
  width: 38px;
  height: 38px;
  flex: 0 0 38px;
  display: grid;
  place-items: center;
  border-radius: 11px;
  color: var(--hue-a);
  background: rgba(167, 139, 250, 0.14);
  border: 1px solid rgba(167, 139, 250, 0.22);
}
.ficon svg {
  width: 20px;
  height: 20px;
}
.finfo {
  min-width: 0;
  flex: 1;
}
.fname {
  font-size: 12.5px;
  font-weight: 550;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 260px;
}
.fmeta {
  font-size: 10.5px;
  color: var(--ink-3);
  margin-top: 1px;
}
.bar {
  height: 3px;
  margin-top: 5px;
  border-radius: 2px;
  background: rgba(255, 255, 255, 0.1);
  overflow: hidden;
}
.bar i {
  display: block;
  height: 100%;
  border-radius: 2px;
  background: linear-gradient(90deg, var(--hue-a), var(--hue-b));
  transition: width 0.2s;
}
.fdl {
  width: 30px;
  height: 30px;
  display: grid;
  place-items: center;
  border-radius: 9px;
  color: var(--ink-2);
  transition: background 0.16s, color 0.16s;
}
.fdl svg {
  width: 17px;
  height: 17px;
}
.fdl:hover {
  background: rgba(255, 255, 255, 0.1);
  color: var(--ink-1);
}
.fsize {
  font-size: 12px;
  color: var(--ink-3);
}

/* ---- 通话 ---- */
.callcard {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 170px;
}
.cicon {
  width: 22px;
  height: 22px;
  color: var(--hue-c);
}
.ctitle {
  font-size: 12.5px;
  font-weight: 550;
}
.csub {
  font-size: 10.5px;
  color: var(--ink-3);
}
.missed {
  margin-left: 6px;
  padding: 0 6px;
  font-size: 10px;
  border-radius: var(--r-pill);
  color: #fda4af;
  background: rgba(244, 63, 94, 0.16);
}

/* ---- 终端 ---- */
.term {
  width: min(460px, 60vw);
  border-radius: 12px;
  overflow: hidden;
  background: rgba(6, 9, 14, 0.82);
  border: 1px solid rgba(255, 255, 255, 0.09);
}
html[data-theme="light"] .term {
  background: rgba(18, 22, 30, 0.94);
}
.termhead {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 6px 10px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.07);
}
.tdot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}
.tdot.r {
  background: #ff5f57;
}
.tdot.y {
  background: #febc2e;
}
.tdot.g {
  background: #28c840;
}
.ttitle {
  margin-left: 6px;
  font-size: 10.5px;
  color: rgba(255, 255, 255, 0.55);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.tout {
  margin: 0;
  padding: 9px 11px;
  max-height: 240px;
  overflow: auto;
  font-family: "JetBrains Mono", ui-monospace, Consolas, monospace;
  font-size: 11.5px;
  line-height: 1.6;
  color: #d5f5e3;
  white-space: pre-wrap;
  word-break: break-all;
  user-select: text;
}
.tout.denied {
  color: #fda4af;
}
.termfoot {
  padding: 4px 11px 7px;
  font-size: 10px;
  color: rgba(255, 255, 255, 0.35);
}

/* ---- 元信息 ---- */
.meta {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 0 4px;
  font-size: 9.5px;
  color: var(--ink-4);
}
.meta.mine {
  justify-content: flex-end;
}
.tick svg {
  width: 11px;
  height: 11px;
  animation: spin-slow 2s linear infinite;
}
@keyframes spin-slow {
  to {
    transform: rotate(360deg);
  }
}

/* ---- 放大 ---- */
.zoomlay {
  position: fixed;
  inset: 0;
  z-index: 200;
  display: grid;
  place-items: center;
  padding: 4vw;
  background: rgba(3, 5, 9, 0.82);
  backdrop-filter: blur(14px);
  cursor: zoom-out;
  animation: fade-up 0.2s var(--ease-out);
}
.zoomlay img {
  max-width: 100%;
  max-height: 100%;
  border-radius: 14px;
  box-shadow: var(--shadow-3);
  animation: pop-in 0.26s var(--ease-spring);
}

.zoomfade-enter-active,
.zoomfade-leave-active {
  transition: opacity 0.2s;
}
.zoomfade-enter-from,
.zoomfade-leave-to {
  opacity: 0;
}
</style>
