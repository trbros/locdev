import { readFileSync } from "node:fs";
import path from "node:path";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig, type Plugin } from "vite";

const host = process.env.TAURI_DEV_HOST;

// package.json is the single source of the version; tauri.conf.json points at it.
const { version } = JSON.parse(readFileSync(new URL("./package.json", import.meta.url), "utf8"));

// The mock daemon reports the real daemon version, so screenshots match the docs.
const daemonVersion =
  readFileSync(new URL("../../Cargo.toml", import.meta.url), "utf8").match(
    /\[workspace\.package\][^[]*?version = "([^"]+)"/,
  )?.[1] ?? "0.0.0";

/** Writes the version into static pages (the splash has no JS to do it). */
const appVersion: Plugin = {
  name: "app-version",
  transformIndexHtml: (html) => html.replaceAll("__APP_VERSION__", version),
};

// https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [react(), tailwindcss(), appVersion],
  define: { __DAEMON_VERSION__: JSON.stringify(daemonVersion), __APP_VERSION__: JSON.stringify(version) },
  resolve: {
    alias: { "@": path.resolve(import.meta.dirname, "./src") },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari15",
    minify: !process.env.TAURI_ENV_DEBUG ? "oxc" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    // Assets load from disk inside the app, so bundle size matters little.
    chunkSizeWarningLimit: 1500,
    rolldownOptions: {
      // The splash is its own page: no React, no shared bundle.
      input: {
        main: path.resolve(import.meta.dirname, "index.html"),
        splash: path.resolve(import.meta.dirname, "splash.html"),
      },
    },
  },
});
