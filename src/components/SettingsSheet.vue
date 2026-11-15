<script setup lang="ts">
import { computed, ref } from "vue";
import { state, toast } from "../core/store";
import { isAndroid, openA11ySettings, requestShizuku, shellProbe } from "../core/bridge";

const emit = defineEmits<{ (e: "close"): void }>();

const fileInput = ref<HTMLInputElement | null>(null);

const kb = computed(() => {
  const n = state.settings.wallpaper.length;
  return n > 1024 ? `${(n / 1024 / 1024).toFixed(1)} MB` : "—";
});

async function refreshShell() {
  state.shell = await shellProbe();
  toast("权限状态已刷新", "info");
}

async function a11y() {
  try {
    await openA11ySettings();
  } catch (e: any) {
    toast(`打不开设置页：${e?.message || e}`, "err");
  }
}

async function shizuku() {
  try {
    await requestShizuku();
    toast("已向 Shizuku 发起授权请求", "info");
    setTimeout(refreshShell, 1200);
  } catch (e: any) {
    toast(`Shizuku 不可用：${e?.message || e}`, "warn");
  }
}

/** 读图 → 压到最长边 2000px 的 JPEG，避免 localStorage 撑爆 */
function pickWallpaper() {
  fileInput.value?.click();
}

async function onWallpaper(e: Event) {
  const input = e.target as HTMLInputElement;
  const f = input.files?.[0];
  input.value = "";
  if (!f) return;
  try {
    const dataUrl = await shrink(f, 2000, 0.82);
    state.settings.wallpaper = dataUrl;
    toast("背景已更新", "ok");
  } catch (err: any) {
    toast(`图片处理失败：${err?.message || err}`, "err");
  }
}

function shrink(file: File, maxSide: number, quality: number): Promise<string> {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file);
    const img = new Image();
    img.onload = () => {
      const scale = Math.min(1, maxSide / Math.max(img.width, img.height));
      const w = Math.round(img.width * scale);
      const h = Math.round(img.height * scale);
      const cv = document.createElement("canvas");
      cv.width = w;
      cv.height = h;
      const ctx = cv.getContext("2d");
      if (!ctx) {
        URL.revokeObjectURL(url);
        reject(new Error("无法创建画布"));
        return;
      }
      ctx.drawImage(img, 0, 0, w, h);
      URL.revokeObjectURL(url);
      resolve(cv.toDataURL("image/jpeg", quality));
    };
    img.onerror = () => {
      URL.revokeObjectURL(url);
      reject(new Error("这不是一张能读的图片"));
    };
    img.src = url;
  });
}

function clearWallpaper() {
  state.settings.wallpaper = "";
  toast("已移除背景图", "info");
}
</script>

