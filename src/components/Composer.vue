<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref } from "vue";
import { sendFile, sendText, sendTyping } from "../core/engine";
import { state, toast } from "../core/store";

const text = ref("");
const box = ref<HTMLTextAreaElement | null>(null);
const showTools = ref(false);
const fileInput = ref<HTMLInputElement | null>(null);

/* ---------------- 输入 ---------------- */

const canSend = computed(() => text.value.trim().length > 0);

function autoGrow() {
  const el = box.value;
  if (!el) return;
  el.style.height = "auto";
  el.style.height = `${Math.min(160, el.scrollHeight)}px`;
}

let typingTimer = 0;
function onInput() {
  autoGrow();
  sendTyping(true);
  window.clearTimeout(typingTimer);
  typingTimer = window.setTimeout(() => sendTyping(false), 1100);
}

function fire() {
  if (!canSend.value) return;
  sendText(text.value);
  text.value = "";
  sendTyping(false);
  nextTick(autoGrow);
}

function onKey(e: KeyboardEvent) {
  if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
    e.preventDefault();
    fire();
  }
}

/* ---------------- 文件 ---------------- */

function pickFile() {
  fileInput.value?.click();
}

async function onFiles(e: Event) {
  const input = e.target as HTMLInputElement;
  const files = Array.from(input.files || []);
  input.value = "";
  for (const f of files) await sendFile(f);
}

/* ---------------- 录音 ---------------- */

const recording = ref(false);
const elapsed = ref(0);
const level = ref(0);
let recorder: MediaRecorder | null = null;
let chunks: BlobPart[] = [];
let startedAt = 0;
let tick = 0;
let audioCtx: AudioContext | null = null;
let analyser: AnalyserNode | null = null;
let meterRaf = 0;

async function startRecord() {
  if (recording.value) return;
  try {
    const stream = await navigator.mediaDevices.getUserMedia({
      audio: { echoCancellation: true, noiseSuppression: true },
    });

    // 实时电平，喂给界面上的那排小柱子
    audioCtx = new AudioContext();
    const src = audioCtx.createMediaStreamSource(stream);
    analyser = audioCtx.createAnalyser();
    analyser.fftSize = 256;
    src.connect(analyser);
    const buf = new Uint8Array(analyser.frequencyBinCount);
    const meter = () => {
      if (!analyser) return;
      analyser.getByteTimeDomainData(buf);
      let peak = 0;
      for (const v of buf) peak = Math.max(peak, Math.abs(v - 128));
      level.value = peak / 128;
      meterRaf = requestAnimationFrame(meter);
    };
    meter();

    const mime = MediaRecorder.isTypeSupported("audio/webm;codecs=opus")
      ? "audio/webm;codecs=opus"
      : "audio/webm";
    recorder = new MediaRecorder(stream, { mimeType: mime });
    chunks = [];
    recorder.ondataavailable = (ev) => {
      if (ev.data.size > 0) chunks.push(ev.data);
    };
    recorder.onstop = async () => {
      const dur = (Date.now() - startedAt) / 1000;
      stream.getTracks().forEach((t) => t.stop());
      cleanupAudio();
      recorder = null;
      recording.value = false;

      if (dur < 0.6) {
        toast("太短了，再说一句", "warn");
        return;
      }
      const blob = new Blob(chunks, { type: "audio/webm" });
      const file = new File([blob], `语音-${Date.now()}.webm`, { type: "audio/webm" });
      await sendFile(file, true, dur);
    };

    startedAt = Date.now();
    elapsed.value = 0;
    recording.value = true;
    recorder.start(250);
    tick = window.setInterval(() => {
      elapsed.value = (Date.now() - startedAt) / 1000;
      if (elapsed.value > 300) stopRecord();
    }, 100);
  } catch (e: any) {
    toast(`拿不到麦克风：${e?.message || e}`, "err");
  }
}

function stopRecord() {
  window.clearInterval(tick);
  if (recorder && recorder.state !== "inactive") recorder.stop();
}

function cleanupAudio() {
  cancelAnimationFrame(meterRaf);
  analyser = null;
  audioCtx?.close().catch(() => {});
  audioCtx = null;
  level.value = 0;
}

function cancelRecord() {
  window.clearInterval(tick);
  if (recorder && recorder.state !== "inactive") {
    recorder.onstop = null;
    recorder.stream.getTracks().forEach((t) => t.stop());
    recorder.stop();
  }
  recorder = null;
  recording.value = false;
  cleanupAudio();
  toast("已取消录音", "info");
}

