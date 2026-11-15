<script setup lang="ts">
import { computed } from "vue";
import { state, toast } from "../core/store";
import { disconnect } from "../core/engine";

const emit = defineEmits<{
  (e: "settings"): void;
}>();

const online = computed(() => state.link.state === "online");

const addresses = computed(() => {
  // IPv4 优先，IPv6 只留第一条 —— 一屏塞太多长地址反而看不清
  const v4 = state.host.lan_ips.filter((ip) => !ip.startsWith("["));
  const v6 = state.host.lan_ips.filter((ip) => ip.startsWith("["));
  const list = [...v4, ...v6.slice(0, 1)];
  if (!list.length) list.push("127.0.0.1");
  return list.slice(0, 4);
});

async function copy(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    toast(`已复制 ${text}`, "ok");
  } catch {
    toast("复制失败，请手动选中", "warn");
  }
}

const myInitial = computed(() => (state.settings.myName || "我").slice(0, 1));

const platformIcon = computed(() => {
  const p = state.me.platform;
  if (p === "windows") return "⊞";
  if (p === "android") return "◉";
  return "◇";
});
</script>

<template>
  <aside class="sidebar">
    <!-- 我 -->
    <div class="me">
      <div class="avatar">
        <span>{{ myInitial }}</span>
        <i class="ring"></i>
      </div>
      <div class="who">
        <input
          v-model="state.settings.myName"
          class="namefield"
          maxlength="24"
          spellcheck="false"
          aria-label="我的昵称"
        />
        <div class="meta">
          <span class="pill">{{ platformIcon }} {{ state.me.platform || "…" }}</span>
          <span class="pill mono">{{ state.host.hostname || "—" }}</span>
        </div>
      </div>
    </div>

    <!-- 本机地址 -->
    <section class="block">
      <h4>本机可达地址<span class="sub">让别人填这里</span></h4>
      <div class="addrlist">
        <button
          v-for="(ip, i) in addresses"
          :key="ip"
          class="addr"
          :title="`${ip}:${state.host.port}`"
          :style="{ animationDelay: `${i * 55}ms` }"
          @click="copy(ip)"
        >
          <span class="mono ip">{{ ip }}</span>
          <span class="port">:{{ state.host.port }}</span>
          <svg class="copyi" viewBox="0 0 16 16">
            <rect x="5.5" y="5.5" width="8" height="8" rx="1.6" stroke="currentColor" fill="none" stroke-width="1.2" />
            <path d="M10.5 5.5V4a1.5 1.5 0 0 0-1.5-1.5H4A1.5 1.5 0 0 0 2.5 4v5A1.5 1.5 0 0 0 4 10.5h1.5" stroke="currentColor" fill="none" stroke-width="1.2" />
          </svg>
        </button>
      </div>
      <p class="hint">同一局域网可直接连；跨网需要公网 IP 或域名。</p>
    </section>

    <!-- 对方 -->
    <section class="block peer">
      <h4>对方</h4>
      <transition name="fade">
        <div v-if="online" class="peerbox glass glass-edge">
          <div class="pavatar">{{ (state.peer.name || "?").slice(0, 1) }}</div>
          <div class="pinfo">
            <div class="pname">{{ state.peer.name || "正在握手…" }}</div>
            <div class="pstate">
              <i class="live"></i>
              {{ state.link.role === "host" ? "对方接入" : "已连出" }}
            </div>
          </div>
        </div>
        <div v-else class="peerbox idle">
          <div class="ghost">还没有人连接</div>
          <div class="ghost-sub">在右侧填入对方地址，或把这台机器的地址发给对方</div>
        </div>
      </transition>
    </section>

    <!-- 底部 -->
    <div class="foot">
      <button class="fbtn" title="设置" @click="emit('settings')">
        <svg viewBox="0 0 20 20">
          <circle cx="10" cy="10" r="2.6" stroke="currentColor" fill="none" stroke-width="1.4" />
          <path
            d="M10 2.4v1.7M10 15.9v1.7M2.4 10h1.7M15.9 10h1.7M4.6 4.6l1.2 1.2M14.2 14.2l1.2 1.2M15.4 4.6l-1.2 1.2M5.8 14.2l-1.2 1.2"
            stroke="currentColor"
            stroke-width="1.4"
            stroke-linecap="round"
          />
        </svg>
        <span>设置</span>
      </button>

      <button
        class="fbtn"
        title="切换主题"
        @click="state.settings.theme = state.settings.theme === 'dark' ? 'light' : 'dark'"
      >
        <svg v-if="state.settings.theme === 'dark'" viewBox="0 0 20 20">
          <path
            d="M16 12.2A6.8 6.8 0 0 1 7.8 4a6.8 6.8 0 1 0 8.2 8.2Z"
            stroke="currentColor"
            fill="none"
            stroke-width="1.4"
            stroke-linejoin="round"
          />
        </svg>
        <svg v-else viewBox="0 0 20 20">
          <circle cx="10" cy="10" r="3.4" stroke="currentColor" fill="none" stroke-width="1.4" />
          <path
            d="M10 2v1.6M10 16.4V18M2 10h1.6M16.4 10H18M4.5 4.5l1.1 1.1M14.4 14.4l1.1 1.1M15.5 4.5l-1.1 1.1M5.6 14.4l-1.1 1.1"
            stroke="currentColor"
            stroke-width="1.4"
            stroke-linecap="round"
          />
        </svg>
        <span>{{ state.settings.theme === "dark" ? "亮色" : "深色" }}</span>
      </button>

      <button v-if="online" class="fbtn danger" title="断开" @click="disconnect">
        <svg viewBox="0 0 20 20">
          <path d="M7 13l6-6M6.2 6.2a4.6 4.6 0 0 0 0 6.5" stroke="currentColor" fill="none" stroke-width="1.4" stroke-linecap="round" />
          <path d="M13.8 13.8a4.6 4.6 0 0 0 0-6.5" stroke="currentColor" fill="none" stroke-width="1.4" stroke-linecap="round" />
        </svg>
        <span>断开</span>
      </button>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  position: relative;
  z-index: 10;
  width: var(--sidebar-w);
  flex: 0 0 var(--sidebar-w);
  display: flex;
  flex-direction: column;
  gap: 18px;
  padding: 16px 14px 12px;
  background: rgba(12, 15, 23, 0.42);
  backdrop-filter: blur(38px) saturate(150%);
  -webkit-backdrop-filter: blur(38px) saturate(150%);
  border-right: 1px solid var(--line-1);
  overflow-y: auto;
}
html[data-theme="light"] .sidebar {
  background: rgba(248, 250, 254, 0.55);
}

