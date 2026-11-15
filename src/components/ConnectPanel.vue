<script setup lang="ts">
import { computed, ref } from "vue";
import { state, toast } from "../core/store";
import { connect } from "../core/engine";

const addr = ref("");
const busy = ref(false);

const myAddr = computed(() => {
  const ip = state.host.lan_ips[0] || "127.0.0.1";
  return `${ip}:${state.host.port}`;
});

const samples = ["192.168.1.20", "::1", "pc.local", "10.0.0.5:19779"];

async function go() {
  const v = addr.value.trim();
  if (!v) {
    toast("先填个地址", "warn");
    return;
  }
  busy.value = true;
  try {
    await connect(v);
  } finally {
    // 连接是异步握手，这里只解除按钮禁用，真实状态看 link
    setTimeout(() => (busy.value = false), 900);
  }
}

async function copyMine() {
  try {
    await navigator.clipboard.writeText(myAddr.value);
    toast(`已复制 ${myAddr.value}`, "ok");
  } catch {
    toast("复制失败，请手动选中", "warn");
  }
}

function useSample(s: string) {
  addr.value = s;
}
</script>

<template>
  <div class="connect">
    <div class="stage">
      <!-- 左：连接表单 -->
      <div class="card glass glass-edge">
        <div class="head">
          <div class="orb"></div>
          <div>
            <h2>连接一台设备</h2>
            <p class="sub">填入对方的地址，双方都用 19779 端口</p>
          </div>
        </div>

        <label class="lbl">对方地址</label>
        <div class="inputrow">
          <input
            v-model="addr"
            class="field mono big"
            placeholder="IPv4 / IPv6 / 域名 / 局域网名"
            spellcheck="false"
            autocomplete="off"
            @keydown.enter="go"
          />
          <button class="btn btn-primary connectbtn" :disabled="busy" @click="go">
            <svg v-if="!busy" viewBox="0 0 20 20">
              <path d="M4 10h11M11 5.5 15.5 10 11 14.5" stroke="currentColor" stroke-width="1.7" fill="none" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
            <span v-else class="spin"></span>
            {{ busy ? "连接中" : "连接" }}
          </button>
        </div>

        <div class="samples">
          <span class="sl">试试</span>
          <button v-for="s in samples" :key="s" class="chip mono" @click="useSample(s)">{{ s }}</button>
        </div>

        <div class="divider"><span>或者让对方连过来</span></div>

        <button class="mine" @click="copyMine">
          <div class="mtext">
            <span class="mlabel">把这台机器的地址发给对方</span>
            <span class="mval mono">{{ myAddr }}</span>
          </div>
          <svg class="mcopy" viewBox="0 0 16 16">
            <rect x="5.5" y="5.5" width="8" height="8" rx="1.6" stroke="currentColor" fill="none" stroke-width="1.2" />
            <path d="M10.5 5.5V4a1.5 1.5 0 0 0-1.5-1.5H4A1.5 1.5 0 0 0 2.5 4v5A1.5 1.5 0 0 0 4 10.5h1.5" stroke="currentColor" fill="none" stroke-width="1.2" />
          </svg>
        </button>

        <p v-if="state.link.state === 'connecting'" class="connecting">正在握手…</p>
      </div>

      <!-- 右：特性说明 -->
      <div class="side">
        <div v-for="(f, i) in [
          { t: '点对点直连', d: '不经过任何服务器，数据只在这两台设备之间流动', c: 'var(--hue-a)' },
          { t: '任意文件', d: '拖进来就发，图片视频自动预览', c: 'var(--hue-b)' },
          { t: '语音 · 视频', d: '实时通话，也支持录一段发过去', c: 'var(--hue-c)' },
          { t: '协助 · 邀请制', d: '只有对方主动邀请，才能查看屏幕或执行指令', c: 'var(--hue-warm)' },
        ]" :key="f.t" class="feat" :style="{ animationDelay: `${140 + i * 90}ms` }">
          <i class="dotc" :style="{ background: f.c }"></i>
          <div>
            <div class="ft">{{ f.t }}</div>
            <div class="fd">{{ f.d }}</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.connect {
  position: relative;
  z-index: 10;
  flex: 1;
  display: grid;
  place-items: center;
  padding: 28px;
  overflow-y: auto;
}

