<script setup lang="ts">
import { computed, ref } from "vue";
import { state } from "../core/store";
import {
  inviteAssist,
  requestExec,
  revokeAssist,
  startScreenShare,
  stopScreenShare,
} from "../core/engine";
import type { AssistScope } from "../core/store";

const props = defineProps<{ tab: "assist" | "term" }>();
const emit = defineEmits<{
  (e: "close"): void;
  (e: "tab", v: "assist" | "term"): void;
}>();

const scopeScreen = ref(true);
const scopeExec = ref(false);
const note = ref("");
const cmd = ref("");

const online = computed(() => state.link.state === "online");
const granted = computed(() => state.granted);
const holding = computed(() => state.holding);

const canInvite = computed(() => scopeScreen.value || scopeExec.value);
const canExec = computed(() => holding.value.includes("exec"));
const canWatch = computed(() => holding.value.includes("screen"));

/** 常用命令，点一下就填进输入框 */
const QUICK: string[] = [
  "whoami",
  "ipconfig",
  "dir",
  'systeminfo | findstr /C:"OS 名称"',
];

const shellLabel = computed(() => state.shell.detail || "检测中…");
const hasShell = computed(() => state.shell.backend !== "none");

function doInvite() {
  const scopes: AssistScope[] = [];
  if (scopeScreen.value) scopes.push("screen");
  if (scopeExec.value) scopes.push("exec");
  if (!scopes.length) return;
  inviteAssist(scopes, note.value.trim());
  note.value = "";
}

function doExec() {
  const c = cmd.value.trim();
  if (!c) return;
  requestExec(c);
  cmd.value = "";
}

function toggleWatch() {
  if (state.monitor.on) stopScreenShare();
  else startScreenShare(2);
}
</script>

<template>
  <div class="mask" @click.self="emit('close')">
    <section class="sheet glass glass-edge">
      <header>
        <div class="htitle">
          <h3>协助与远程</h3>
          <span class="hsub">邀请制 · 只有对方点头才能动手</span>
        </div>
        <button class="x" @click="emit('close')">
          <svg viewBox="0 0 18 18"><path d="M5 5l8 8M13 5l-8 8" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" /></svg>
        </button>
      </header>

      <nav class="tabs">
        <button :class="{ on: props.tab === 'assist' }" @click="emit('tab', 'assist')">协助</button>
        <button :class="{ on: props.tab === 'term' }" @click="emit('tab', 'term')">远程终端</button>
      </nav>

      <div v-if="!online" class="offline">
        <svg viewBox="0 0 40 40">
          <path d="M20 8v12M20 28v.5" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" />
          <circle cx="20" cy="20" r="15" stroke="currentColor" stroke-width="2" fill="none" opacity="0.35" />
        </svg>
        <p>还没有连接对方</p>
        <span>先建立链路，才能发起或接收协助邀请</span>
      </div>

      <!-- ============ 协助 ============ -->
      <div v-else-if="props.tab === 'assist'" class="pane">
        <!-- 我已允许对方 -->
        <div v-if="granted.length" class="state okbox">
          <div class="sh">
            <i class="dotg"></i>
            我允许对方
            <span class="tags">
              <span v-for="s in granted" :key="s" class="tag">{{ s === "screen" ? "查看屏幕" : "执行指令" }}</span>
            </span>
          </div>
          <p class="sm">这是我自己点过同意的。任何时候都可以一键收回。</p>
          <div class="rowbtns">
            <button class="btn small" @click="toggleWatch">
              {{ state.monitor.on ? "停止推送画面" : "开始推送画面" }}
            </button>
            <button class="btn small danger" @click="revokeAssist">收回全部授权</button>
          </div>
        </div>

        <!-- 我获得的 -->
        <div v-if="holding.length" class="state okbox">
          <div class="sh">
            <i class="dotc"></i>
            对方允许我
            <span class="tags">
              <span v-for="s in holding" :key="s" class="tag">{{ s === "screen" ? "查看屏幕" : "执行指令" }}</span>
            </span>
          </div>
          <p class="sm">对方邀请了我，且我接受了。对方随时可以收回。</p>
          <div class="rowbtns">
            <button
              v-if="canWatch"
              class="btn small"
              @click="$emit('tab', 'term'); stopScreenShare(true)"
            >
              查看对方屏幕
            </button>
            <button v-if="canExec" class="btn small" @click="emit('tab', 'term')">打开终端</button>
          </div>
        </div>

        <!-- 邀请别人 -->
        <div class="invite">
          <h4>邀请对方协助我</h4>
          <p class="sm">
            发出邀请后，对方需要在他的设备上点「同意」。同意之前，他什么都做不了。
          </p>

          <label class="chk" :class="{ on: scopeScreen }">
            <input v-model="scopeScreen" type="checkbox" />
            <span class="cbox"></span>
            <div class="ctext">
              <strong>允许查看我的屏幕</strong>
              <em>以约 2 帧/秒推送当前画面</em>
            </div>
          </label>

          <label class="chk" :class="{ on: scopeExec }">
            <input v-model="scopeExec" type="checkbox" />
            <span class="cbox"></span>
            <div class="ctext">
              <strong>允许在我设备上执行指令</strong>
              <em>对方发来的命令会在本机 cmd 中执行</em>
            </div>
          </label>

          <input v-model="note" class="field note" placeholder="附一句话，比如「帮我看看这个报错」" maxlength="80" />

          <button class="btn btn-primary full" :disabled="!canInvite" @click="doInvite">
            发出协助邀请
          </button>
        </div>

        <div class="warnbox">
          <svg viewBox="0 0 20 20">
            <path d="M10 3.2 2.6 16.4h14.8L10 3.2Z" stroke="currentColor" fill="none" stroke-width="1.5" stroke-linejoin="round" />
            <path d="M10 8v3.6M10 13.6v.4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
          </svg>
          <p>授权时请确认对面是你认识的人。链路断开或程序退出，授权立即失效。</p>
        </div>
      </div>

      <!-- ============ 终端 ============ -->
      <div v-else class="pane">
        <div class="shellinfo" :class="{ ok: hasShell }">
          <i class="sdot"></i>
          <span class="slabel">本机执行环境</span>
          <span class="sval mono">{{ shellLabel }}</span>
        </div>

        <template v-if="canExec">
          <p class="sm">对方已授权。命令会在对方的设备上执行，输出会同步回到聊天记录。</p>
          <div class="cmdrow">
            <span class="prompt mono">›</span>
            <input
              v-model="cmd"
              class="field mono"
              placeholder="例如 dir /b  或  ipconfig"
              spellcheck="false"
              @keydown.enter="doExec"
            />
            <button class="btn btn-primary" @click="doExec">执行</button>
          </div>
          <div class="quick">
            <button v-for="q in QUICK" :key="q" class="chip mono" @click="cmd = q">
              {{ q }}
            </button>
          </div>
        </template>

        <div v-else class="denied">
          <div class="dt">权限不足</div>
          <p>对方还没有邀请你，或者邀请的范围不包含「执行指令」。</p>
          <p class="tiny">等对方在「协助」页发出邀请并选择「允许在我设备上执行指令」，这里才会解锁。</p>
          <div class="shellcard">
            <i class="sdot"></i>
            <span class="mono">{{ shellLabel }}</span>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.mask {
  position: fixed;
  inset: 0;
  z-index: 90;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgba(4, 6, 11, 0.5);
  backdrop-filter: blur(8px);
  animation: fade-up 0.18s var(--ease-out);
}