onBeforeUnmount(() => {
  window.clearInterval(tick);
  cleanupAudio();
});

const mmss = computed(() => {
  const s = Math.floor(elapsed.value);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
});

/* ---------------- 拖放 ---------------- */

const dragging = ref(false);

function onDragOver(e: DragEvent) {
  e.preventDefault();
  dragging.value = true;
}
function onDragLeave() {
  dragging.value = false;
}
async function onDrop(e: DragEvent) {
  e.preventDefault();
  dragging.value = false;
  const files = Array.from(e.dataTransfer?.files || []);
  for (const f of files) await sendFile(f);
}
</script>

<template>
  <div
    class="composer-wrap"
    @dragover="onDragOver"
    @dragleave="onDragLeave"
    @drop="onDrop"
  >
    <div v-if="dragging" class="dropmask">
      <div class="dropinner">
        <svg viewBox="0 0 40 40">
          <path d="M20 27V11M13 18l7-7 7 7" stroke="currentColor" stroke-width="2.2" fill="none" stroke-linecap="round" stroke-linejoin="round" />
          <path d="M8 28v3a3 3 0 0 0 3 3h18a3 3 0 0 0 3-3v-3" stroke="currentColor" stroke-width="2.2" fill="none" stroke-linecap="round" />
        </svg>
        <span>松手就发送</span>
      </div>
    </div>

    <!-- 录音态 -->
    <transition name="rec">
      <div v-if="recording" class="recbar glass glass-edge">
        <button class="rb cancel" title="取消" @click="cancelRecord">
          <svg viewBox="0 0 18 18"><path d="M5 5l8 8M13 5l-8 8" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" /></svg>
        </button>
        <span class="recdot"></span>
        <span class="rectime mono">{{ mmss }}</span>
        <div class="meter">
          <i
            v-for="n in 9"
            :key="n"
            :style="{ height: `${6 + Math.min(1, level * (0.6 + Math.random() * 0.9)) * 26}px` }"
          ></i>
        </div>
        <button class="rb stop" title="发送" @click="stopRecord">
          <svg viewBox="0 0 18 18">
            <path d="M2.5 9h9M8 4.5 13 9l-5 4.5" stroke="currentColor" stroke-width="1.7" fill="none" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>
      </div>
    </transition>

    <!-- 正常输入 -->
    <div v-if="!recording" class="composer glass glass-edge">
      <div class="tools" :class="{ open: showTools }">
        <button class="tool" title="发送文件" @click="pickFile">
          <svg viewBox="0 0 20 20">
            <path d="M11.4 6.2 6.6 11a2.6 2.6 0 0 0 3.7 3.7l5.3-5.3a4.1 4.1 0 0 0-5.8-5.8L4.3 8.9a5.6 5.6 0 0 0 7.9 7.9l4.3-4.3" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>
        <button class="tool" title="录音" @click="startRecord">
          <svg viewBox="0 0 20 20">
            <rect x="7.4" y="2.6" width="5.2" height="9.6" rx="2.6" stroke="currentColor" stroke-width="1.5" fill="none" />
            <path d="M4.6 9.6a5.4 5.4 0 0 0 10.8 0M10 15v2.4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" />
          </svg>
        </button>
        <button class="tool emoji" title="表情" @click="text += ' 🐋'">
          <svg viewBox="0 0 20 20">
            <circle cx="10" cy="10" r="7.2" stroke="currentColor" stroke-width="1.5" fill="none" />
            <circle cx="7.6" cy="8.4" r="1" fill="currentColor" />
            <circle cx="12.4" cy="8.4" r="1" fill="currentColor" />
            <path d="M7.2 11.8a3.4 3.4 0 0 0 5.6 0" stroke="currentColor" stroke-width="1.4" fill="none" stroke-linecap="round" />
          </svg>
        </button>
      </div>

      <textarea
        ref="box"
        v-model="text"
        class="input"
        rows="1"
        placeholder="说点什么…（Enter 发送，Shift+Enter 换行）"
        spellcheck="false"
        @input="onInput"
        @keydown="onKey"
      ></textarea>

      <button class="send" :class="{ hot: canSend }" :disabled="!canSend" title="发送" @click="fire">
        <svg viewBox="0 0 22 22">
          <path d="M3.4 11 18.2 4.2 14.6 18 10.4 12.9 3.4 11Z" stroke="currentColor" stroke-width="1.6" fill="none" stroke-linejoin="round" />
          <path d="M10.4 12.9 18.2 4.2" stroke="currentColor" stroke-width="1.6" fill="none" stroke-linecap="round" />
        </svg>
      </button>
    </div>

    <input
      ref="fileInput"
      type="file"
      multiple
      class="hidden"
      @change="onFiles"
    />
    <p class="foottip">点对点直连 · 不经过服务器 · 端口 {{ state.host.port }}</p>
  </div>