/* ---- 我 ---- */
.me {
  display: flex;
  align-items: center;
  gap: 11px;
  padding: 4px 2px 2px;
}

.avatar {
  position: relative;
  width: 44px;
  height: 44px;
  flex: 0 0 44px;
  border-radius: 15px;
  display: grid;
  place-items: center;
  font-size: 18px;
  font-weight: 700;
  color: #14101f;
  background: linear-gradient(135deg, var(--hue-a), var(--hue-b));
  box-shadow: 0 6px 18px rgba(167, 139, 250, 0.38);
}
.ring {
  position: absolute;
  inset: -3px;
  border-radius: 18px;
  border: 1px solid rgba(255, 255, 255, 0.22);
  animation: breathe 4s var(--ease-smooth) infinite;
}

.who {
  min-width: 0;
  flex: 1;
}

.namefield {
  width: 100%;
  padding: 2px 6px;
  margin-left: -6px;
  font-size: 14px;
  font-weight: 620;
  color: var(--ink-1);
  background: transparent;
  border: 1px solid transparent;
  border-radius: 7px;
  outline: none;
  transition: background 0.2s, border-color 0.2s;
}
.namefield:hover {
  background: rgba(255, 255, 255, 0.06);
}
.namefield:focus {
  background: rgba(0, 0, 0, 0.25);
  border-color: rgba(167, 139, 250, 0.5);
}

