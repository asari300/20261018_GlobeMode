# ADR-0003 ツールチェーン（stable を既定にする）

- 状態: **Proposed**（2026-10-18、S001）。P1 で確かめてから Accepted にする。

## 背景

- Rumoca は `nightly-2026-02-27` を固定している（wasm のスレッド対応のため）。
- 手元のツールチェーンは stable 1.99.1 で、wasm と Android の target はまだ入っていない。
- 言語が多い（Rust / Python / Zig / Julia / Elixir / Gleam / Node）ので、ツールの版をそろえる仕組みが要る。main では mise の手順書を準備中。

## 決定（案）

- `rust-toolchain.toml` は **stable 1.99.1** にする。target は wasm32-unknown-unknown、wasm32-wasip2、aarch64-linux-android。
- スレッドを使う wasm だけを nightly でビルドする: `xtask wasm --threads` で `cargo +nightly-2026-02-27` を呼ぶ。
- 言語ごとのツールの版は mise でそろえる。Python 本体は uv が `.python-version` から取得する。
- `rumoca-compile` が stable でビルドできなかった場合は、次の順に試す。
  1. nightly の機能を使っている部分を `cfg` で分ける PR を上流に出す。
  2. `[patch]` で fork を当てる。
  3. 最後の手段として、ワークスペース全体を nightly に固定する。

## 結果

- 既定のブラウザ版はシングルスレッドになる。rayon が現在のスレッドで動くようにフォールバックするかは P1 で確かめる。
