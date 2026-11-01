# ADR-0030 Desktop は MSL を同梱せず、初回に app data dir へ取得する。`--library` と `GLOBE_LIBRARY` は今のまま優先する

- 状態: Accepted（2026-11-01、S069。`desktop.yml` run 41308587091 で、MSL を渡さずに起動したアプリが MSL 4.1.0 を取得して `ChuaCircuit` を検査・実行し、2 度目の起動が取得済みの置き場を使うことを確かめた。結果は「結果」の節）
- 関係: [ADR-0023](0023-desktop-and-web-process-layout.md)（sidecar と `--library`）の持ち越し「MSL を同梱するか取得するか」（S049・S056）を決める。ADR-0023 は覆さない

## 背景

- Desktop は MSL を、起動時の `--library <DIR>` か `GLOBE_LIBRARY` で sidecar（`globe lsp`）に渡すだけだった（S049・S056）。何も渡さなければ MSL 無しで動き、`TauriBackend.library()` は例外になる。配布版（.deb・.msi）を起動した利用者は、環境変数を知らなければ MSL を使えない。
- MSL 4.1.0 の公式 zip は 29 MB、ライセンスは BSD-3-Clause で再配布できる（[licenses.md](../licenses.md)）。`cargo xtask fetch-msl` は同じ zip を URL と sha256 を固定して取得し、`.globe-mode-msl` の目印を最後に書く。
- 現在の配布物は .deb 20.0 MB・.msi 18.2 MB（S056）。

## 決定

### (a) 置き方は「初回に取得」。比べた 3 案

| 案 | 長所 | 短所 |
| --- | --- | --- |
| 同梱 | 初回から使える。オフラインで動く | 配布物が約 29 MB（zip）以上増える。MSL の版を上げるたびにアプリを配り直す。版を切り替えられない |
| 初回に取得（採用） | 配布物が増えない。版ごとに置き場を分けられる（`msl/<版>/`）。MSL の更新はアプリ側の定数の更新だけ | 初回にネットワークが要る（29 MB）。取得の失敗を UI に出す必要がある |
| 利用者が場所を指定 | 実装は最小（`--library` が既にある） | MSL を持っていない利用者に何も起きない。UI の選択ダイアログは Tauri の plugin の依存を足す |

- 「利用者が場所を指定」は**今の道として残す**（`--library`・`GLOBE_LIBRARY` があれば取得しない）。MSL 以外のライブラリには、これが唯一の道。
- 同梱を選ばなかった決め手は、更新と版の切り替え。MSL は 4.0.0 と 4.1.0 が併存しており（S067）、配布物に焼き込むと版の選択肢を閉じる。同梱はライセンス上は可能（BSD-3-Clause）。

### (b) 置き場と優先順位