.meta {
  display: flex;
  gap: 5px;
  margin-top: 3px;
  overflow: hidden;
}
.pill {
  font-size: 10px;
  padding: 1px 7px;
  border-radius: var(--r-pill);
  color: var(--ink-3);
  background: rgba(255, 255, 255, 0.055);
  border: 1px solid var(--line-1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* ---- 区块 ---- */
.block h4 {
  display: flex;
  align-items: baseline;
  gap: 6px;
  margin-bottom: 8px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--ink-3);
}
.sub {
  font-size: 10px;
  letter-spacing: 0;
  text-transform: none;
  color: var(--ink-4);
  font-weight: 400;
}

.addrlist {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.addr {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 8px 10px;
  border-radius: var(--r-sm);
  text-align: left;
  color: var(--ink-1);
  background: rgba(255, 255, 255, 0.045);
  border: 1px solid var(--line-1);
  transition: background 0.18s, border-color 0.18s, transform 0.18s var(--ease-spring);
  animation: fade-up 0.45s var(--ease-out) both;
}
.addr:hover {
  background: rgba(167, 139, 250, 0.14);
  border-color: rgba(167, 139, 250, 0.38);
  transform: translateX(2px);
}
.ip {
  font-size: 12.5px;
  letter-spacing: 0.01em;
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.port {
  font-size: 11px;
  color: var(--ink-3);
  margin-left: 1px;
}
.copyi {
  width: 13px;
  height: 13px;
  margin-left: auto;
  color: var(--ink-3);
  opacity: 0;
  transition: opacity 0.18s;
}
.addr:hover .copyi {
  opacity: 1;
}

.hint {
  margin-top: 7px;
  font-size: 10.5px;
  line-height: 1.5;
  color: var(--ink-4);
}

/* ---- 对方 ---- */
.peer {
  flex: 1 1 auto;
  min-height: 90px;
}

.peerbox {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 11px;
  border-radius: var(--r-md);
  animation: pop-in 0.42s var(--ease-spring) both;
}

.pavatar {
  width: 36px;
  height: 36px;
  flex: 0 0 36px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  font-weight: 700;
  color: #0d1018;
  background: linear-gradient(135deg, var(--hue-c), var(--hue-a));
}

.pinfo {
  min-width: 0;
}
.pname {
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.pstate {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 10.5px;
  color: var(--ink-3);
}
.live {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: #4ade80;
  box-shadow: 0 0 7px rgba(74, 222, 128, 0.8);
}

.idle {
  flex-direction: column;
  align-items: flex-start;
  gap: 3px;
  border: 1px dashed var(--line-1);
  background: rgba(255, 255, 255, 0.02);
}
.ghost {
  font-size: 12px;
  color: var(--ink-3);
}
.ghost-sub {
  font-size: 10.5px;
  line-height: 1.5;
  color: var(--ink-4);
}

/* ---- 底部 ---- */
.foot {
  display: flex;
  gap: 6px;
  padding-top: 10px;
  border-top: 1px solid var(--line-1);
}

.fbtn {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 3px;
  padding: 7px 2px;
  border-radius: var(--r-sm);
  font-size: 10px;
  color: var(--ink-3);
  transition: background 0.18s, color 0.18s;
}
.fbtn svg {
  width: 17px;
  height: 17px;
}
.fbtn:hover {
  background: rgba(255, 255, 255, 0.07);
  color: var(--ink-1);
}
.fbtn.danger:hover {
  background: rgba(244, 63, 94, 0.16);
  color: #fda4af;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s var(--ease-smooth), transform 0.3s var(--ease-out);
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(6px);
}
</style>
