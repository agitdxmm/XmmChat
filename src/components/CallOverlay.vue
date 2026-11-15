<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { state, fmtDuration } from "../core/store";
import { acceptCall, camOn, hangup, localStream, micOn, rejectCall, remoteStream, toggleCam, toggleMic } from "../core/call";

const remoteEl = ref<HTMLVideoElement | null>(null);
const localEl = ref<HTMLVideoElement | null>(null);
const seconds = ref(0);
let timer = 0;

watch(remoteStream, (s) => {
  if (remoteEl.value && s) remoteEl.value.srcObject = s;
});
watch(localStream, (s) => {
  if (localEl.value && s) localEl.value.srcObject = s;
});

watch(
  () => state.call.phase,
  (p) => {
    window.clearInterval(timer);
    if (p === "active") {
      seconds.value = 0;
      timer = window.setInterval(() => (seconds.value += 1), 1000);
    }
  }
);

onBeforeUnmount(() => window.clearInterval(timer));

const phase = computed(() => state.call.phase);
const isVideo = computed(() => state.call.video || state.call.incomingVideo);
const incoming = computed(() => phase.value === "incoming");

const initial = computed(() => (state.peer.name || "?").slice(0, 1));
</script>

<template>
  <transition name="callfade">
    <div v-if="phase !== 'idle' && phase !== 'ended'" class="call">
      <div class="bgblur"></div>

      <!-- 视频层 -->
      <div class="stage">
        <video
          v-show="isVideo && phase === 'active'"
          ref="remoteEl"
          class="remote"
          autoplay
          playsinline
        ></video>

        <div v-if="!isVideo || phase !== 'active'" class="voiceface">
          <div class="bigav">{{ initial }}</div>
          <div class="rings">
            <i></i><i></i><i></i>
          </div>
          <div class="aname">{{ state.peer.name || "对方" }}</div>
          <div class="astate">
            <template v-if="incoming">来电 · {{ isVideo ? "视频" : "语音" }}通话</template>
            <template v-else-if="phase === 'outing'">正在呼叫…</template>
            <template v-else>{{ fmtDuration(seconds) }}</template>
          </div>
        </div>

        <!-- 本地画中画 -->
        <video
          v-show="isVideo && phase === 'active'"
          ref="localEl"
          class="local"
          autoplay
          playsinline
          muted
        ></video>
      </div>

      <!-- 控制条 -->
      <div class="controls">
        <template v-if="incoming">
          <button class="cbtn reject" @click="rejectCall">
            <svg viewBox="0 0 24 24">
              <path d="M4.5 4.8h3.8l1.6 4.2-2.4 1.6a12 12 0 0 0 6.4 6.4l1.6-2.4 4.2 1.6v3.8a1.9 1.9 0 0 1-2.1 1.9C9.2 20.4 3.6 14.8 2.6 6.9A1.9 1.9 0 0 1 4.5 4.8Z" fill="currentColor" />
              <path d="M3 21 21 3" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" />
            </svg>
            <span>拒绝</span>
          </button>
          <button class="cbtn accept" @click="acceptCall">
            <svg viewBox="0 0 24 24">
              <path d="M4.5 4.8h3.8l1.6 4.2-2.4 1.6a12 12 0 0 0 6.4 6.4l1.6-2.4 4.2 1.6v3.8a1.9 1.9 0 0 1-2.1 1.9C9.2 20.4 3.6 14.8 2.6 6.9A1.9 1.9 0 0 1 4.5 4.8Z" fill="currentColor" />
            </svg>
            <span>接听</span>
          </button>
        </template>

        <template v-else>
          <button class="cbtn round" :class="{ off: !micOn }" title="麦克风" @click="toggleMic">
            <svg v-if="micOn" viewBox="0 0 24 24">
              <rect x="9" y="3" width="6" height="11" rx="3" stroke="currentColor" fill="none" stroke-width="1.7" />
              <path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21" stroke="currentColor" stroke-width="1.7" fill="none" stroke-linecap="round" />
            </svg>
            <svg v-else viewBox="0 0 24 24">
              <rect x="9" y="3" width="6" height="11" rx="3" stroke="currentColor" fill="none" stroke-width="1.7" />
              <path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21M3.5 3.5l17 17" stroke="currentColor" stroke-width="1.7" fill="none" stroke-linecap="round" />
            </svg>
          </button>

          <button v-if="isVideo" class="cbtn round" :class="{ off: !camOn }" title="摄像头" @click="toggleCam">
            <svg v-if="camOn" viewBox="0 0 24 24">
              <rect x="3" y="6.5" width="12" height="11" rx="2.6" stroke="currentColor" fill="none" stroke-width="1.7" />
              <path d="M15 10.8 21 7.6v8.8l-6-3.2" stroke="currentColor" fill="none" stroke-width="1.7" stroke-linejoin="round" />
            </svg>
            <svg v-else viewBox="0 0 24 24">
              <rect x="3" y="6.5" width="12" height="11" rx="2.6" stroke="currentColor" fill="none" stroke-width="1.7" />
              <path d="M15 10.8 21 7.6v8.8l-6-3.2M3 3l18 18" stroke="currentColor" fill="none" stroke-width="1.7" stroke-linejoin="round" />
            </svg>
          </button>

          <button class="cbtn end" title="挂断" @click="hangup">
            <svg viewBox="0 0 24 24">
              <path d="M4.5 4.8h3.8l1.6 4.2-2.4 1.6a12 12 0 0 0 6.4 6.4l1.6-2.4 4.2 1.6v3.8a1.9 1.9 0 0 1-2.1 1.9C9.2 20.4 3.6 14.8 2.6 6.9A1.9 1.9 0 0 1 4.5 4.8Z" fill="currentColor" transform="rotate(135 12 12)" />
            </svg>
          </button>
        </template>
      </div>

      <div v-if="phase === 'active'" class="clock mono">{{ fmtDuration(seconds) }}</div>
    </div>
  </transition>
