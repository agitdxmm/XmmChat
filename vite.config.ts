import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// 固定端口：Tauri 的 devUrl 指向 1420
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
  },
  build: {
    target: "chrome105",
    minify: "esbuild",
    sourcemap: false,
    chunkSizeWarningLimit: 2000,
    emptyOutDir: false,
  },
});
