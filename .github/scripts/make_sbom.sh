#!/usr/bin/env bash
# 配布物ごとの CycloneDX 1.5 の SBOM（JSON）を <出力先>（既定 sbom）に作る（S380）。release.yml のジョブ sbom と web.yml が呼ぶ。
# 要るもの: cargo-cyclonedx（版は mise.toml）・pnpm・uv。Rust は配布する target ごとに 1 つ（about.toml の targets と同じ）。
#   使い方: make_sbom.sh [出力先] [rust|npm|python ...]   既定は全部。web.yml は rust のうち wasm だけ要るので `wasm` も取れる
set -euo pipefail
out="${1:-sbom}"; shift || true
kinds="${*:-rust npm python}"
mkdir -p "$out"; out="$(cd "$out" && pwd)"
cd "$(dirname "$0")/../.."

# cargo-cyclonedx は manifest が属する workspace の全 crate の SBOM を各 crate のディレクトリに書く。要るのは manifest の crate の分だけ
rust() { # 名前 Cargo.toml target...
  local name="$1" manifest="$2"; shift 2
  for t in "$@"; do
    cargo cyclonedx --manifest-path "$manifest" --format json --spec-version 1.5 --target "$t" --override-filename "sbom-$name-$t"
    mv "$(dirname "$manifest")/sbom-$name-$t.json" "$out/sbom-$name-$t.cdx.json"
    find src -name "sbom-$name-$t.json" -delete
  done
}

for k in $kinds; do
  case "$k" in
    rust)
      linux_win_mac="x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-pc-windows-msvc aarch64-apple-darwin"
      # shellcheck disable=SC2086
      rust globe src/Rust/mod/globe-mode-cli/Cargo.toml $linux_win_mac      # アーカイブの globe（と Desktop の sidecar）
      # shellcheck disable=SC2086
      rust wheel src/Rust/mod/globe-mode-py/Cargo.toml $linux_win_mac       # wheel の _native
      rust desktop src/Rust/apps/desktop/Cargo.toml x86_64-unknown-linux-gnu x86_64-pc-windows-msvc aarch64-apple-darwin
      rust wasm src/Rust/mod/globe-mode-wasm/Cargo.toml wasm32-unknown-unknown ;;
    wasm) rust wasm src/Rust/mod/globe-mode-wasm/Cargo.toml wasm32-unknown-unknown ;;
    npm)  # dev 依存も含む（Svelte の実行時などが dist に入る。licenses の表と同じ範囲）
      (cd src/Svelte && pnpm sbom --sbom-format cyclonedx --sbom-spec-version 1.5 --sbom-type application --out "$out/sbom-npm.cdx.json") ;;
    python)  # wheel が宣言する依存（marimo・plotly・polars と extra の widget）。wheel には入らず、入れるときに解決される。uv.lock の版
      uv export -q --frozen --no-dev --extra widget --format cyclonedx1.5 -o "$out/sbom-python.cdx.json" ;;
    *) echo "知らない種類: $k" >&2; exit 2 ;;
  esac
done
ls -l "$out"
