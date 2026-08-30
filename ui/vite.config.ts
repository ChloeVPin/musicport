import { defineConfig } from "vite";

// Tauri expects a fixed non-conflicting dev port.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: {
      ignored: ["**/crates/desktop/**", "**/target/**"],
    },
  },
});