</template>

<style scoped>
.call {
  position: fixed;
  inset: 0;
  z-index: 150;
  display: flex;
  flex-direction: column;
  background: #05070c;
  overflow: hidden;
}

.bgblur {
  position: absolute;
  inset: -10%;
  background: radial-gradient(circle at 30% 20%, rgba(167, 139, 250, 0.28), transparent 55%),
    radial-gradient(circle at 75% 75%, rgba(94, 234, 212, 0.22), transparent 55%),
    radial-gradient(circle at 50% 50%, rgba(240, 171, 252, 0.16), transparent 60%);
  filter: blur(60px);
  animation: drift-a 22s var(--ease-smooth) infinite;
}

.stage {
  position: relative;
  flex: 1;
  display: grid;
  place-items: center;
  overflow: hidden;
}

.remote {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.local {
  position: absolute;
  right: 22px;
  bottom: 22px;
  width: 190px;
  aspect-ratio: 16 / 10;
  object-fit: cover;
  border-radius: 16px;
  border: 1px solid rgba(255, 255, 255, 0.18);
  box-shadow: var(--shadow-3);
  background: #0b0e14;
  transform: scaleX(-1);
}

.voiceface {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  z-index: 2;
}

.bigav {
  width: 116px;
  height: 116px;
  display: grid;
  place-items: center;
  font-size: 44px;
  font-weight: 700;
  color: #0d1018;
  border-radius: 38px;
  background: linear-gradient(135deg, var(--hue-c), var(--hue-a));
  box-shadow: 0 18px 50px rgba(94, 234, 212, 0.3);
  animation: breathe 3.4s var(--ease-smooth) infinite;
}

.rings {
  position: absolute;
  top: 0;
  left: 50%;
  transform: translateX(-50%);
  pointer-events: none;
}
.rings i {
  position: absolute;
  top: 58px;
  left: 50%;
  width: 116px;
  height: 116px;
  margin-left: -58px;
  margin-top: -58px;
  border-radius: 38px;
  border: 1px solid rgba(255, 255, 255, 0.22);
  animation: ring 2.8s var(--ease-out) infinite;
}
.rings i:nth-child(2) {
  animation-delay: 0.9s;
}
.rings i:nth-child(3) {
  animation-delay: 1.8s;
}
@keyframes ring {
  0% {
    transform: scale(1);
    opacity: 0.55;
  }
  100% {
    transform: scale(1.85);
    opacity: 0;
  }
}

.aname {
  margin-top: 12px;
  font-size: 19px;
  font-weight: 650;
  color: #fff;
}
.astate {
  font-size: 12.5px;
  color: rgba(255, 255, 255, 0.55);
}

.controls {
  position: relative;
  z-index: 3;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 18px;
  padding: 26px 0 40px;
}

.cbtn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 7px;
  font-size: 11px;
  color: rgba(255, 255, 255, 0.8);
}
.cbtn svg {
  width: 26px;
  height: 26px;
}
.cbtn span {
  letter-spacing: 0.04em;
}
.cbtn svg,
.cbtn.round,
.end,
.reject,
.accept {
  transition: transform 0.2s var(--ease-spring), background 0.2s, box-shadow 0.2s;
}
.cbtn:hover {
  transform: translateY(-2px);
}

.cbtn.round {
  width: 54px;
  height: 54px;
  border-radius: 50%;
  justify-content: center;
  background: rgba(255, 255, 255, 0.12);
  border: 1px solid rgba(255, 255, 255, 0.15);
  backdrop-filter: blur(14px);
}
.cbtn.round svg {
  width: 23px;
  height: 23px;
}
.cbtn.round.off {
  background: rgba(255, 255, 255, 0.05);
  color: rgba(255, 255, 255, 0.4);
}

.cbtn.end {
  width: 62px;
  height: 62px;
  border-radius: 50%;
  justify-content: center;
  color: #fff;
  background: linear-gradient(135deg, #ef4444, #b91c1c);
  box-shadow: 0 10px 30px rgba(239, 68, 68, 0.45);
}
.cbtn.end svg {
  width: 26px;
  height: 26px;
}

.cbtn.accept {
  width: 74px;
  height: 74px;
  border-radius: 50%;
  justify-content: center;
  color: #062e1c;
  background: linear-gradient(135deg, #4ade80, #16a34a);
  box-shadow: 0 12px 34px rgba(74, 222, 128, 0.45);
  animation: pulse-ring 2s infinite;
}
.cbtn.accept svg {
  width: 30px;
  height: 30px;
}

.cbtn.reject {
  width: 74px;
  height: 74px;
  border-radius: 50%;
  justify-content: center;
  color: #fff;
  background: linear-gradient(135deg, #ef4444, #991b1b);
  box-shadow: 0 12px 34px rgba(239, 68, 68, 0.4);
}
.cbtn.reject svg {
  width: 30px;
  height: 30px;
}

.clock {
  position: absolute;
  top: 20px;
  left: 50%;
  transform: translateX(-50%);
  padding: 4px 12px;
  font-size: 12px;
  border-radius: var(--r-pill);
  color: rgba(255, 255, 255, 0.8);
  background: rgba(0, 0, 0, 0.35);
  backdrop-filter: blur(10px);
}

.callfade-enter-active,
.callfade-leave-active {
  transition: opacity 0.28s var(--ease-smooth), transform 0.28s var(--ease-out);
}
.callfade-enter-from,
.callfade-leave-to {
  opacity: 0;
  transform: scale(1.02);
}
</style>
