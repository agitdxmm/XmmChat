/**
 * 原生桥。
 *
 * 桌面端和安卓端走的是**同一套 Tauri IPC**（Tauri 2 的移动端同样用 invoke/event），
 * 所以这里不需要平台分支 —— 真正的平台差异被 Rust 和 Kotlin 插件吸收掉了。
 *
 * 在浏览器里跑（`npm run dev` 直接开页面）时，所有调用降级为本地模拟，
 * 保证 UI 依然能调试。
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export const isTauri =
  typeof window !== "undefined" &&
  ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);

/** 在安卓 WebView 里跑？ */
export const isAndroid =
  typeof navigator !== "undefined" && /android/i.test(navigator.userAgent);

/* ---------------- 类型 ---------------- */

export type HostInfo = {
  hostname: string;
  platform: string;
  arch: string;
  lan_ips: string[];
  port: number;
};

export type LinkState = {
  online: boolean;
  peer: string;
  role: string;
};

export type ShellBackend = "cmd" | "shizuku" | "accessibility" | "none";

export type ShellProbe = {
  backend: ShellBackend | string;
  /** 直接显示给人看的一句话 */
  detail: string;
  /** 安卓：无障碍服务是否已开启 */
  a11y: boolean;
  /** 安卓：Shizuku 是否可用且已授权 */
  shizuku: boolean;
  /** 安卓：是否 root */
  root: boolean;
};

/** 把可能缺字段的原始返回值补齐成完整对象 */
function normProbe(p: Partial<ShellProbe> | null | undefined): ShellProbe {
  return {
    backend: p?.backend ?? "none",
    detail: p?.detail ?? "没有Shell权限的终端",
    a11y: !!p?.a11y,
    shizuku: !!p?.shizuku,
    root: !!p?.root,
  };
}

export type ShellResult = {
  code: number;
  stdout: string;
  stderr: string;
  timed_out: boolean;
};

export type LinkEvent = {
  state: "listening" | "connecting" | "online" | "closed" | "error";
  peer: string;
  role: string;
  msg: string;
};

/* ---------------- 本机 ---------------- */

export async function hostInfo(): Promise<HostInfo> {
  if (!isTauri) {
    return {
      hostname: "浏览器预览",
      platform: "web",
      arch: "wasm",
      lan_ips: ["127.0.0.1"],
      port: 19779,
    };
  }
  return invoke<HostInfo>("host_info");
}

/* ---------------- 链路 ---------------- */

export async function dial(addr: string): Promise<string> {
  if (!isTauri) {
    throw new Error("浏览器预览模式无法建立连接，请运行桌面端");
  }
  return invoke<string>("net_dial", { addr });
}

export async function send(text: string): Promise<void> {
  if (!isTauri) return;
  await invoke("net_send", { text });
}

export async function sendBin(b64: string): Promise<void> {
  if (!isTauri) return;
  await invoke("net_send_bin", { b64 });
}

export async function hangup(): Promise<void> {
  if (!isTauri) return;
  await invoke("net_hangup");
}

export async function linkState(): Promise<LinkState> {
  if (!isTauri) return { online: false, peer: "", role: "" };
  return invoke<LinkState>("net_state");
}

/* ---------------- 事件 ---------------- */

export async function onLink(cb: (e: LinkEvent) => void): Promise<UnlistenFn> {
  if (!isTauri) return () => {};
  return listen<LinkEvent>("xmm://link", (ev) => cb(ev.payload));
}

export async function onData(cb: (text: string) => void): Promise<UnlistenFn> {
  if (!isTauri) return () => {};
  return listen<{ text: string }>("xmm://data", (ev) => cb(ev.payload.text));
}

export async function onBin(cb: (b64: string) => void): Promise<UnlistenFn> {
  if (!isTauri) return () => {};
  return listen<{ b64: string }>("xmm://bin", (ev) => cb(ev.payload.b64));
}

/* ---------------- 原生能力 ---------------- */

/**
 * 探测本机的执行能力。
 *
 * 桌面端由 Rust 直接回答（cmd）。
 * 安卓端必须问 Kotlin 插件：应用自身没有 shell 身份，
 * 只有 Shizuku（拿到 shell 级 UID）或 root 才能执行命令；
 * 无障碍服务能做的只是 UI 自动化，不能跑 shell。
 *
 * 三者都没有时，返回的 detail 就是那句「没有Shell权限的终端」。
 */
export async function shellProbe(): Promise<ShellProbe> {
  if (!isTauri) return normProbe(null);

  let base = normProbe(null);
  try {
    base = normProbe(await invoke<Partial<ShellProbe>>("shell_probe"));
  } catch {
    /* Rust 侧不可用就继续往下走 */
  }

  if (!isAndroid) return base;

  try {
    const extra = normProbe(await invoke<Partial<ShellProbe>>("plugin:xmm-shell|probe"));
    const backend = extra.backend !== "none" ? extra.backend : base.backend;
    let detail = extra.detail;
    if (!detail || extra.backend === "none") {
      detail =
        backend === "shizuku"
          ? "Shizuku · shell 身份"
          : backend === "root"
          ? "root · shell 身份"
          : "没有Shell权限的终端";
    }
    return { ...extra, backend, detail };
  } catch {
    // 插件未打包（或版本不匹配）→ 当作没有 shell
    return normProbe({ backend: "none", detail: "没有Shell权限的终端" });
  }
}

export async function shellRun(
  cmd: string,
  granted: boolean,
  timeoutMs?: number
): Promise<ShellResult> {
  if (!isTauri) throw new Error("浏览器预览模式无法执行指令");

  if (isAndroid) {
    return invoke<ShellResult>("plugin:xmm-shell|run", {
      cmd,
      timeoutMs: timeoutMs ?? null,
    });
  }
  return invoke<ShellResult>("shell_run", {
    cmd,
    granted,
    timeoutMs: timeoutMs ?? null,
  });
}

/** 跳到系统的无障碍设置页（安卓） */
export async function openA11ySettings(): Promise<void> {
  if (!isTauri || !isAndroid) return;
  await invoke("plugin:xmm-shell|openA11ySettings");
}

/** 弹 Shizuku 授权（安卓） */
export async function requestShizuku(): Promise<void> {
  if (!isTauri || !isAndroid) return;
  await invoke("plugin:xmm-shell|requestShizuku");
}

export async function screenGrab(maxWidth = 960): Promise<string> {
  if (!isTauri) throw new Error("浏览器预览模式无法抓屏");
  return invoke<string>("screen_grab", { maxWidth });
}

/* ---------------- 窗口（仅桌面） ---------------- */

export async function winMinimize() {
  if (!isTauri) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().minimize();
}

export async function winToggleMax() {
  if (!isTauri) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().toggleMaximize();
}

export async function winClose() {
  if (!isTauri) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().close();
}

export async function winIsMaxed(): Promise<boolean> {
  if (!isTauri) return false;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow().isMaximized();
}
