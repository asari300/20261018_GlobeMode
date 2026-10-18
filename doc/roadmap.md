# GlobeMode ロードマップ

- 版: v0（2026-10-18、S001）。
- **P** は開発の段階（Phase）、**S** はセッション番号（`doc/sessions/SNNN-*`）。
- 1 つの Phase は複数のセッションにまたがる。セッションの数は目安。

## 全体

```mermaid
flowchart LR
  P0[P0 運用基盤] --> P1[P1 骨格] --> P2[P2 遅延クエリと検査] --> P3[P3 シミュレーション] --> P4[P4 Desktop MVP]
  P4 --> P5[P5 FMI] --> P6[P6 Web / marimo / Streamlit / Android] --> P7[P7 言語 API と UI 比較] --> P8[P8 拡充]
  P4 -.余裕ができたら.-> B[Track B 自前コア]
```

## Phase ごとの範囲と完了条件

| Phase | 範囲 | 完了条件 | 目安 |
| --- | --- | --- | --- |
| **P0 運用基盤** | 調査、計画、CLAUDE.md、スキル、セッション記録のフック、スライドと解説書の型 | S001 の成果物がそろう。フックが伏せ字版を出力する。リンク切れが 0 件 | S001 |
| **P1 骨格**（S003 で完了） | workspace 化、mise の rust の targets、cargo-deny、CI（Linux / Windows / wasm）。core / ir / frontend / cli。MSL の取得スクリプト。Python パッケージの改名 | `cargo deny check` が通る。MSL の例題 20 本で `globe check` の結果が Rumoca と一致する。frontend が wasm32 向けにビルドできる。[ADR-0003](adr/0003-toolchain.md) が Accepted になる | 3 回（workspace と CI / アダプタと CLI / wasm の確認） |
| **P2 遅延クエリと検査**（S004〜S008。完了条件は S008 で 3 つとも満たした。S9・S10 は候補のまま） | db、outline、class_index、connect_graph、`assert_simulatable` の S0〜S10 | 空白だけの編集の再検査が 50 ms 未満で、`flat` が再実行されない（salsa のイベントで確かめる）。MSL の全例題に判定が出て、panic は 0 件。S5 が omc と 95% 以上一致する | 4 回 |
| **P3 シミュレーション** | exec、driver、bdf / esdirk34 / tsit45 / dopri / 固定刻み、ResultStore | MSL の例題 153 本以上が実行できる。そのうち 90% 以上が参照結果のファネルに収まる。bdf が wasm で（Node 上で）動く | 5 回 |
| **P4 Desktop MVP** | CodeMirror 6 と LSP、ライブラリツリー、検査パネル、プロット、sigma.js の connect グラフ | ChuaCircuit で開く・検査・実行・プロットが通る。5k ノードのグラフが 30 fps 以上で動く。RSS の基準値を記録する。CI が .deb と .msi を作る | 6 回 |
| **P5 FMI** | Import（ME / CS / SE）とマスター。Export は P5a（Rumoca のテンプレート）の後に P5b（自前のランタイム） | Reference-FMUs が linux64 と win64 で一致する。Export した FMU が FMPy・fmusim・fmuCheck を通る。Clocks（SE）が往復できる | 8 回 |
| **P6 Web / marimo / Streamlit / Android** | WorkerBackend、MSL のパッケージ単位の分割読み込み、anywidget、wheel、APK | ブラウザが CSP `connect-src 'self'` のまま実行できる（サーバーで計算しない）。marimo の WASM で 2 GB 未満に収まる。Android API 34 のエミュレータで検査と実行が通る | 5 回 |
| **P7 言語 API と UI 比較** | C / Zig / Julia、Elixir / Gleam、Python。Leptos 版と Dioxus 版、負荷の測定ハーネス | 参照シナリオが 6 言語で通る。`doc/bench/ui.md` に 3 UI × 3 OS の比較を書く | 6 回 |
| **P8 拡充** | radau5、SUNDIALS、ESDIRK / CERK、SSP 2.0.1、Diagram ビュー、BLT デバッガ | ソルバの表がそろう。SSP が往復できる | 未定 |
| **Track B 自前コア** | B1 rowan の CST と寛容なパーサ → B2 resolve → B3 flatten → B4 DAE。`Frontend` の feature `native` として作る | 段階ごとに Rumoca 以上の数字を出す（parse 566 / flatten 562 / balanced 464 / simulate 153）。`xtask diff-frontend` で回帰が 0 件になったら既定を切り替える | P4 以降、余裕ができてから |

