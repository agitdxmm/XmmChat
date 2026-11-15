<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import Wallpaper from "./components/Wallpaper.vue";
import TitleBar from "./components/TitleBar.vue";
import SideBar from "./components/SideBar.vue";
import ConnectPanel from "./components/ConnectPanel.vue";
import ChatView from "./components/ChatView.vue";
import AssistSheet from "./components/AssistSheet.vue";
import SettingsSheet from "./components/SettingsSheet.vue";
import CallOverlay from "./components/CallOverlay.vue";

import { boot, pendingInvite, replyInvite } from "./core/engine";
import { startCall } from "./core/call";
import { state } from "./core/store";

const online = computed(() => state.link.state === "online");

const sheet = ref<null | "assist" | "term">(null);
const tab = ref<"assist" | "term">("assist");
const settingsOpen = ref(false);

function openAssist() {
  tab.value = "assist";
  sheet.value = "assist";
}
function openTerm() {
  tab.value = "term";
  sheet.value = "term";
}

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") {
    if (pendingInvite.value) {
      replyInvite(false);
      return;
    }
    if (sheet.value) sheet.value = null;
    if (settingsOpen.value) settingsOpen.value = false;
  }
}

onMounted(async () => {
  await boot();
  window.addEventListener("keydown", onKey);
});

onBeforeUnmount(() => window.removeEventListener("keydown", onKey));

const inviteScopes = computed(() => pendingInvite.value?.scopes || []);
</script>

<template>
  <div class="app">
    <Wallpaper />
    <TitleBar />

    <div class="body">
      <SideBar @settings="settingsOpen = true" />

      <ChatView
        v-if="online"
        @call="(v: boolean) => startCall(v)"
        @assist="openAssist"
        @terminal="openTerm"
        @settings="settingsOpen = true"
      />
      <ConnectPanel v-else />
    </div>

    <!-- 协助 / 终端 -->
    <AssistSheet
      v-if="sheet"
      :tab="tab"
      @close="sheet = null"
      @tab="(v) => (tab = v)"
    />

    <!-- 设置 -->
    <SettingsSheet v-if="settingsOpen" @close="settingsOpen = false" />

    <!-- 通话 -->
    <CallOverlay />

    <!-- 协助邀请确认（邀请制的关键一环） -->
    <transition name="inv">
      <div v-if="pendingInvite" class="imask" @click.self="replyInvite(false)">
        <div class="icard glass glass-edge">
          <div class="ihead">
            <div class="iava">{{ (state.peer.name || "?").slice(0, 1) }}</div>
            <div>
              <h3>{{ state.peer.name || "对方" }} 邀请你协助</h3>
              <p class="isub">对方希望你能在他的设备上做下面这些事</p>
            </div>
          </div>

          <ul class="iscopes">
            <li v-for="s in inviteScopes" :key="s">
              <i class="ib"></i>
              <span v-if="s === 'screen'">查看他的屏幕</span>
              <span v-else>在他的设备上执行指令</span>
            </li>
            <li v-if="!inviteScopes.length"><i class="ib"></i><span>（未指定范围）</span></li>
          </ul>

          <p v-if="pendingInvite.note" class="inote">「{{ pendingInvite.note }}」</p>

          <p class="iwarn">接受后，对方就能按上面的范围操作你的设备。随时可以在「协助」里收回。</p>

          <div class="ibtns">
            <button class="btn" @click="replyInvite(false)">拒绝</button>
            <button class="btn btn-primary" @click="replyInvite(true)">同意并授权</button>
          </div>
        </div>
      </div>
    </transition>

    <!-- 提示 -->
    <div class="toasts">
      <transition-group name="toast">
        <div v-for="t in state.toasts" :key="t.id" class="toast" :class="t.level">
          <span class="tbar"></span>
          <span class="tt">{{ t.text }}</span>
        </div>
      </transition-group>
    </div>
  </div>
</template>

<style scoped>
.app {
  position: relative;
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border-radius: 0;
  /* 手机上的刘海 / 手势条留白，桌面端 env() 为 0，不影响 */
  padding-top: env(safe-area-inset-top, 0px);
  padding-bottom: env(safe-area-inset-bottom, 0px);
}

.body {
  position: relative;
  z-index: 5;
  flex: 1;
  display: flex;
  min-height: 0;
}

