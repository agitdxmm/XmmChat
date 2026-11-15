/**
 * 协议引擎：把 WebSocket 上的字节翻译成界面上的消息。
 *
 * 安全模型（务必保持）：
 *   · 远程抓屏 / 远程执行指令，只有「本机用户点过同意」之后才会响应。
 *   · 同意是**对端发起邀请 → 本机用户确认**，不是控制方单方面决定。
 *   · 断链或对方撤销，授权立刻失效。
 */

import { ref } from "vue";
import * as bridge from "./bridge";
import {
  applyTheme,
  applyVars,
  loadSettings,
  newId,
  pushMessage,
  state,
  systemNote,
  toast,
  type AssistScope,
  type Message,
  type Transfer,
} from "./store";
import { frame, PROTO_VERSION, type FileMetaData, type AssistInviteData } from "./protocol";
import { handleCallSignal } from "./call";

const CHUNK = 64 * 1024;

/* ----------------------------- 待处理邀请 ----------------------------- */

export type PendingInvite = {
  scopes: AssistScope[];
  note: string;
};

/** 收到的协助邀请；非 null 时界面弹出确认卡片 */
export const pendingInvite = ref<PendingInvite | null>(null);

/* ----------------------------- 接收缓冲 ----------------------------- */

type Inbox = {
  meta: FileMetaData & { voice?: number };
  parts: Uint8Array[];
  got: number;
  msg: Message;
};

const inbox = new Map<string, Inbox>();

/* ----------------------------- 编解码 ----------------------------- */

function toB64(buf: ArrayBuffer): string {
  const bytes = new Uint8Array(buf);
  let s = "";
  const step = 0x8000;
  for (let i = 0; i < bytes.length; i += step) {
    s += String.fromCharCode(...bytes.subarray(i, i + step));
  }
  return btoa(s);
}

function fromB64(b64: string): Uint8Array {
  const raw = atob(b64);
  const out = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) out[i] = raw.charCodeAt(i);
  return out;
}

/* ----------------------------- 发送 ----------------------------- */

export function sendFrame<T>(t: string, d?: T) {
  const f = frame(t, d);
  void bridge.send(JSON.stringify(f)).catch(() => {
    /* 断链时静默 */
  });
  return f.id!;
}

/* ----------------------------- 启动 ----------------------------- */

let booted = false;

export async function boot() {
  if (booted) return;
  booted = true;

  loadSettings();
  applyTheme();
  applyVars();

  const host = await bridge.hostInfo();
  state.host = host;
  state.me.platform = host.platform;
  state.me.name = state.settings.myName || host.hostname || "我";

  state.shell = await bridge.shellProbe();

  await bridge.onLink((e) => {
    state.link.state = e.state;
    state.link.online = e.state === "online";
    state.link.peer = e.peer;
    state.link.role = e.role;

    if (e.state === "online") {
      state.me.id = newId("u");
      sendFrame("hello", {
        v: PROTO_VERSION,
        name: state.settings.myName || state.host.hostname || "对方",
        platform: state.me.platform,
        caps: ["text", "file", "voice", "call", "video", "screen", "exec", "assist", state.shell.backend !== "none" ? "shell" : ""].filter(Boolean),
      });
      systemNote(`已与 ${e.peer} 建立链路`);
    }

    if (e.state === "closed") {
      systemNote("链路已断开");
      revokeAll();
    }
    if (e.state === "error") {
      systemNote(`链路异常：${e.msg}`);
      revokeAll();
      toast(e.msg || "连接失败", "err");
    }
  });

  await bridge.onData(handleText);
  await bridge.onBin(async (b64) => {
    // 二进制帧预留：目前大文件走 JSON+base64，这里保持兼容
    void b64;
  });

  state.booted = true;
}

function revokeAll() {
  state.link.online = false;
  state.granted = [];
  state.holding = [];
  state.monitor.on = false;
  state.typing = false;
  state.peer = { name: "", platform: "", caps: [] };
  pendingInvite.value = null;
}

/* ----------------------------- 收帧 ----------------------------- */