Track B の B1〜B2 は IDE の機能（補完、アウトライン）から先に使える。コンパイルのパイプラインは引き続き Rumoca が受け持つ。ただし AST が別物なので、両者を混ぜるのは「IDE 機能か、コンパイルか」の境目に限る。

## 進み具合

### S002（P1 の 1 回目、2026-10-18）で済んだこと

- ルートを virtual workspace にし、4 つの crate を作った: `globe-mode-core`、`globe-mode-ir`、`globe-mode-frontend`（Rumoca アダプタ）、`globe-mode-cli`（`globe outline` / `globe check`）。
- **Rumoca v0.10.2（`276e82bd`）の `rumoca-compile` が、stable 1.99.1 でビルドできることを確かめた**。
  - ネイティブでも `wasm32-unknown-unknown` でも `cargo check` が通る。
  - これで [ADR-0003](adr/0003-toolchain.md) を Accepted にした。
- `mise.toml` に wasm と Android の target、cargo-deny、`mise run check` / `mise run docs:check` を足した。
- `deny.toml` を作った。
  - ライセンス・bans・取得元はすべて ok。
  - `rumoca-compile` を使えるのはアダプタだけに限った（wrappers）。違反させて検出できることも確かめた。
- CI の雛形（`.github/workflows/ci.yml`）を作った。Linux・Windows・wasm・Python とスライドの検査の各ジョブ。**まだ push していないので、実行は確かめていない**。
- Python パッケージを `globe_mode`（`src/Python/mod/`）に改名した。

### P1 の残り（S003〜）

| 完了条件 | 状態 |
| --- | --- |
| `cargo deny check` が通る | 済（S002） |
| frontend が wasm32 向けにビルドできる | 済（S002、`cargo check`） |
| ADR-0003 が Accepted | 済（S002） |
| MSL の例題 20 本で `globe check` の結果が Rumoca と一致する | 済（S003、20 本中 20 本） |
| CI が GitHub 上で通る | 済（run 39093072719、全ジョブ成功） |

### S003（P1 の 2 回目、2026-10-18）で済んだこと

- `cargo xtask fetch-msl`: MSL 4.1.0 を sha256 で確かめて `data/msl/` に展開する。
- `cargo xtask fetch-rumoca`: Rumoca の CLI v0.10.2 を取得する（照合用、Linux x86_64）。
- `globe check --library data/msl -m <モデル>`: MSL の中のモデルを検査できるようにした。
- `cargo xtask msl-compare`: 例題 20 本で Rumoca の CLI と突き合わせた。**20 本すべて一致**（[報告](sessions/S003-msl-check/msl-compare.md)）。
- CI に `msl` ジョブを足した（全体を検査するときだけ）。
- これで **P1 の完了条件はすべて満たした**。

### S004（P2 の 1 回目、2026-10-18）で済んだこと

- `globe-mode-db`（salsa 0.28.5）を作った。クエリは `outline` → `outline_shape`（位置を落とした防火壁）→ `class_index` → `connect_graph`。
- `globe-mode-graph`: connect グラフを flatten せずに作る。
  - 辺の種類は flow / stream / real / boolean …。
  - depth で部分回路を展開できる。
  - 出力は graphology の JSON。CLI は `globe graph`。
- 検証用のパッケージ `src/Modelica/GlobeModeValidation` を作った。
  - Real の potential / flow、Real と Boolean の信号、Medium の流体ポート、階層、型の誤りのモデルを入れた。
  - OMEdit などでも開けるパッケージの構成にした。
- **空白とコメントだけの編集では、`class_index` と `connect_graph` が再実行されないことをテストで確かめた。**
  - 再計算は中央値 0.20 ms で、目標の 50 ms 未満。
- Rumoca v0.10.2 の制約を見つけた（ライブラリとして読んだクラスから別のライブラリを extends できない）。作業中のパッケージは文書として読んで回避した。