<template>
  <div class="mask" @click.self="emit('close')">
    <section class="sheet glass glass-edge">
      <header>
        <div>
          <h3>外观与偏好</h3>
          <span class="hsub">全部即时生效</span>
        </div>
        <button class="x" @click="emit('close')">
          <svg viewBox="0 0 18 18"><path d="M5 5l8 8M13 5l-8 8" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" /></svg>
        </button>
      </header>

      <div class="pane">
        <!-- 主题 -->
        <div class="grp">
          <label class="gl">主题</label>
          <div class="seg">
            <button :class="{ on: state.settings.theme === 'dark' }" @click="state.settings.theme = 'dark'">
              <svg viewBox="0 0 18 18"><path d="M14.4 11A6.2 6.2 0 0 1 7 3.6a6.2 6.2 0 1 0 7.4 7.4Z" stroke="currentColor" fill="none" stroke-width="1.4" stroke-linejoin="round" /></svg>
              深色
            </button>
            <button :class="{ on: state.settings.theme === 'light' }" @click="state.settings.theme = 'light'">
              <svg viewBox="0 0 18 18"><circle cx="9" cy="9" r="3" stroke="currentColor" fill="none" stroke-width="1.4" /><path d="M9 2v1.4M9 14.6V16M2 9h1.4M14.6 9H16M4.2 4.2l1 1M12.8 12.8l1 1M13.8 4.2l-1 1M5.2 12.8l-1 1" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" /></svg>
              亮色
            </button>
          </div>
        </div>

        <!-- 背景图 -->
        <div class="grp">
          <label class="gl">聊天背景<span class="gh">亚克力图片</span></label>
          <div class="wp">
            <div
              class="wpv"
              :style="state.settings.wallpaper ? { backgroundImage: `url('${state.settings.wallpaper}')` } : {}"
            >
              <span v-if="!state.settings.wallpaper">未设置</span>
            </div>
            <div class="wpbtns">
              <button class="btn small" @click="pickWallpaper">选择图片</button>
              <button v-if="state.settings.wallpaper" class="btn small danger" @click="clearWallpaper">移除</button>
            </div>
          </div>
          <input ref="fileInput" type="file" accept="image/*" class="hidden" @change="onWallpaper" />
          <p class="tip">图片会被压到 2000px 并模糊处理，体积 {{ kb }}</p>
        </div>

        <template v-if="state.settings.wallpaper">
          <div class="grp">
            <label class="gl">背景模糊 <b>{{ state.settings.wallpaperBlur }}px</b></label>
            <input v-model.number="state.settings.wallpaperBlur" type="range" min="0" max="40" step="1" class="rng" />
          </div>
          <div class="grp">
            <label class="gl">背景压暗 <b>{{ Math.round(state.settings.wallpaperDim * 100) }}%</b></label>
            <input v-model.number="state.settings.wallpaperDim" type="range" min="0" max="0.9" step="0.02" class="rng" />
          </div>
        </template>

        <!-- 玻璃 -->
        <div class="grp">
          <label class="gl">气泡毛玻璃 <b>{{ state.settings.glassBlur }}px</b></label>
          <input v-model.number="state.settings.glassBlur" type="range" min="0" max="40" step="1" class="rng" />
          <p class="tip">默认 25px，就是「25% 毛玻璃」的手感</p>
        </div>

        <div class="grp">
          <label class="gl">气泡不透明度 <b>{{ Math.round(state.settings.bubbleAlpha * 100) }}%</b></label>
          <input v-model.number="state.settings.bubbleAlpha" type="range" min="0.02" max="0.4" step="0.01" class="rng" />
        </div>

        <!-- 其他 -->
        <div class="grp">
          <label class="gl">消息密度</label>
          <div class="seg">
            <button :class="{ on: state.settings.density === 'cozy' }" @click="state.settings.density = 'cozy'">宽松</button>
            <button :class="{ on: state.settings.density === 'compact' }" @click="state.settings.density = 'compact'">紧凑</button>
          </div>
        </div>

        <div class="grp">
          <label class="gl">我的昵称</label>
          <input v-model="state.settings.myName" class="field" maxlength="24" placeholder="对方看到的名称" />
        </div>

        <!-- 安卓专属：权限（全部选填） -->
        <div v-if="isAndroid" class="grp">
          <label class="gl">
            安卓权限
            <span class="gh">选填</span>
            <b class="mono">{{ state.shell.detail }}</b>
          </label>

          <div class="perm">
            <div class="prow">
              <span class="pk">无障碍服务</span>
              <span class="pv" :class="{ ok: state.shell.a11y }">
                {{ state.shell.a11y ? "已开启" : "未开启" }}
              </span>
            </div>
            <div class="prow">
              <span class="pk">Shizuku</span>
              <span class="pv" :class="{ ok: state.shell.shizuku }">
                {{ state.shell.shizuku ? "已授权" : "未授权" }}
              </span>
            </div>
            <div class="prow">
              <span class="pk">Root</span>
              <span class="pv" :class="{ ok: state.shell.root }">
                {{ state.shell.root ? "可用" : "无" }}
              </span>
            </div>

            <div class="pbtns">
              <button class="btn small" @click="a11y">打开无障碍设置</button>
              <button class="btn small" @click="shizuku">请求 Shizuku</button>
              <button class="btn small ghost" @click="refreshShell">刷新</button>
            </div>

            <p class="tip">
              不授权也能正常聊天、发文件、通话。只有「远程执行指令」需要 Shizuku 或 root；
              没有的话终端会显示「没有Shell权限的终端」。
            </p>
          </div>
        </div>

        <div class="about">
          <span>咪的天 XmmChat 1.0.0</span>
          <span class="mono">端口 {{ state.host.port }} · 点对点</span>
          <span class="sign">作者 · 爱玩的小喵咪</span>
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
  width: min(480px, 94vw);
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
  padding: 16px 18px 10px;
}
header h3 {
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

.pane {
  padding: 2px 18px 18px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 15px;
}

.grp {
  display: flex;
  flex-direction: column;
  gap: 7px;
}

.gl {
  display: flex;
  align-items: baseline;
  gap: 6px;
  font-size: 11.5px;
  color: var(--ink-2);
}
.gl b {
  margin-left: auto;
  font-size: 10.5px;
  font-weight: 500;
  color: var(--ink-3);
}
.gh {
  font-size: 9.5px;
  padding: 0 6px;
  border-radius: var(--r-pill);
  color: var(--ink-4);
  background: rgba(255, 255, 255, 0.05);
}

.tip {
  font-size: 10px;
  color: var(--ink-4);
  line-height: 1.5;
}

.seg {
  display: flex;
  gap: 4px;
  padding: 3px;
  border-radius: var(--r-sm);
  background: rgba(0, 0, 0, 0.24);
  border: 1px solid var(--line-1);
}
.seg button {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 7px;
  font-size: 11.5px;
  border-radius: 8px;
  color: var(--ink-3);
  transition: all 0.2s var(--ease-smooth);
}
.seg button svg {
  width: 15px;
  height: 15px;
}
.seg button.on {
  color: var(--ink-1);
  background: rgba(255, 255, 255, 0.1);
  box-shadow: var(--shadow-1);
}

.wp {
  display: flex;
  gap: 10px;
  align-items: center;
}
.wpv {
  width: 96px;
  height: 58px;
  flex: 0 0 96px;
  border-radius: 10px;
  background-size: cover;
  background-position: center;
  background-color: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--line-1);
  display: grid;
  place-items: center;
  font-size: 10px;
  color: var(--ink-4);
  overflow: hidden;
}
.wpbtns {
  display: flex;
  flex-direction: column;
  gap: 6px;
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

.rng {
  -webkit-appearance: none;
  appearance: none;
  width: 100%;
  height: 4px;
  border-radius: 3px;
  background: linear-gradient(90deg, var(--hue-a), var(--hue-b));
  outline: none;
  opacity: 0.9;
}
.rng::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #fff;
  border: 2px solid rgba(167, 139, 250, 0.9);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
  cursor: pointer;
  transition: transform 0.15s var(--ease-spring);
}
.rng::-webkit-slider-thumb:hover {
  transform: scale(1.18);
}

.hidden {
  display: none;
}

/* ---- 作者署名 ---- */
.sign {
  font-size: 11px;
  font-style: italic;
  letter-spacing: 0.06em;
  color: var(--ink-3);
}

/* ---- 安卓权限 ---- */
.perm {
  padding: 11px 12px;
  border-radius: var(--r-md);
  background: rgba(255, 255, 255, 0.035);
  border: 1px solid var(--line-1);
}
.prow {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 5px 0;
  font-size: 11.5px;
  border-bottom: 1px solid var(--line-1);
}
.prow:last-of-type {
  border-bottom: 0;
}
.pk {
  color: var(--ink-2);
}
.pv {
  font-size: 11px;
  color: var(--ink-3);
}
.pv.ok {
  color: #4ade80;
}
.pbtns {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 9px;
}
.btn.ghost {
  color: var(--ink-3);
  background: transparent;
}

.about {
  display: flex;
  justify-content: space-between;
  padding-top: 12px;
  border-top: 1px solid var(--line-1);
  font-size: 10px;
  color: var(--ink-4);
}
</style>
