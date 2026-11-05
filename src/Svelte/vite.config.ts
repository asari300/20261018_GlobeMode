import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

const ISOLATION = { "Cross-Origin-Opener-Policy": "same-origin", "Cross-Origin-Embedder-Policy": "require-corp" };

export default defineConfig({
  plugins: [svelte()],
  preview: { headers: ISOLATION },
  base: "./", // 相対パス: dist/ をどのサブパスに置いても動く（S145。import.meta.env.BASE_URL も "./" になり、worker・MSL.json は location.href からの相対で引く）
  // worker は bundle せず public/wasm/ から配る（node:worker_threads の分岐と実行時の import(pkg) を Vite に触らせない）
  // graph-test.html は Graph.svelte を単体で mount する試験用の入口（tests/test_graph.py）
  build: { rollupOptions: { input: { main: "index.html", "graph-test": "graph-test.html" } } },
  // S157: 実行を止めたときに途中の列を返すための SharedArrayBuffer は cross-origin isolated が要る（付けない配信では今までどおり Worker を作り直す）
  server: { headers: ISOLATION, fs: { allow: ["../.."] } }, // client.js を src/Rust/mod/globe-mode-wasm/web/ から直接 import する
});