</template>

<style scoped>
.composer-wrap {
  position: relative;
  padding: 0 18px 12px;
  flex: 0 0 auto;
}

.dropmask {
  position: absolute;
  inset: -6px 12px 6px;
  z-index: 30;
  display: grid;
  place-items: center;
  border-radius: var(--r-lg);
  border: 2px dashed rgba(167, 139, 250, 0.65);
  background: rgba(167, 139, 250, 0.12);
  backdrop-filter: blur(10px);
  animation: pop-in 0.2s var(--ease-spring);
}
.dropinner {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  color: var(--hue-a);
  font-size: 13px;
  font-weight: 600;
}
.dropinner svg {
  width: 40px;
  height: 40px;
  animation: breathe 1.4s ease-in-out infinite;
}

.composer {
  display: flex;
  align-items: flex-end;
  gap: 8px;
  padding: 7px 8px 7px 6px;
  border-radius: var(--r-lg);
}

.tools {
  display: flex;
  gap: 2px;
  padding-bottom: 2px;
}

.tool {
  width: 34px;
  height: 34px;
  display: grid;
  place-items: center;
  border-radius: 11px;
  color: var(--ink-3);
  transition: background 0.16s, color 0.16s, transform 0.16s var(--ease-spring);
}
.tool svg {
  width: 18px;
  height: 18px;
}
.tool:hover {
  background: rgba(255, 255, 255, 0.09);
  color: var(--ink-1);
  transform: translateY(-1px);
}
.tool:active {
  transform: scale(0.92);
}

.input {
  flex: 1;
  min-height: 34px;
  max-height: 160px;
  padding: 7px 4px;
  font-size: 13.5px;
  line-height: 1.55;
  color: var(--ink-1);
  background: transparent;
  border: 0;
  outline: none;
  resize: none;
  user-select: text;
}
.input::placeholder {
  color: var(--ink-4);
}

.send {
  width: 36px;
  height: 36px;
  flex: 0 0 36px;
  display: grid;
  place-items: center;
  border-radius: 12px;
  color: var(--ink-3);
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid var(--line-1);
  transition: all 0.2s var(--ease-spring);
}
.send svg {
  width: 19px;
  height: 19px;
}
.send.hot {
  color: #12101c;
  background: linear-gradient(135deg, var(--hue-a), var(--hue-b));
  border-color: transparent;
  box-shadow: 0 6px 18px rgba(167, 139, 250, 0.42);
}
.send.hot:hover {
  transform: scale(1.06) rotate(-4deg);
}
.send:disabled {
  cursor: default;
}

.hidden {
  display: none;
}

.foottip {
  margin-top: 6px;
  text-align: center;
  font-size: 10px;
  color: var(--ink-4);
  letter-spacing: 0.02em;
}

/* ---- 录音条 ---- */
.recbar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border-radius: var(--r-lg);
}

.rb {
  width: 34px;
  height: 34px;
  display: grid;
  place-items: center;
  border-radius: 11px;
  transition: background 0.16s, color 0.16s;
}
.rb svg {
  width: 18px;
  height: 18px;
}
.rb.cancel {
  color: #fda4af;
}
.rb.cancel:hover {
  background: rgba(244, 63, 94, 0.16);
}
.rb.stop {
  margin-left: auto;
  color: #12101c;
  background: linear-gradient(135deg, var(--hue-c), var(--hue-a));
}

.recdot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #f43f5e;
  animation: pulse-ring 1.4s infinite;
}

.rectime {
  font-size: 12.5px;
  color: var(--ink-1);
  min-width: 38px;
}

.meter {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 3px;
  height: 32px;
}
.meter i {
  width: 3px;
  border-radius: 2px;
  background: linear-gradient(to top, var(--hue-a), var(--hue-b));
  transition: height 0.08s linear;
  opacity: 0.85;
}

.rec-enter-active,
.rec-leave-active {
  transition: opacity 0.22s var(--ease-smooth), transform 0.22s var(--ease-out);
}
.rec-enter-from,
.rec-leave-to {
  opacity: 0;
  transform: translateY(8px) scale(0.98);
}
</style>