function handleText(text: string) {
  let f: any;
  try {
    f = JSON.parse(text);
  } catch {
    return;
  }
  if (!f || typeof f.t !== "string") return;
  const d = f.d ?? {};

  switch (f.t) {
    case "hello": {
      state.peer.name = d.name || "对方";
      state.peer.platform = d.platform || "unknown";
      state.peer.caps = Array.isArray(d.caps) ? d.caps : [];
      systemNote(`${state.peer.name} 已接入 · ${platformLabel(state.peer.platform)}`);
      break;
    }

    case "text": {
      pushMessage({
        id: f.id || newId("p"),
        from: "peer",
        kind: "text",
        ts: f.ts || Date.now(),
        text: String(d.body ?? ""),
      });
      state.unread++;
      sendFrame("ack", { ids: [f.id] });
      break;
    }

    case "typing":
      state.typing = !!d.on;
      break;

    case "ack": {
      const ids: string[] = Array.isArray(d.ids) ? d.ids : [];
      for (const m of state.messages) {
        if (ids.includes(m.id)) m.pending = false;
      }
      break;
    }

    case "file-meta":
      onFileMeta(f, d);
      break;

    case "file-chunk":
      onFileChunk(d);
      break;

    case "file-done":
      onFileDone(d);
      break;

    case "assist-invite": {
      const inv = d as AssistInviteData;
      pendingInvite.value = {
        scopes: (inv.scopes || []).filter((s) => s === "screen" || s === "exec"),
        note: inv.note || "",
      };
      break;
    }

    case "assist-reply": {
      if (d.accept) {
        state.holding = (d.scopes || []).filter((s: string) => s === "screen" || s === "exec");
        systemNote(`对方已同意协助：${scopeLabel(state.holding)}`);
        toast("协助已建立", "ok");
      } else {
        state.holding = [];
        systemNote("对方拒绝了协助邀请");
        toast("对方拒绝了邀请", "warn");
      }
      break;
    }

    case "assist-end": {
      state.granted = [];
      state.holding = [];
      state.monitor.on = false;
      systemNote("协助会话已结束");
      break;
    }

    case "exec-req": {
      void handleExecRequest(d.rid, d.cmd);
      break;
    }

    case "exec-res": {
      const m = state.messages.find((x) => x.id === `exec-${d.rid}`);
      if (m) {
        m.exec = {
          cmd: m.exec?.cmd || "",
          code: d.code ?? -1,
          stdout: d.stdout ?? "",
          stderr: d.stderr ?? "",
          denied: !!d.denied,
        };
        m.pending = false;
      }
      break;
    }

    case "screen-frame": {
      const m = state.messages[state.messages.length - 1];
      const url = `data:image/png;base64,${d.png}`;
      if (m && m.kind === "screen") {
        m.screen = { url };
        m.ts = Date.now();
      } else {
        pushMessage({
          id: newId("sc"),
          from: "peer",
          kind: "screen",
          ts: Date.now(),
          screen: { url },
        });
      }
      break;
    }

    case "screen-stop": {
      state.monitor.on = false;
      systemNote("对方停止了屏幕查看");
      break;
    }

    case "sys": {
      systemNote(String(d.text ?? ""));
      break;
    }

    case "call-offer":
    case "call-answer":
    case "call-ice":
    case "call-end": {
      void handleCallSignal(f.t, d);
      break;
    }

    default:
      break;
  }
}

function platformLabel(p: string): string {
  const map: Record<string, string> = {
    windows: "Windows",
    android: "Android",
    linux: "Linux",
    macos: "macOS",
    web: "浏览器",
  };
  return map[p] || p || "未知设备";
}

function scopeLabel(s: AssistScope[]): string {
  if (!s.length) return "无";
  return s.map((x) => (x === "screen" ? "查看屏幕" : "执行指令")).join(" + ");
}

/* ----------------------------- 文件接收 ----------------------------- */