.sheet {
  width: min(560px, 94vw);
  max-height: 86vh;
  display: flex;
  flex-direction: column;
  border-radius: var(--r-lg);
  overflow: hidden;
  animation: pop-in 0.3s var(--ease-spring);
}

header {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 16px 18px 10px;
}
.htitle h3 {
  font-size: 15px;
  font-weight: 650;
}
.hsub {
  display: block;
  margin-top: 2px;
  font-size: 10.5px;
  color: var(--ink-3);
}
.x {
  margin-left: auto;
  width: 28px;
  height: 28px;
  display: grid;
  place-items: center;
  border-radius: 8px;
  color: var(--ink-3);
  transition: background 0.16s, color 0.16s;
}
.x svg {
  width: 16px;
  height: 16px;
}
.x:hover {
  background: rgba(255, 255, 255, 0.1);
  color: var(--ink-1);
}

.tabs {
  display: flex;
  gap: 4px;
  padding: 0 18px 10px;
}
.tabs button {
  padding: 5px 13px;
  font-size: 12px;
  border-radius: var(--r-pill);
  color: var(--ink-3);
  border: 1px solid transparent;
  transition: all 0.18s;
}
.tabs button.on {
  color: var(--ink-1);
  background: rgba(255, 255, 255, 0.09);
  border-color: var(--line-2);
}

