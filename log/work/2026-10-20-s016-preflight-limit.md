# S016 `preflight-limit`

| | |
| --- | --- |
| 日付 | 2026-10-20 |
| ブランチ | `feature/s016-preflight-limit` |
| worktree | `.git-worktree/s016-preflight-limit` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S016-preflight-limit/` |
| セッション記録 | `log/sessions/2026-10-20-b7d1157a-eaea-4a40-935a-b45b29ceaab8.md` |

## 依頼

「リポジトリの方針を確認し、1を進めよ。サブエージェントはOpus/effort/mediumとせよ。」（1 = S015 の残し: `DrumBoiler` の S10 のプリフライト（t0 の評価）が 481 s 以上終わらない。プリフライトは警告しか出さないので上限を掛けられる。[roadmap.md](../../doc/roadmap.md) の S016 の候補 4）

## 決めたこと

- **プリフライトに上限を掛けない（ユーザーと決めた）**: 調べた結果、DrumBoiler はプリフライトの評価（`eval_at`）に入っておらず、Rumoca の `SolveRuntime::new` の中で、根の条件 31 本（計 59.7 万演算）の構造解析が終わらなかった。組み立ての段も同じ `construct` を通る（`fmi_me/kernel/component.rs:547`）ので、上限を掛けても次の組み立てで詰まる。ほかの 355 本のプリフライトは最大 1.9 s。3 案（別スレッドと待ちの打ち切り、締め切りの仕組み、組み立ての後に回す）を比べ、どれも本数が増えずにコードと危険だけが増えるので採らなかった。ユーザーに 3 案を示して「記録して上流へ回す」を選んでもらった（[ADR-0018](../../doc/adr/0018-no-preflight-limit.md)）。
- **WSL のメモリ 3 GB のまま、全体の実行はしない（ユーザーと決めた）**: `.wslconfig` が `memory=3GB`。コードを変えないので、`msl:funnel`・`msl:sim` を回す決まりにも当たらない。
- サブエージェントは Opus・effort medium（ユーザーの指示）。調べもの 2 つ（DrumBoiler を回して標本を取る担当、Rumoca のコードを読む担当。後者はビルドしない）を並べ、メモリの都合で release のビルドと実行は 1 人に限った。続けて EngineV6 の外れを 1 人に頼んだ。
- 実験の担当の「組み立ても `prepared_vectors.rs:93` の `SolveRuntime::new` を呼ぶ」は誤りだった（その関数は組み立ての道から呼ばれない）。メインがコードで確かめ、組み立ての道は `fmi_me/kernel/component.rs:547` の `new_with_execution_backend` で同じ `construct` を通ると改めた。結論は変わらない。

- **EngineV6 は既知の外れの一覧に載せない（既定のまま進めた）**: 原因は Rumoca の初期化の不具合（初期化方程式を関係式 `v_rel < 0` の既定値 false で解き、反転しても解き直さない）。一覧の方針は「モデル・参照・比べ方の限界」なので、実行系の不具合は外れとして数え続ける。上流の候補 11 にした。メインが小さいモデル R1 を `globe simulate` で回し直し、t = 0 で `u = 1`・`filter.x = 2` を確かめた。

## やったこと

- 調べもの（サブエージェント 3 人、Opus・effort medium）: DrumBoiler を 5 回回して `perf` の標本と一時的な計測（すべて元に戻した）、Rumoca の `eval_at` から下と打ち切りの手段をコードで読む、EngineV6 の t = 0 の値を参照・Rumoca の CLI・omc と比べて小さいモデルで再現。
- 文書: ADR-0018（新規）と ADR の索引、ADR-0017 の結果に追記、research/msl-sim-failures.md（S015 の見込みの訂正、節「S016 で調べたこと」、上流の候補 10・11）、roadmap.md（S016 の節、S017 の候補 4 を「初期化の後の初期化方程式の検査」に、候補 8 に 10・11）、セッション文書と索引。
- コードは変えていない。

## 検証

- コードの検査（fmt / clippy / test / deny）: 対象外（コードを変えていない）。
- `msl:funnel`・`msl:sim`: 回していない（コードを変えていない。WSL のメモリ 3 GB）。数字は S015 のまま（309 本が実行でき、301 / 308 本が管に収まる）。
- R1 の再現: メインが `target/release/globe simulate` で回し直し、t = 0 で `u = 1`・`filter.x[1] = filter.x[2] = 2` を確かめた。
- `check_docs.py`: デッキ全件 OK / 文書 110 件・相対リンク 752 件中の切れ 0 件
- `render_slides.py`（S016 のデッキ、8 枚 × 3 表示）: 問題 0 件。S05 の図で枠からはみ出した文字を直し、ライト・ダーク・スマートフォンを目で見た。
- markdownlint: 機械に入っていないので回していない（空行の規則は目で確かめた）。
- セッション記録の `--check`: 7 件中、取りこぼし 0 件
- ステージした差分の漏洩検査: 0 件

## 残したこと

- `DrumBoiler` と `EngineV6` は上流（Rumoca）待ち。報告はユーザーが出す（roadmap の候補 8）。
- GlobeMode は初期化の後の初期化方程式の残差を見ていない（S017 の候補 4）。
- WSL のメモリが 3 GB なので、`msl:funnel`・`msl:sim` の全体は回していない（コードを変えていないので、決まりにも当たらない）。数字は S015 のまま（309 本が実行でき、301 / 308 本が管に収まる）。
- DrumBoiler の根の条件 1 本の解析が実際に何秒で終わるか、組み立ての段が最後まで終わるかは確かめていない（15 分の枠と機械の混み具合のため）。
- 締めの後に、ユーザーから「Claude の実行場所を RPi に移す。最初のセッションで Rumoca の CLI の aarch64 対応をする」と指示があった（2026-10-22）。roadmap.md の S017 の候補の頭に「S017 の最初にやること」として書いた（xtask の `fetch-rumoca` の aarch64 対応、RPi の環境、メモリ）。
