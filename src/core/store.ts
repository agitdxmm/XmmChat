/**
 * 全局状态与本地偏好。
 *
 * 只放「数据」和「纯函数式修改」，网络与协议逻辑在 engine.ts。
 */

import { reactive, watch } from "vue";

export type ThemeMode = "dark" | "light";

export type MsgKind =
  | "text"
  | "file"
  | "voice"
  | "image"
  | "video"
  | "system"
  | "call"
  | "exec"
  | "screen";

export type Transfer = {
  xid: string;
  name: string;
  size: number;
  mime: string;
  progress: number; // 0..1
  url?: string;
  status: "sending" | "receiving" | "done" | "failed";
  dir: "in" | "out";
};

export type Message = {
  id: string;
  from: "me" | "peer" | "sys";
  kind: MsgKind;
  ts: number;
  text?: string;
  file?: Transfer;
  voice?: { duration: number; url?: string; size: number; status: Transfer["status"] };
  call?: { video: boolean; seconds: number; missed?: boolean; dir?: "in" | "out" };
  exec?: { cmd: string; code: number; stdout: string; stderr: string; denied?: boolean };
  screen?: { url: string };
  pending?: boolean;
};

export type Settings = {
  theme: ThemeMode;
  /** 聊天背景图（data URL），空串表示不用图 */
  wallpaper: string;
  wallpaperBlur: number;
  wallpaperDim: number;
  glassBlur: number;
  bubbleAlpha: number;
  density: "cozy" | "compact";
  sound: boolean;
  myName: string;
};

const DEFAULT_SETTINGS: Settings = {
  theme: "dark",
  wallpaper: "",
  wallpaperBlur: 18,
  wallpaperDim: 0.42,
  glassBlur: 25,
  bubbleAlpha: 0.1,
  density: "cozy",
  sound: true,
  myName: "小喵咪",
};

export type AssistScope = "screen" | "exec";

export const state = reactive({
  booted: false,

  me: { name: "我", platform: "unknown", id: "" },
  host: {
    hostname: "",
    platform: "",
    arch: "",
    lan_ips: [] as string[],
    port: 19779,
  },

  link: {
    state: "idle" as "idle" | "listening" | "connecting" | "online" | "closed" | "error",
    /** 是否已握手完成（state === "online" 的便捷镜像） */
    online: false,
    peer: "",
    role: "",
  },
  peer: { name: "", platform: "", caps: [] as string[] },

  messages: [] as Message[],
  typing: false,
  unread: 0,

  settings: { ...DEFAULT_SETTINGS } as Settings,

  /** 我作为「被协助方」已经授予对方的范围（邀请制：这是我自己点同意的） */
  granted: [] as AssistScope[],
  /** 我作为「协助方」被对端授予的范围 */
  holding: [] as AssistScope[],
  assistNote: "",

  call: {
    phase: "idle" as "idle" | "outing" | "incoming" | "active" | "ended",
    video: false,
    startedAt: 0,
    incomingVideo: false,
  },

  shell: {
    backend: "none",
    detail: "检测中…",
    /** 安卓：无障碍服务是否已连接 */
    a11y: false,
    /** 安卓：Shizuku 是否可用且已授权 */
    shizuku: false,
    /** 安卓：是否 root */
    root: false,
  },

  monitor: { on: false, fps: 2 },

  toasts: [] as { id: number; text: string; level: "info" | "warn" | "err" | "ok" }[],
});

/* ----------------------------- 设置持久化 ----------------------------- */

const KEY = "xmmchat.settings.v1";

export function loadSettings() {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return;
    const parsed = JSON.parse(raw) as Partial<Settings>;
    Object.assign(state.settings, DEFAULT_SETTINGS, parsed);
  } catch {
    /* 坏数据就当没有 */
  }
}

export function saveSettings() {
  try {
    const data = { ...state.settings };
    // 背景图可能有几 MB，单独存，失败就丢弃这张图而不是拖垮全部设置
    try {
      localStorage.setItem(KEY, JSON.stringify(data));
    } catch {
      const slim = { ...data, wallpaper: "" };
      localStorage.setItem(KEY, JSON.stringify(slim));
      toast("背景图太大，本次未保存", "warn");
    }
  } catch {
    /* 存储不可用就别烦用户了 */
  }
}

watch(
  () => state.settings,
  () => {
    saveSettings();
    applyTheme();
    applyVars();
  },
  { deep: true }
);

/* ----------------------------- 主题 / 变量 ----------------------------- */

export function applyTheme() {
  const root = document.documentElement;
  root.dataset.theme = state.settings.theme;
}

export function applyVars() {
  const root = document.documentElement;
  root.style.setProperty("--glass-blur", `${state.settings.glassBlur}px`);
  root.style.setProperty("--glass-alpha", String(state.settings.bubbleAlpha));
  root.style.setProperty(
    "--wallpaper-blur",
    `${state.settings.wallpaperBlur}px`
  );
  root.style.setProperty("--wallpaper-dim", String(state.settings.wallpaperDim));
}

/* ----------------------------- 消息工具 ----------------------------- */

let localSeq = 0;
export function newId(prefix = "m"): string {
  localSeq = (localSeq + 1) % 1_000_000;
  return `${prefix}${Date.now().toString(36)}${localSeq.toString(36)}`;
}

export function pushMessage(m: Message) {
  state.messages.push(m);
  if (state.messages.length > 3000) {
    state.messages.splice(0, state.messages.length - 3000);
  }
}

export function systemNote(text: string) {
  pushMessage({
    id: newId("s"),
    from: "sys",
    kind: "system",
    ts: Date.now(),
    text,
  });
}

export function toast(text: string, level: "info" | "warn" | "err" | "ok" = "info") {
  const id = Date.now() + Math.random();
  state.toasts.push({ id, text, level });
  window.setTimeout(() => {
    const i = state.toasts.findIndex((t) => t.id === id);
    if (i >= 0) state.toasts.splice(i, 1);
  }, 3600);
}

/* ----------------------------- 格式化 ----------------------------- */

export function fmtSize(n: number): string {
  if (!Number.isFinite(n) || n < 0) return "—";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  let v = n;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v < 10 && i > 0 ? v.toFixed(1) : Math.round(v)} ${units[i]}`;
}

export function fmtClock(ts: number): string {
  const d = new Date(ts);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

export function fmtDuration(sec: number): string {
  const s = Math.max(0, Math.round(sec));
  const m = Math.floor(s / 60);
  const r = s % 60;
  return `${m}:${String(r).padStart(2, "0")}`;
}
