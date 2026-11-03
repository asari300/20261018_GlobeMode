#!/usr/bin/env bash
# dist/ を配布物 <out>/ にまとめる（S145、web.yml が呼ぶ。手元でも回せる）。
#   bash src/Svelte/scripts/web-bundle.sh src/Svelte/dist /tmp/globe-web
# CSP は index.html の meta から取り出す（正は index.html の 1 か所。test_ui.py の header もそこから取る）。
# meta は Web Worker に効かないので、配布先は同じ CSP を応答の header に付ける: _headers（Cloudflare Pages / Netlify）と nginx.csp.conf（nginx の include）。
# Cross-Origin-Opener/Embedder-Policy は要らない（SharedArrayBuffer を使わない。S145 で確かめた）。
# 最後に大きさ（生・gzip -9・brotli -q 11）を Markdown の表にして標準出力へ出す。brotli が無ければその列は「未測定」。
set -euo pipefail
dist=$1
out=$2
rm -rf "$out"
mkdir -p "$out"
cp -r "$dist"/. "$out"/
rm -f "$out/graph-test.html" # 試験用の入口
csp=$(sed -n 's/.*http-equiv="Content-Security-Policy" content="\([^"]*\)".*/\1/p' "$out/index.html")
test -n "$csp"
cat > "$out/_headers" <<EOF
/*
  Content-Security-Policy: $csp
  X-Content-Type-Options: nosniff
  Referrer-Policy: no-referrer
EOF
echo "add_header Content-Security-Policy \"$csp\" always;" > "$out/nginx.csp.conf"

size() { stat -c %s "$1"; }
mib() { awk -v n="$1" 'BEGIN { printf "%.1f", n / 1048576 }'; }
echo "| ファイル | 生 (MiB) | gzip -9 (MiB) | brotli -q 11 (MiB) |"
echo "| --- | --- | --- | --- |"
row() { # 名前 ファイル...（複数なら tar で 1 本にして測る）
  local name=$1 tmp raw gz br="未測定"
  shift
  tmp=$(mktemp)
  cat "$@" > "$tmp"
  raw=$(size "$tmp")
  gz=$(gzip -9 -c "$tmp" | wc -c)
  if command -v brotli > /dev/null; then br=$(mib "$(brotli -q 11 -c "$tmp" | wc -c)"); fi
  echo "| $name | $(mib "$raw") | $(mib "$gz") | $br |"
  rm -f "$tmp"
}
# 合計は「ファイルごとに圧縮した大きさの和」に近いが、ここでは連結して測る（配布先が個別に圧縮するので目安）
row "wasm (globe_mode_wasm_bg.wasm)" "$out/wasm/pkg/globe_mode_wasm_bg.wasm"
if [ -f "$out/msl/MSL.json" ]; then row "msl/MSL.json" "$out/msl/MSL.json"; fi
row "JS・CSS (assets/)" "$out"/assets/*
row "合計（連結）" $(find "$out" -type f | sort)
echo
echo "ファイル数: $(find "$out" -type f | wc -l)、合計 $(mib "$(du -sb "$out" | cut -f1)") MiB（圧縮前）"