### S005（P2 の 2 回目、2026-10-18）で済んだこと

- **検査（S0〜S6）を salsa のクエリにした**（[ADR-0005](adr/0005-lazy-query-db.md) の結果）。
  - 防火壁 0 の `source_shape`（コメントを除き空白を畳んだ本文）を足した。計画では `outline` を防火壁にしていたが、Rumoca に渡せるのは本文なので直した。
  - クエリは `source_shape` → `check`（S0〜S5）→ `dae_shape`（防火壁 2）→ `structural`（S6）。診断の位置は `shape_map` で元の本文に戻す。
  - **空白とコメントだけの編集では `check` が再実行されない**ことをテストで確かめた。再検査は中央値 0.01 ms。
- **MSL は一度だけ読む**。`Checker` が、MSL を読んだ Rumoca の `Session` を使い回す。読んだあとの別のモデルの検査は 0.12 s。
- `globe-mode-analysis` を作った: Hopcroft–Karp と粗い Dulmage–Mendelsohn 分解（S6、Rumoca に依存しない）。
  - **S6 の判定は、`x` と `der(x)` を 1 つの列にまとめた二部グラフ**で行う。最初は der の形で判定し、MSL の正しい例題 166 本を構造特異にしてしまったので直した。der の形の結果は、S7 の情報 `GM-I7001`（微分インデックスが 1 より大きい見込み）にした。
- **GM-U の振り分けを決めた**（[ADR-0007](adr/0007-unsupported-classification.md)）: Rumoca のコードの規則（`GM-U0001` / `GM-U0002`）と、既知の制約の一覧 50 行（`GM-U0003`）の併用。omc との照合は任意（S005 では未実施。S008 で行い、一覧の 50 本は全部 omc で通った）。
- **MSL の全例題 566 本に判定を出した**（`cargo xtask msl-funnel`、[報告](sessions/S005-check-queries/msl-funnel.md)）。例題の選び方は Rumoca と同じ名前の規則で、本数も Rumoca の基準値と一致した。
  - 最終の結果: OK 369（うち S7 の情報 154）・NG 13（S5 1・S6 12）・GM-U 184・panic 0。S6 で止まった 12 本は、Examples の下にある接地の無い部品モデル（`OpAmpCircuits.*`、`FluxTubes…Components.*`）で、単体で検査すると電位が 1 つ決まらない。omc では未確認。
  - 最初の実行では flatten 532 / DAE 382 / balanced 381 本が通り、Rumoca の基準値（flatten 494 / DAE 305 / balanced 293）を上回った。基準値は Rumoca の別の commit（`b92932c0`）で記録されたものなので、差の内訳は確かめていない。
- 検証用のモデル `Invalid.StructurallySingular`（S6 で止まる）と `Invalid.HighIndexDuplicate`（S6 は通り、S7 の情報が出る）を足した。
- CLI: `globe check` を db 経由にし、S6 と GM-U を表示する。`globe flat` で flat モデルの要約を出す。

### S007（P2 の 3 回目、2026-10-18）で済んだこと

- 範囲は S007 の候補のうち `deps_closure`（1）、S7（3）、S8（4）。**omc との照合（5）はユーザーの指示で後回し**にした（omc 未導入。OpenModelica のサイトが重くて取得できない。入れたら再開する）。`Checker` のメモリ（2）は P4 のプロセス構成が決まってから、上流への報告（6）はユーザーが出す。`class_index` の残り（7）は import だけを扱った。
- **`deps_closure`**: `check` に渡すファイルを、モデルが使うものだけに絞った（[ADR-0005](adr/0005-lazy-query-db.md) の結果）。
  - 識別子の単純名と、ファイルが定義するトップレベルのクラスの名前を突き合わせ、固定点まで広げる。広めに見積もり、足りないことは無い。
  - `Connect.RealCircuit` の閉包は 5 / 23 ファイル。閉包だけの検査と全ファイルの検査は、検証用パッケージのモデル 20 本以上のすべてで一致した。
  - 使わないファイルの式を変えた後の再検査は、`check` 0 回で中央値 0.19 ms（MSL を使う `MediumFlow`、release）。代わりに、閉包の違うモデルへ初めて切り替えると、Rumoca が MSL ごと解決し直すので約 1.7 s かかる（全ファイルを渡したままなら 67 ms）。
  - CLI: `globe deps <files> -m <model>` で閉包を見られる。