- 置き場は Tauri の app data dir の `msl/4.1.0/`（Linux は `~/.local/share/io.github.asari300.globemode/msl/4.1.0/`、Windows は `%APPDATA%\io.github.asari300.globemode\msl\4.1.0\`）。
- 起動時: `--library`・`GLOBE_LIBRARY` があればそれだけを sidecar に渡す。無くて、置き場に目印（`.globe-mode-msl`、内容 `4.1.0 <sha256>`）があれば置き場を渡す。どちらも無ければ MSL 無しで起動する。
- UI: 無いときは、「MSL を読む」ボタン（S056 までの `loadMsl`）が Desktop では「MSL を取得する（29 MB、初回のみ）」になり、押すと `TauriBackend.library()` が command `fetch_msl` を呼ぶ。取得中・読んだ・失敗の文言を出す（進捗の割合は出さない）。

### (c) 取得の手順（Rust 側 `src/Rust/apps/desktop/src/msl.rs`）

1. `curl --fail --location` で zip を `<置き場>/.download/msl.zip` に落とす。
2. sha256 を照合する。違えば展開せず失敗にする（zip は消す）。
3. `zip` crate で展開する。`enclosed_name()` で `..`・絶対パスを拒む。
4. **目印は最後に書く**。途中で落ちたら目印が無く、次回の起動でも「無い」扱いで、押し直すと最初からやり直す。zip は成否にかかわらず消す。
5. `fetch_msl` は取得のあと sidecar を止め、新しい `--library` で作り直す（開いているファイルは `Worker` が送り直す）。`initialize` の応答まで待つので、読めない MSL はここで UI に返る。

- ダウンロードは外部コマンド `curl` を使う（xtask の `download` と同じ）。Windows 10 以降は `curl.exe` を OS が持ち、.deb は `Depends: curl` を宣言する。HTTP クライアントの crate（TLS の依存が増える。desktop の `Cargo.lock` の reqwest は wasm 用で TLS を持たない）を足さないための選択。
- 足した依存は `sha2`（`Cargo.lock` に既にある）と `zip` 9（xtask と同じ版・features。MIT）。[licenses.md](../licenses.md) に追記し、desktop の `cargo deny` は通る。

### (d) xtask とコードを共有するか

- 共有しない。`xtask` は root workspace の bin で、依存は Rumoca の frontend など重く、desktop は別の workspace（webkit2gtk を要求するため root の `cargo build --workspace` から外している）。path 依存にすると desktop の `Cargo.lock` に root の依存が流れ込む。
- 共有 crate を新設する案（`globe-mode-fetch`）は、`src/Rust/mod/` と xtask の書き換えを伴う。約 100 行のために採らない。
- 代わりに定数のずれを CI で止める: `desktop.yml` が xtask の `MSL_URL`・`MSL_SHA256` と `msl.rs` の `URL`・`SHA256` を比べ、違えば落とす。目印の形式（`<版> <sha256>\n`）も同じなので、`cargo xtask fetch-msl --dest <置き場>` で取得した置き場を Desktop はそのまま使える。

### (e) 対象外

- 署名・自動更新・MSL 4.0.0 の切り替えの UI・取得の割合表示・取得の取り消し・複数ウィンドウでの同時取得の排他（UI がボタンを無効にするだけ。同時に呼ぶと同じファイルを書き合い、先に終えた側の目印の後に後続が上書き中のファイルを読むことがある。Rust 側に排他は付けていない）。

## 結果

`desktop.yml`（run 41308587091、ubuntu-24.04 の .deb、xvfb）の smoke を 3 回回した。いずれも `Modelica.Electrical.Analog.Examples.ChuaCircuit` の検査（`passed=true`）と実行（`ok=true`）が通った。

| 起動 | MSL の渡し方 | 取得 | sidecar の VmHWM（simulate の後） |
| --- | --- | --- | --- |
| 1 | `GLOBE_LIBRARY=data/msl`（S056 の道） | 無し | 171,444 kB |
| 2 | 渡さない + `GLOBE_DESKTOP_SMOKE_FETCH=1` | **1.0 s**（取得 + 展開） | 171,576 kB |
| 3 | 渡さない（置き場の目印を起動時に見つける） | 無し | 171,516 kB |

- 取得した MSL でも、`data/msl` を渡したときと検査・実行の結果・メモリが同じ。
- sidecar の VmHWM が S056 の 1.52 GiB から約 168 MiB に下がったのは S054（[ADR-0025](0025-native-library-closure.md)、閉包のファイルだけを読む）による。この ADR の変更ではない。
- 守ること: 取得に失敗したとき（ネットワーク無し・sha256 の不一致）は UI の `failure` に理由が出て、目印が無いのでボタンを押し直せる。この経路は単体試験（sha256 不一致 → 展開も目印も無し、展開先の外を指す名前を拒む）で確かめ、実際のネットワーク断は試していない。
- Windows の取得（`curl.exe` の存在と `%APPDATA%` の置き場）は CI の smoke が Linux だけなので確かめていない。