/* ---- 邀请卡 ---- */
.imask {
  position: fixed;
  inset: 0;
  z-index: 120;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgba(4, 6, 11, 0.55);
  backdrop-filter: blur(10px);
}

.icard {
  width: min(430px, 92vw);
  padding: 22px;
  border-radius: var(--r-lg);
  animation: pop-in 0.34s var(--ease-spring);
}

.ihead {
  display: flex;
  align-items: center;
  gap: 13px;
}
.iava {
  width: 46px;
  height: 46px;
  flex: 0 0 46px;
  display: grid;
  place-items: center;
  font-size: 19px;
  font-weight: 700;
  color: #0d1018;
  border-radius: 15px;
  background: linear-gradient(135deg, var(--hue-c), var(--hue-a));
  animation: breathe 2.6s var(--ease-smooth) infinite;
}
.ihead h3 {
  font-size: 15px;
  font-weight: 650;
}
.isub {
  margin-top: 3px;
  font-size: 11px;
  color: var(--ink-3);
}

.iscopes {
  list-style: none;
  margin: 16px 0 0;
  padding: 12px;
  border-radius: var(--r-md);
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--line-1);
  display: flex;
  flex-direction: column;
  gap: 9px;
}
.iscopes li {
  display: flex;
  align-items: center;
  gap: 9px;
  font-size: 12.5px;
  color: var(--ink-1);
}
.ib {
  width: 6px;
  height: 6px;
  flex: 0 0 6px;
  border-radius: 50%;
  background: var(--hue-a);
  box-shadow: 0 0 9px rgba(167, 139, 250, 0.9);
}

.inote {
  margin-top: 12px;
  padding: 9px 12px;
  font-size: 12px;
  line-height: 1.6;
  color: var(--ink-2);
  border-left: 2px solid var(--hue-b);
  background: rgba(240, 171, 252, 0.07);
  border-radius: 0 var(--r-xs) var(--r-xs) 0;
}

.iwarn {
  margin-top: 12px;
  font-size: 10.5px;
  line-height: 1.6;
  color: #fbbf24cc;
}

.ibtns {
  display: flex;
  gap: 9px;
  margin-top: 18px;
}
.ibtns .btn {
  flex: 1;
  justify-content: center;
  padding: 10px;
}

/* ---- toast ---- */
.toasts {
  position: fixed;
  top: calc(var(--titlebar-h) + 12px);
  right: 16px;
  z-index: 200;
  display: flex;
  flex-direction: column;
  gap: 8px;
  pointer-events: none;
}

.toast {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 9px 14px 9px 10px;
  max-width: 340px;
  font-size: 12px;
  line-height: 1.5;
  color: var(--ink-1);
  border-radius: var(--r-sm);
  background: rgba(20, 24, 34, 0.86);
  border: 1px solid var(--line-2);
  box-shadow: var(--shadow-2);
  backdrop-filter: blur(20px);
}
html[data-theme="light"] .toast {
  background: rgba(255, 255, 255, 0.94);
}
.tbar {
  width: 3px;
  height: 16px;
  border-radius: 2px;
  background: var(--hue-a);
}
.toast.ok .tbar {
  background: #4ade80;
}
.toast.warn .tbar {
  background: #fbbf24;
}
.toast.err .tbar {
  background: #f43f5e;
}

.toast-enter-active,
.toast-leave-active {
  transition: all 0.3s var(--ease-out);
}
.toast-enter-from {
  opacity: 0;
  transform: translateX(20px) scale(0.96);
}
.toast-leave-to {
  opacity: 0;
  transform: translateX(20px) scale(0.96);
}

.inv-enter-active,
.inv-leave-active {
  transition: opacity 0.22s var(--ease-smooth);
}
.inv-enter-from,
.inv-leave-to {
  opacity: 0;
}

/* 窗口圆角：配合 tauri 的 windowEffects.radius */
:global(html),
:global(body) {
  border-radius: 14px;
}

/* ---- 窄屏（手机 / 小窗）：收起侧栏，聊天占满 ---- */
@media (max-width: 720px) {
  :global(aside.sidebar) {
    display: none !important;
  }
  :global(html),
  :global(body) {
    border-radius: 0;
  }
}
</style>
