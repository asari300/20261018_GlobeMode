# S158 `sim-sdirk34hw-thyristor`

| | |
| --- | --- |
| 日付 | 2026-11-05 |
| ブランチ | `feature/s158-sim-sdirk34hw-thyristor` |
| worktree | `.git-worktree/s158-sim-sdirk34hw-thyristor` |
| Phase | P3（実行系） |
| セッション文書 | `doc/sessions/S158-sim-sdirk34hw-thyristor/` |
| セッション記録 | （親が写す） |

## 依頼

`sdirk34hw` が MSL の `ThyristorBridge2Pulse_DC_Drive` で `GM-E11003` を出す持ち越し（S126・S139）を調べ、直せるなら最小の変更で直し、直せないなら原因を特定して記録する。

## 決めたこと

- 原因は Rumoca の host（`rumoca-solver` の `relation_memory.rs`）の `value == 0.0` と `roots[*index] == 0.0`。GlobeMode 側では直さない（Rumoca は変えない規則。プラグインで補間の雑音を整えると全例題の数値が動く）。
- 追跡の道具: `rk.rs` に `GM_TRACE` の `eprintln`、Rumoca の checkout を scratchpad に複製して `[patch]` で差し、`relation_memory.rs` に追跡と `abs() <= 1e-24` の確認用の書き換え。いずれもコミットしていない（`Cargo.toml`・`Cargo.lock`・`rk.rs` は戻した）。
- 上流の下書きは 34 番。`.mo` は MSL の例題の experiment を決めるだけ（Rumoca の CLI で小さく再現する形は作れなかった。雑音の符号は刻みの列の偶然）。README の規則（MSL の例題の件は `.mo` を置かない）から外れるが、依頼の 4 点に合わせた。
- 実行系は変えていないので、`wasm:node`・`wasm:compare`・`msl-sim.yml` は回さない。

## やったこと

- `src/Rust/mod/globe-mode-exec/tests/simulate.rs`: 既知の限界の試験 `msl_sdirk34hw_thyristor_bridge_event_iteration_is_a_known_limit`（`#[ignore]`、release で 116 s）。
- `src/Rust/xtask/src/solvers.rs`・`doc/msl-solvers.md`: 表の注記に 1 文（生成する側と生成物で同じ文）。
- `doc/research/upstream/34-…`（`.md`・`.mo`・`_jp.md`・`_jp.html`）、`doc/research/upstream/README.md`・`doc/upstream.md` の行と提出の順・回避を外す条件の行。
- `doc/sessions/S158-sim-sdirk34hw-thyristor/`、`doc/sessions/README.md`、`doc/roadmap.md`、`doc/features.md`。

## 検証

- 再現: release の `globe simulate --solver sdirk34hw --tolerance 1e-7` で 57 s、t = 13.040017098 に `GM-E11003`。`--stop-time 13.2` は通る（56.5 s）。
- 確認用の書き換え（コミットしていない）: host の 2 行を `abs() <= 1e-24` にして 15 s まで通った（出力 87,498 行、64.7 s）。
- 既知の限界の試験: `cargo test --release -p globe-mode-exec --test simulate msl_sdirk34hw -- --ignored` が成功（116.17 s）。
- コードの検査（`mise run check`、load average 0.4 で 1 回）: fmt・clippy `-D warnings`・deny（advisories / bans / licenses / sources ok）・`uv sync --locked` が通り、試験は 653 passed・失敗 0・ignored 12（新しい `#[ignore]` を含む）
- `check_docs.py`: 文書の相対リンク 切れ 0 件（スライドは全件 OK）。`render_slides.py` は S158 のデッキと下書き 34 の 1 枚で 問題 0 件（1 回目で通った）
- セッション記録の `--check`: （親が写す）
- ステージした差分の漏洩検査: 0 件（メールアドレス・ホームのパス・トークンのパターンで検索）

## 残したこと

- 下書き 34 の提出（ユーザー）。最新の Rumoca で実行して確かめる（今回はコードの 2 行の比較だけ）。
- 同じ雑音がほかのサイリスタ・ダイオードの回路（`ThyristorBridge2mPulse` など）で当たる数は未調査。
- `msl:funnel`・`msl:sim` 系は回していない（実行系・Rumoca の rev を変えていないため）。
