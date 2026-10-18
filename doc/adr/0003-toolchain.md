# ADR-0003 ツールチェーン（mise・uv・cargo）

- 状態: **Proposed**（2026-10-18、S001）。
  - ツールの管理方法は決定済み。
  - Rumoca が stable でビルドできるかは P1 で確かめる。確かめたら Accepted にする。

## 背景

- 言語が多い（Rust / Python / Zig / Julia / Elixir / Gleam、P4 から Node）ので、ツールの版をそろえる仕組みが要る。
- main に `mise.toml` と `mise.lock` が入った。手順書は [doc/mise-setup.html](../mise-setup.html)。
- Rumoca は `nightly-2026-02-27` を固定している（wasm のスレッド対応のため）。

## 決定

**言語の管理は mise・uv・cargo の3つで分担する。**

| 道具 | 受け持つもの | ロックファイル |
| --- | --- | --- |
| **mise** | ツールの版: rust、uv、zig、julia、erlang、elixir、gleam（P4 で node を足す）。タスク（`mise run …`） | `mise.lock` |
| **uv** | Python 本体（`.python-version`）と Python の依存 | `uv.lock` |
| **cargo** | Rust の依存と workspace | `Cargo.lock` |

- **`rust-toolchain.toml` は作らない。** Rust の版は `mise.toml` に一本化する。
  - mise が設定する `RUSTUP_TOOLCHAIN` が優先され、`rust-toolchain.toml` は無視されるため。
- wasm と Android の target は、mise の rust の `targets` で宣言する（P1）。
  - 対象は wasm32-unknown-unknown、wasm32-wasip2、aarch64-linux-android。
- スレッドを使う wasm だけを nightly でビルドする。
  - `xtask wasm --threads` が `cargo +nightly-2026-02-27` を呼ぶ。rustup は mise の下でも使える。
- 新しい worktree では `mise trust` と `mise install --locked` を実行してから `uv sync` する（start-session スキル）。
- `rumoca-compile` が stable でビルドできなかった場合は、次の順に試す。
  1. nightly の機能を使っている部分を `cfg` で分ける PR を上流に出す。
  2. `[patch]` で fork を当てる。
  3. 最後の手段として、mise の rust を nightly に固定する。

## 結果

- CI でも mise を使う（`mise install --locked`）。手元と同じ版でビルドする。
- 既定のブラウザ版はシングルスレッドになる。rayon が現在のスレッドで動くようにフォールバックするかは P1 で確かめる。