.pane {
  padding: 4px 18px 18px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.offline {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 34px 18px 44px;
  color: var(--ink-3);
}
.offline svg {
  width: 38px;
  height: 38px;
  margin-bottom: 4px;
  color: var(--ink-4);
}
.offline p {
  font-size: 13px;
  color: var(--ink-2);
}
.offline span {
  font-size: 11px;
  color: var(--ink-4);
}

/* ---- 状态盒 ---- */
.state {
  padding: 12px;
  border-radius: var(--r-md);
  border: 1px solid var(--line-1);
  background: rgba(255, 255, 255, 0.04);
  animation: fade-up 0.3s var(--ease-out) both;
}
.okbox {
  border-color: rgba(94, 234, 212, 0.28);
  background: rgba(94, 234, 212, 0.07);
}
.sh {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 7px;
  font-size: 12.5px;
  font-weight: 600;
}
.dotg,
.dotc {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: #4ade80;
  box-shadow: 0 0 8px rgba(74, 222, 128, 0.8);
}
.dotc {
  background: var(--hue-c);
  box-shadow: 0 0 8px rgba(94, 234, 212, 0.8);
}
.tags {
  display: flex;
  gap: 4px;
}
.sm {
  margin-top: 5px;
  font-size: 11px;
  line-height: 1.6;
  color: var(--ink-3);
}
.rowbtns {
  display: flex;
  gap: 6px;
  margin-top: 9px;
}
.btn.small {
  padding: 5px 11px;
  font-size: 11.5px;
}
.btn.danger {
  color: #fda4af;
  border-color: rgba(244, 63, 94, 0.3);
  background: rgba(244, 63, 94, 0.1);
}
.btn.danger:hover {
  background: rgba(244, 63, 94, 0.2);
}
.btn.full {
  width: 100%;
  justify-content: center;
  padding: 10px;
}

/* ---- 邀请 ---- */
.invite h4 {
  margin: 2px 0 4px;
  font-size: 12px;
  font-weight: 620;
  color: var(--ink-2);
  letter-spacing: 0.04em;
}

.chk {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 10px 12px;
  margin-top: 7px;
  border-radius: var(--r-md);
  border: 1px solid var(--line-1);
  background: rgba(255, 255, 255, 0.03);
  cursor: pointer;
  transition: border-color 0.2s, background 0.2s;
}
.chk.on {
  border-color: rgba(167, 139, 250, 0.42);
  background: rgba(167, 139, 250, 0.09);
}
.chk input {
  display: none;
}
.cbox {
  width: 16px;
  height: 16px;
  flex: 0 0 16px;
  margin-top: 2px;
  border-radius: 5px;
  border: 1.5px solid var(--line-2);
  position: relative;
  transition: all 0.18s var(--ease-spring);
}
.chk.on .cbox {
  background: linear-gradient(135deg, var(--hue-a), var(--hue-b));
  border-color: transparent;
}
.chk.on .cbox::after {
  content: "";
  position: absolute;
  left: 4.6px;
  top: 1.6px;
  width: 4px;
  height: 8px;
  border: solid #12101c;
  border-width: 0 2px 2px 0;
  transform: rotate(42deg);
}
.ctext strong {
  display: block;
  font-size: 12.5px;
  font-weight: 600;
}
.ctext em {
  display: block;
  margin-top: 2px;
  font-size: 10.5px;
  font-style: normal;
  color: var(--ink-3);
  line-height: 1.5;
}
.note {
  margin-top: 9px;
  font-size: 12.5px;
}

.warnbox {
  display: flex;
  gap: 8px;
  padding: 10px 12px;
  border-radius: var(--r-sm);
  background: rgba(251, 191, 36, 0.08);
  border: 1px solid rgba(251, 191, 36, 0.22);
}
.warnbox svg {
  width: 16px;
  height: 16px;
  flex: 0 0 16px;
  margin-top: 2px;
  color: #fbbf24;
}
.warnbox p {
  font-size: 10.5px;
  line-height: 1.6;
  color: rgba(251, 191, 36, 0.9);
}

/* ---- 终端 ---- */
.shellinfo {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 9px 12px;
  border-radius: var(--r-sm);
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--line-1);
}
.shellinfo.ok {
  background: rgba(94, 234, 212, 0.07);
  border-color: rgba(94, 234, 212, 0.24);
}
.sdot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--ink-4);
}
.shellinfo.ok .sdot {
  background: var(--hue-c);
  box-shadow: 0 0 7px rgba(94, 234, 212, 0.8);
}
.slabel {
  font-size: 11px;
  color: var(--ink-3);
}
.sval {
  margin-left: auto;
  font-size: 11px;
  color: var(--ink-2);
}

.cmdrow {
  display: flex;
  align-items: center;
  gap: 8px;
}
.prompt {
  font-size: 15px;
  color: var(--hue-a);
}
.cmdrow .field {
  flex: 1;
  padding: 9px 12px;
  font-size: 12.5px;
}

.quick {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}
.chip {
  padding: 3px 9px;
  font-size: 10.5px;
  border-radius: var(--r-pill);
  color: var(--ink-2);
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid var(--line-1);
  transition: all 0.16s;
}
.chip:hover {
  background: rgba(167, 139, 250, 0.16);
  border-color: rgba(167, 139, 250, 0.4);
}

.denied {
  padding: 22px 16px;
  text-align: center;
  border-radius: var(--r-md);
  border: 1px dashed var(--line-2);
  background: rgba(255, 255, 255, 0.02);
}
.dt {
  font-size: 15px;
  font-weight: 650;
  color: var(--ink-2);
  letter-spacing: 0.04em;
}
.denied p {
  margin-top: 6px;
  font-size: 11.5px;
  line-height: 1.6;
  color: var(--ink-3);
}
.denied .tiny {
  font-size: 10.5px;
  color: var(--ink-4);
}
.shellcard {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin-top: 14px;
  padding: 5px 11px;
  border-radius: var(--r-pill);
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--line-1);
  font-size: 10.5px;
  color: var(--ink-3);
}
</style>