- **S7（Pantelides 法と状態選択の候補）**と **S8（初期化系の検査）**を足した（[ADR-0009](adr/0009-structural-index-and-init.md)）。
  - S7 は情報にとどめる（`GM-I7001` インデックス 2 以上、`GM-I7002` 状態選択が要る）。打ち切りだけ `GM-E7001`。
  - S8 は初期条件の数と属性で判定する。要る数には S7 の動的な状態の数を使う。過剰は `GM-E8001`（離散変数を含むか、インデックスの低減で状態が減るときは、確定できないので警告 `GM-W8004`）、不足は `GM-W8001`・`GM-W8002`、値の無いパラメータは `GM-W8003`。
  - 検証用のモデル `Index.Pendulum`（インデックス 3、動的な状態 2 / 4）、`Index.ParallelCapacitors`（インデックス 2、動的な状態 1 / 2）、`Init.*` の 5 本を足した。`Invalid.HighIndexDuplicate` は、動的な状態が 0 で `fixed = true` が 1 つ余るので、S8 の警告 `GM-W8004` を出して通る。
  - `globe check` に S7・S8 の行と `--json` の `s7` / `s8` / `init`。`check_model` は S0〜S8。
- `msl:funnel` の報告の置き場を [msl-funnel.md](msl-funnel.md) に固定した。S7・S8 の列と集計を足した。
  - 最初の実行で、MSL の正しい例題 3 本（インデックス 2 で、拘束でつながった状態すべてに `fixed = true`）が S8 の Error になったので、過剰の規則を直した（[ADR-0009](adr/0009-structural-index-and-init.md)）。
  - 最終の結果: 566 本: OK 369（うち S7 の情報 154）・NG 13（S5 1・S6 12・S7 0・S8 0）・GM-U 184（82 / 52 / 50）・panic 0。OK・NG の本数は S005 と同じ。S7 を回した 369 本の構造インデックスは 1 が 215・2 が 58・3 が 95・4 以上が 1・打ち切り 0、状態選択が要るもの 154。S8 の 369 本は合う 111・過剰の見込み 3（`GM-W8004`。Error 0）・不足 255・入力なし 0、`GM-W8003` あり 2。S6 と Rumoca の照合 381 / 381。検査 833.0 s（1 本 1.47 s）、最大 RSS 7.2 GiB。
- テストは workspace 全体で 209 本が通過。clippy 0 件、fmt の差分 0。

### S008（P2 の 4 回目、2026-10-19）で済んだこと

- 範囲は S008 の候補のうち **omc との照合（1）だけ**。ユーザーが OpenModelica 1.27.1 を入れたので、S007 で後回しにした項目を最初に再開した。方法は [ADR-0010](adr/0010-omc-comparison.md)、報告は [omc-compare.md](omc-compare.md)。
- **`cargo xtask omc-compare`**（`mise run msl:omc`）: `msl-funnel --json` の結果を入力に、omc を別プロセスで呼んで `checkModel`（方程式数・変数数）と `translateModel`（バックエンドまで）の結果を突き合わせる。omc に読ませる MSL は `data/msl` の素の 4.1.0（omc 同梱の `+maint.om` ではない）。50 本ごとに 1 プロセス、落ちたら 1 本ずつ再実行。`--reuse` で omc を呼ばずに照合だけやり直せる。
- **結果（566 本、omc は 391 s）**: S5 は判定のある 382 本すべてで一致（**100%。P2 の完了条件を達成**）。GM-U 184 本は全部 omc の `checkModel` で数が合い、既知の制約の一覧（`GM-U0003`）の 50 本は全部 omc で通った。S6 は 363 本中 362 本、S8 の過剰は 355 本中 354 本が一致。globe が S6 で止めた 12 本のうち 7 本は omc も特異（残り 5 本は omc が手前の「fixed なパラメータに値が無い」で止まる）。globe の `GM-W8004` の 3 本は omc も「over specified」で、余分な本数まで同じ。
- 食い違い 2 本（`AdvancedSolenoid`: globe は通し omc は Pantelides の内部エラー。`NonCircularPipes`: globe は不足、omc は冗長な初期方程式 2 本）は S009 の候補にした。
- `msl:funnel` は JSON（`target/msl-funnel.json`、`counts.unknowns` を追加）も残すようにした。ファネルを回し直した本数は S007 と同じ（OK 369・NG 13・GM-U 184・panic 0）。