function onFileMeta(f: any, d: FileMetaData & { voice?: number }) {
  const isVoice = typeof d.voice === "number";
  const transfer: Transfer = {
    xid: d.xid,
    name: d.name || "未命名",
    size: d.size || 0,
    mime: d.mime || "application/octet-stream",
    progress: 0,
    status: "receiving",
    dir: "in",
  };

  const msg: Message = {
    id: `x-${d.xid}`,
    from: "peer",
    kind: isVoice ? "voice" : guessKind(d.mime, d.name),
    ts: f.ts || Date.now(),
    file: isVoice ? undefined : transfer,
    voice: isVoice
      ? { duration: d.voice!, size: d.size || 0, status: "receiving" }
      : undefined,
    text: isVoice ? undefined : undefined,
  };
  pushMessage(msg);

  inbox.set(d.xid, {
    meta: d,
    parts: [],
    got: 0,
    msg,
  });
}

function onFileChunk(d: { xid: string; i: number; b: string }) {
  const entry = inbox.get(d.xid);
  if (!entry) return;
  try {
    entry.parts.push(fromB64(d.b));
  } catch {
    return;
  }
  entry.got++;
  const total = Math.max(1, entry.meta.chunks || 1);
  const p = Math.min(1, entry.got / total);
  if (entry.msg.file) entry.msg.file.progress = p;
  if (entry.msg.voice) entry.msg.voice.status = "receiving";
}

function onFileDone(d: { xid: string }) {
  const entry = inbox.get(d.xid);
  if (!entry) return;
  const blob = new Blob(entry.parts as BlobPart[], {
    type: entry.meta.mime || "application/octet-stream",
  });
  const url = URL.createObjectURL(blob);

  if (entry.msg.file) {
    entry.msg.file.url = url;
    entry.msg.file.progress = 1;
    entry.msg.file.status = "done";
  }
  if (entry.msg.voice) {
    entry.msg.voice.url = url;
    entry.msg.voice.status = "done";
  }
  inbox.delete(d.xid);
}

function guessKind(mime: string, name: string): Message["kind"] {
  if (mime.startsWith("image/")) return "image";
  if (mime.startsWith("video/")) return "video";
  if (/\.(png|jpe?g|gif|webp|bmp|avif)$/i.test(name)) return "image";
  if (/\.(mp4|webm|mov|mkv)$/i.test(name)) return "video";
  return "file";
}

/* ----------------------------- 发送：文本 ----------------------------- */

export function sendText(body: string) {
  const text = body.trim();
  if (!text) return;
  if (!state.link.online) {
    toast("还没有连上对方", "warn");
    return;
  }
  const id = sendFrame("text", { body: text });
  pushMessage({
    id,
    from: "me",
    kind: "text",
    ts: Date.now(),
    text,
    pending: true,
  });
}

export function sendTyping(on: boolean) {
  if (!state.link.online) return;
  sendFrame("typing", { on });
}

/* ----------------------------- 发送：文件 ----------------------------- */

export async function sendFile(file: File, asVoice = false, duration = 0) {
  if (!state.link.online) {
    toast("还没有连上对方", "warn");
    return;
  }
  const xid = newId("x");
  const chunks = Math.max(1, Math.ceil(file.size / CHUNK));

  const msg: Message = {
    id: `x-${xid}`,
    from: "me",
    kind: asVoice ? "voice" : guessKind(file.type, file.name),
    ts: Date.now(),
    file: asVoice
      ? undefined
      : {
          xid,
          name: file.name,
          size: file.size,
          mime: file.type || "application/octet-stream",
          progress: 0,
          status: "sending",
          dir: "out",
        },
    voice: asVoice
      ? { duration, size: file.size, status: "sending" }
      : undefined,
  };
  pushMessage(msg);

  sendFrame("file-meta", {
    xid,
    name: file.name,
    size: file.size,
    mime: file.type || "application/octet-stream",
    chunks,
    voice: asVoice ? duration : undefined,
  });

  for (let i = 0; i < chunks; i++) {
    const slice = file.slice(i * CHUNK, (i + 1) * CHUNK);
    const buf = await slice.arrayBuffer();
    sendFrame("file-chunk", { xid, i, b: toB64(buf) });
    const p = (i + 1) / chunks;
    if (msg.file) msg.file.progress = p;
    // 每 8 块让一次事件循环，避免长任务把界面冻住
    if (i % 8 === 7) {
      await new Promise((r) => setTimeout(r, 0));
    }
  }

  sendFrame("file-done", { xid });
  if (msg.file) {
    msg.file.progress = 1;
    msg.file.status = "done";
  }
  if (msg.voice) msg.voice.status = "done";
}

