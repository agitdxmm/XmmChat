/**
 * 语音 / 视频通话（WebRTC）。
 *
 * 信令复用同一条 WebSocket：call-offer / call-answer / call-ice / call-end。
 * 局域网内不需要 STUN —— host candidate 就能直连。
 * 若要跨公网，把 ICE_SERVERS 换成一个可用的 STUN/TURN。
 */

import { ref } from "vue";
import * as bridge from "./bridge";
import { frame } from "./protocol";
import { pushMessage, state, toast } from "./store";

const ICE_SERVERS: RTCIceServer[] = [];

export const remoteStream = ref<MediaStream | null>(null);
export const localStream = ref<MediaStream | null>(null);
export const micOn = ref(true);
export const camOn = ref(true);

let pc: RTCPeerConnection | null = null;
const pending: RTCIceCandidateInit[] = [];
let startedAt = 0;

function send(t: string, d?: unknown) {
  void bridge.send(JSON.stringify(frame(t, d))).catch(() => {});
}

function newPc(): RTCPeerConnection {
  const p = new RTCPeerConnection({ iceServers: ICE_SERVERS });

  p.onicecandidate = (e) => {
    if (e.candidate) send("call-ice", { candidate: e.candidate.toJSON() });
  };

  p.ontrack = (e) => {
    remoteStream.value = e.streams[0] ?? null;
  };

  p.onconnectionstatechange = () => {
    if (!p) return;
    if (p.connectionState === "connected" && state.call.phase !== "active") {
      state.call.phase = "active";
      state.call.startedAt = Date.now();
      startedAt = Date.now();
    }
    if (p.connectionState === "failed") {
      toast("通话连接失败，可能是网络不通", "err");
      cleanup(true);
    }
  };

  return p;
}

async function flush() {
  if (!pc) return;
  while (pending.length) {
    const c = pending.shift()!;
    try {
      await pc.addIceCandidate(c);
    } catch {
      /* 过期的 candidate 忽略 */
    }
  }
}

/* ---------------- 发起 ---------------- */

export async function startCall(video: boolean) {
  if (!state.link.online) {
    toast("还没有连上对方", "warn");
    return;
  }
  if (state.call.phase !== "idle" && state.call.phase !== "ended") return;

  state.call.phase = "outing";
  state.call.video = video;
  state.call.incomingVideo = video;

  try {
    const stream = await navigator.mediaDevices.getUserMedia({
      audio: true,
      video: video ? { width: { ideal: 1280 }, height: { ideal: 720 } } : false,
    });
    localStream.value = stream;
    micOn.value = true;
    camOn.value = video;

    pc = newPc();
    stream.getTracks().forEach((t) => pc!.addTrack(t, stream));

    const offer = await pc.createOffer();
    await pc.setLocalDescription(offer);
    send("call-offer", { video, sdp: offer.sdp });
  } catch (e: any) {
    state.call.phase = "idle";
    toast(`无法开始通话：${e?.message || e}`, "err");
  }
}

/* ---------------- 接听 ---------------- */

export async function acceptCall() {
  const video = state.call.incomingVideo;
  try {
    const stream = await navigator.mediaDevices.getUserMedia({
      audio: true,
      video: video ? { width: { ideal: 1280 }, height: { ideal: 720 } } : false,
    });
    localStream.value = stream;
    micOn.value = true;
    camOn.value = video;

    if (!pc) pc = newPc();
    stream.getTracks().forEach((t) => pc!.addTrack(t, stream));

    const answer = await pc.createAnswer();
    await pc.setLocalDescription(answer);
    send("call-answer", { sdp: answer.sdp });

    state.call.phase = "active";
    state.call.startedAt = Date.now();
    startedAt = Date.now();
    await flush();
  } catch (e: any) {
    toast(`接听失败：${e?.message || e}`, "err");
    rejectCall();
  }
}

export function rejectCall() {
  send("call-end", { reason: "declined" });
  cleanup(false, true);
}

/* ---------------- 挂断 ---------------- */

export function hangup() {
  send("call-end", { reason: "hangup" });
  cleanup(true);
}

function cleanup(log: boolean, missed = false) {
  const wasVideo = state.call.video || state.call.incomingVideo;
  const seconds = startedAt ? (Date.now() - startedAt) / 1000 : 0;

  localStream.value?.getTracks().forEach((t) => t.stop());
  remoteStream.value?.getTracks().forEach((t) => t.stop());
  localStream.value = null;
  remoteStream.value = null;

  if (pc) {
    pc.onicecandidate = null;
    pc.ontrack = null;
    pc.onconnectionstatechange = null;
    pc.close();
    pc = null;
  }
  pending.length = 0;

  if (log) {
    pushMessage({
      id: `call-${Date.now()}`,
      from: "me",
      kind: "call",
      ts: Date.now(),
      call: { video: wasVideo, seconds, missed },
    });
  }

  state.call.phase = "idle";
  state.call.startedAt = 0;
  startedAt = 0;
}

/* ---------------- 信令入口 ---------------- */

export async function handleCallSignal(t: string, d: any) {
  switch (t) {
    case "call-offer": {
      // 正在通话中就不再接第二路
      if (state.call.phase === "active") {
        send("call-end", { reason: "busy" });
        return;
      }
      try {
        pc = newPc();
        await pc.setRemoteDescription({ type: "offer", sdp: d.sdp });
        state.call.incomingVideo = !!d.video;
        state.call.video = !!d.video;
        state.call.phase = "incoming";
        await flush();
      } catch (e: any) {
        toast(`来电处理失败：${e?.message || e}`, "err");
        cleanup(false);
      }
      break;
    }

    case "call-answer": {
      if (!pc) return;
      try {
        await pc.setRemoteDescription({ type: "answer", sdp: d.sdp });
        state.call.phase = "active";
        state.call.startedAt = Date.now();
        startedAt = Date.now();
        await flush();
      } catch (e: any) {
        toast(`应答失败：${e?.message || e}`, "err");
      }
      break;
    }

    case "call-ice": {
      if (!pc) return;
      if (!pc.remoteDescription) {
        pending.push(d.candidate);
        return;
      }
      try {
        await pc.addIceCandidate(d.candidate);
      } catch {
        /* ignore */
      }
      break;
    }

    case "call-end": {
      const wasActive = state.call.phase === "active";
      const missed = !wasActive && state.call.phase === "incoming";
      if (state.call.phase !== "idle") cleanup(wasActive, missed);
      break;
    }

    default:
      break;
  }
}

/* ---------------- 设备开关 ---------------- */

export function toggleMic() {
  const t = localStream.value?.getAudioTracks()[0];
  if (!t) return;
  t.enabled = !t.enabled;
  micOn.value = t.enabled;
}

export function toggleCam() {
  const t = localStream.value?.getVideoTracks()[0];
  if (!t) return;
  t.enabled = !t.enabled;
  camOn.value = t.enabled;
}
