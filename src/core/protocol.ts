/**
 * 应用层协议。
 *
 * 传输层（Rust / WebSocket）只搬字符串，所有语义都在这里。
 * 每个帧都是一个 JSON 对象：
 *   { t: 类型, id: 帧号, ts: 时间戳, d: 载荷 }
 */

export const PROTO_VERSION = 1;

export type Frame<T = unknown> = {
  t: string;
  id?: string;
  ts?: number;
  d?: T;
};

/* ---------------- 握手 ---------------- */

export type HelloData = {
  v: number;
  name: string;
  platform: "windows" | "android" | "linux" | "macos" | string;
  caps: Capability[];
};

export type Capability =
  | "text"
  | "file"
  | "voice"
  | "call"
  | "video"
  | "screen"
  | "exec"
  | "assist"
  | "shell";

/* ---------------- 消息 ---------------- */

export type TextData = {
  /** 回复的目标消息 id */
  re?: string;
  body: string;
};

export type AckData = {
  /** 已读/已送达的帧 id 列表 */
  ids: string[];
};

export type TypingData = {
  on: boolean;
};

/* ---------------- 文件 ---------------- */

export type FileMetaData = {
  xid: string; // 传输会话 id
  name: string;
  size: number;
  mime: string;
  chunks: number;
};

export type FileChunkData = {
  xid: string;
  i: number;
  /** base64 块 */
  b: string;
};

export type FileDoneData = {
  xid: string;
};

/* ---------------- 语音 ---------------- */

export type VoiceData = {
  xid: string;
  duration: number; // 秒
  mime: string;
  size: number;
};

/* ---------------- 通话（WebRTC 信令） ---------------- */

export type CallOfferData = {
  video: boolean;
  sdp: string;
};

export type CallAnswerData = {
  sdp: string;
};

export type CallIceData = {
  candidate: RTCIceCandidateInit;
};

export type CallEndData = {
  reason: string;
};

/* ---------------- 协助（邀请制） ---------------- */

export type AssistInviteData = {
  /** 请求的能力 */
  scopes: AssistScope[];
  /** 邀请方备注 */
  note: string;
};

export type AssistScope = "screen" | "exec";

export type AssistReplyData = {
  accept: boolean;
  scopes: AssistScope[];
  note?: string;
};

/* ---------------- 远程指令 ---------------- */

export type ExecReqData = {
  rid: string;
  cmd: string;
};

export type ExecResData = {
  rid: string;
  code: number;
  stdout: string;
  stderr: string;
  timedOut: boolean;
  denied?: boolean;
  reason?: string;
};

/* ---------------- 屏幕帧 ---------------- */

export type ScreenFrameData = {
  /** base64 PNG */
  png: string;
  w?: number;
  h?: number;
};

/* ---------------- 系统提示 ---------------- */

export type SysData = {
  level: "info" | "warn" | "error";
  text: string;
};

/* ---------------- 信封构造 ---------------- */

let seq = 0;

export function frame<T>(t: string, d?: T): Frame<T> {
  seq = (seq + 1) % 1_000_000;
  return { t, id: `f${Date.now().toString(36)}${seq.toString(36)}`, ts: Date.now(), d };
}