/* ----------------------------- 协助邀请 ----------------------------- */

export function inviteAssist(scopes: AssistScope[], note: string) {
  if (!state.link.online) {
    toast("还没有连上对方", "warn");
    return;
  }
  sendFrame("assist-invite", { scopes, note });
  systemNote(`已向 ${state.peer.name || "对方"} 发出协助邀请：${scopeLabel(scopes)}`);
}

export function replyInvite(accept: boolean) {
  const inv = pendingInvite.value;
  if (!inv) return;
  sendFrame("assist-reply", {
    accept,
    scopes: accept ? inv.scopes : [],
  });
  state.granted = accept ? [...inv.scopes] : [];
  if (accept) {
    systemNote(`已授权对方：${scopeLabel(inv.scopes)}`);
    toast("已授权", "ok");
  } else {
    systemNote("已拒绝协助邀请");
  }
  pendingInvite.value = null;
}

export function revokeAssist() {
  sendFrame("assist-end");
  state.granted = [];
  state.holding = [];
  state.monitor.on = false;
  systemNote("已结束协助会话");
}

/* ----------------------------- 远程指令 ----------------------------- */

const execWaiters = new Map<string, (v: any) => void>();

export async function requestExec(cmd: string) {
  if (!state.holding.includes("exec")) {
    toast("对方还没有授权执行指令", "warn");
    return;
  }
  const rid = newId("r");
  pushMessage({
    id: `exec-${rid}`,
    from: "me",
    kind: "exec",
    ts: Date.now(),
    exec: { cmd, code: 0, stdout: "", stderr: "" },
    pending: true,
  });
  sendFrame("exec-req", { rid, cmd });
}

async function handleExecRequest(rid: string, cmd: string) {
  if (!state.granted.includes("exec")) {
    sendFrame("exec-res", {
      rid,
      code: -1,
      stdout: "",
      stderr: "",
      timedOut: false,
      denied: true,
      reason: "本机未授权执行指令",
    });
    return;
  }

  pushMessage({
    id: newId("ex"),
    from: "peer",
    kind: "exec",
    ts: Date.now(),
    exec: { cmd, code: 0, stdout: "执行中…", stderr: "" },
    pending: true,
  });

  try {
    const r = await bridge.shellRun(cmd, true);
    sendFrame("exec-res", {
      rid,
      code: r.code,
      stdout: r.stdout,
      stderr: r.stderr,
      timedOut: r.timed_out,
    });
  } catch (e: any) {
    sendFrame("exec-res", {
      rid,
      code: -1,
      stdout: "",
      stderr: "",
      timedOut: false,
      denied: true,
      reason: String(e?.message || e),
    });
  }
}

/* ----------------------------- 屏幕流 ----------------------------- */

let screenTimer: number | null = null;

export function startScreenShare(fps = 2) {
  if (!state.granted.includes("screen")) {
    toast("尚未授权屏幕查看", "warn");
    return;
  }
  if (screenTimer !== null) return;
  state.monitor.on = true;
  const interval = Math.max(200, Math.round(1000 / Math.max(0.5, fps)));

  const tick = async () => {
    if (!state.monitor.on || !state.granted.includes("screen")) {
      stopScreenShare(true);
      return;
    }
    try {
      const png = await bridge.screenGrab(1280);
      sendFrame("screen-frame", { png });
    } catch {
      /* 抓屏偶发失败不打断 */
    }
    if (state.monitor.on) screenTimer = window.setTimeout(tick, interval);
  };
  tick();
}

export function stopScreenShare(notifyPeer = true) {
  state.monitor.on = false;
  if (screenTimer !== null) {
    window.clearTimeout(screenTimer);
    screenTimer = null;
  }
  if (notifyPeer && state.link.online) sendFrame("screen-stop", {});
}

/* ----------------------------- 断链 ----------------------------- */

export async function disconnect() {
  await bridge.hangup();
  revokeAll();
  state.link.state = "closed";
  state.link.online = false;
}

export async function connect(addr: string) {
  try {
    await bridge.dial(addr);
  } catch (e: any) {
    toast(String(e?.message || e), "err");
  }
}