.stage {
  display: flex;
  gap: 26px;
  align-items: stretch;
  max-width: 940px;
  width: 100%;
  animation: fade-up 0.55s var(--ease-out) both;
}

.card {
  flex: 1 1 520px;
  max-width: 540px;
  padding: 26px;
  border-radius: var(--r-lg);
}

.head {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-bottom: 22px;
}
.head h2 {
  font-size: 17px;
  font-weight: 650;
  letter-spacing: 0.01em;
}
.sub {
  margin-top: 3px;
  font-size: 11.5px;
  color: var(--ink-3);
}

.orb {
  width: 42px;
  height: 42px;
  flex: 0 0 42px;
  border-radius: 14px;
  background: radial-gradient(circle at 32% 28%, #fff8, transparent 55%),
    conic-gradient(from 200deg, var(--hue-a), var(--hue-b), var(--hue-c), var(--hue-a));
  box-shadow: 0 8px 24px rgba(167, 139, 250, 0.4), inset 0 1px 0 rgba(255, 255, 255, 0.6);
  animation: breathe 5s var(--ease-smooth) infinite;
}

.lbl {
  display: block;
  margin-bottom: 7px;
  font-size: 11px;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--ink-3);
}

.inputrow {
  display: flex;
  gap: 8px;
}

.field.big {
  flex: 1;
  padding: 12px 14px;
  font-size: 13.5px;
}

.connectbtn {
  flex: 0 0 auto;
  padding: 0 18px;
  font-size: 13px;
}
.connectbtn svg {
  width: 17px;
  height: 17px;
}

.spin {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  border: 2px solid rgba(18, 16, 28, 0.3);
  border-top-color: #12101c;
  animation: spin 0.7s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.samples {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  margin-top: 10px;
}
.sl {
  font-size: 10.5px;
  color: var(--ink-4);
}
.chip {
  padding: 3px 9px;
  font-size: 11px;
  border-radius: var(--r-pill);
  color: var(--ink-2);
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid var(--line-1);
  transition: background 0.16s, color 0.16s, border-color 0.16s;
}
.chip:hover {
  background: rgba(167, 139, 250, 0.16);
  border-color: rgba(167, 139, 250, 0.4);
  color: var(--ink-1);
}

.divider {
  display: flex;
  align-items: center;
  gap: 12px;
  margin: 20px 0 14px;
  font-size: 10.5px;
  color: var(--ink-4);
}
.divider::before,
.divider::after {
  content: "";
  flex: 1;
  height: 1px;
  background: var(--line-1);
}

.mine {
  display: flex;
  align-items: center;
  width: 100%;
  padding: 12px 14px;
  border-radius: var(--r-md);
  text-align: left;
  background: linear-gradient(120deg, rgba(94, 234, 212, 0.10), rgba(167, 139, 250, 0.09));
  border: 1px solid rgba(94, 234, 212, 0.24);
  transition: border-color 0.2s, transform 0.2s var(--ease-spring), background 0.2s;
}
.mine:hover {
  border-color: rgba(94, 234, 212, 0.55);
  transform: translateY(-1px);
}
.mtext {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
}
.mlabel {
  font-size: 10.5px;
  color: var(--ink-3);
}
.mval {
  font-size: 13.5px;
  color: var(--ink-1);
  letter-spacing: 0.02em;
}
.mcopy {
  width: 15px;
  height: 15px;
  margin-left: auto;
  color: var(--ink-3);
}

.connecting {
  margin-top: 12px;
  font-size: 11.5px;
  color: var(--hue-a);
  animation: breathe 1.6s ease-in-out infinite;
}

/* 右侧特性 */
.side {
  flex: 0 0 300px;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 16px;
  padding: 8px 4px;
}

.feat {
  display: flex;
  gap: 11px;
  animation: fade-up 0.55s var(--ease-out) both;
}
.dotc {
  width: 7px;
  height: 7px;
  margin-top: 7px;
  flex: 0 0 7px;
  border-radius: 50%;
  box-shadow: 0 0 10px currentColor;
}
.ft {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--ink-1);
}
.fd {
  margin-top: 2px;
  font-size: 11px;
  line-height: 1.55;
  color: var(--ink-3);
}

@media (max-width: 1080px) {
  .side {
    display: none;
  }
  .card {
    max-width: 620px;
  }
}
</style>