### P2 の完了条件の進み具合

| 完了条件 | 状態 |
| --- | --- |
| 空白だけの編集の再検査が 50 ms 未満で、下流が再実行されない | **済**（S005）。S0〜S6 の再検査が中央値 0.01 ms（最大 0.02 ms、リリースビルド、20 回）。`check`（Rumoca の flatten を含む）は再実行されない（salsa のイベントで確かめた） |
| MSL の全例題に判定が出て、panic は 0 件 | **済**（S005）。566 本すべてに判定。panic 0 件。S007 で S0〜S8 にして回し直した: OK 369・NG 13・GM-U 184・panic 0（S005 と同じ本数。S7・S8 の Error は 0） |
| S5 が omc と 95% 以上一致する | **済**（S008）。omc 1.27.1 の `checkModel` と、判定のある 382 本すべてで一致（100%）。方程式数と未知数が同じ数なのは 307 本（違いは `Clocked` のクロック変数など、両側に同じだけ付く）。S6 は 363 本中 362 本、S8 の過剰は 355 本中 354 本が一致。報告は [omc-compare.md](omc-compare.md)、方法は [ADR-0010](adr/0010-omc-comparison.md) |

## 次のセッション

**S009（P2 の 5 回目）** の候補:

1. **omc と食い違った 2 本を調べる**（[omc-compare.md](omc-compare.md)）: `SolenoidActuator.Components.AdvancedSolenoid`（globe は S6 を通し S7 でインデックス 2、omc は Pantelides の内部エラー。omc の `-d=bltdump` と globe の S6・S7 の詳細を見る）と、`Fluid.Examples.NonCircularPipes`（globe は不足、omc は冗長な初期方程式 2 本。下の 3 と同じ話）。omc が手前の「fixed なパラメータに値が無い」で止めるモデル（globe の `GM-W8003` 相当）を S6 より前で報告するかも決める。
2. **閉包の切り替えの費用を和らげる**: `deps_closure` で文書の組が変わると、Rumoca が MSL ごと解決し直す（約 1.5〜1.8 s）。`Checker` に文書の和集合を持たせて、組が変わらないようにする、など。
3. **S8 を精密にする**: 初期方程式の incidence を持ち、初期化系をマッチングする（どの初期方程式がどの変数を決めるか）。S7 で微分した拘束と合わせて組む。
4. **離散変数の when の区別**: when の外の式で決まる離散変数（MLS 8.6 の暗黙の `pre(v) = v`）か、when の中で決まるかを分け、S8 の `GM-W8004` と柔らかい `GM-W8001` を確定させる。
5. **S9（代数ループ）・S10（プリフライト）**: 非線形の反復変数の start 値、パラメータの評価、t0 での残差、experiment annotation。
6. **`Checker` のメモリの上限**（P4 のプロセス構成が決まってから）: MSL の例題 566 本で最大 RSS 7.2 GiB（大部分は 1 本の大きなモデル）。大きなモデルの扱いと `reset()` を呼ぶ条件。
7. **Rumoca の制約を上流に報告する**（ユーザーが出す）: ライブラリ間の extends（S004）、GM-U に振り分けたもの（ED001 の数え方、ET004 の次元など）。
8. **`class_index` の残り**: for や if の中の connect、継承したクラスの中の入れ子のクラス（import は S007 でアウトラインに持たせた）。

## 運用上の決まり

- 各セッションでは `doc/sessions/SNNN-<topic>/` に `slides.html` と `解説書.md` を作る（[sessions/README.md](sessions/README.md)）。
- 計画を変えたら、この文書と該当する ADR を同じコミットで直す。
