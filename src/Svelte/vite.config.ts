import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  // worker は bundle せず public/wasm/ から配る（node:worker_threads の分岐と実行時の import(pkg) を Vite に触らせない）
  server: { fs: { allow: ["../.."] } }, // client.js を src/Rust/mod/globe-mode-wasm/web/ から直接 import する
});
