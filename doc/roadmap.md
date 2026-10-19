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
| **P2 遅延クエリと検査**（S004〜S010。完了条件は S008 で 3 つとも満たした。S9・S10 は S010 で足した。S10 の数値の部分は P3） | db、outline、class_index、connect_graph、`assert_simulatable` の S0〜S10 | 空白だけの編集の再検査が 50 ms 未満で、`flat` が再実行されない（salsa のイベントで確かめる）。MSL の全例題に判定が出て、panic は 0 件。S5 が omc と 95% 以上一致する | 4 回 |
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

### S009（P2 の 5 回目、2026-10-19）で済んだこと

- 範囲は S009 の候補のうち **omc と食い違った 2 本の調査（1）** と **S8 の精密化（3）**。1 の調査で `NonCircularPipes` の食い違いが「数だけの S8 は冗長を見られない」ことだったので、3 をそのまま主な実装にした。方法は [ADR-0011](adr/0011-init-system-matching.md)。
- **`AdvancedSolenoid` は S6 の限界**（直さない）: 接地の無い部品の単体の検査で、枝 8 本 ≥ 節点 7 つなので二部グラフには完全マッチングがあるが、キルヒホッフの電流則が数値的に従属（電位は差しか決まらない）。omc は記号的な簡約の後に `0.0 = 0.0` を見つけて止める（`-d=bltdump`）。構造の検査では捕まえられないので、S10 のプリフライトで初期化のヤコビ行列の階数を見る候補にした。検証用のモデル `Invalid.UngroundedLoop` に記録（`Connect.Subsystem` のように枝 < 節点 なら構造でも特異で、S6 で止まる）。
- **S8 の精密化**: 初期化系（DAE とその微分 + 初期方程式 + `fixed = true`）を二部グラフにして、最大マッチングと DM 分解で**冗長な初期条件**（過剰な部分）と**初期条件の無い未知数**（不足している部分）を名指しする（`globe-mode-analysis/src/init_system.rs`）。数だけの判定は「不足 − 冗長」しか分からなかった。
  - `InitStructure` に初期方程式の参照（`der(x)`・代数、状態そのもの、パラメータ、離散）と、初期化で決めるパラメータへの DAE の方程式からの参照を足した。フロントエンドは `structure.rs` と同じ鍵で写す。
  - 過剰は、過剰な部分に DAE の行（拘束やその微分）があれば `GM-W8004`（整合する値なら冗長なだけ。omc も Warning で外す）、無ければ `GM-E8001`。不足は `GM-W8001` と、不足している部分の start の無い状態に `GM-W8002`。両方あれば両方出す。
  - `globe check` の S8 にマッチングの行、`--json` の `s8.matching`。`msl-funnel` の S8 の列に「冗長 k・決まらない m」、`omc-compare` の S8 の照合は「過剰 = 冗長が 1 つ以上」。
  - 検証用のモデル `Init.RedundantAndMissing`（数は 1 / 1 で合うが、冗長 1・決まらない 1）。
- **Rumoca v0.10.2 の制約を 2 つ見つけた**（どちらも S8 の入力が欠けたり誤ったりする。GlobeMode 側では直せないので記録し、上流への報告の候補にした）。(1) `outer` の参照を `inner` の修正ではなく型の既定で評価する。MSL の Fluid の例題 11 本は `inner Modelica.Fluid.System system(energyDynamics = FixedInitial)` の形なので、`PartialDistributedVolume` の `if` の中の初期方程式（`mediums.p = ps_start` など）が globe の DAE に無く、S8 が「不足」になる。`NonCircularPipes` で omc が冗長と言う 2 本はこれ（globe の入力に無い）。検証用のモデル `Init.OuterModification` と db のテスト（Rumoca が直ったら落ちる）に記録。(2) 例題の `wheelSet(theta1(fixed = true))` のような修正が、部品の中の同名の `wheelSetJoint.theta1`（MSL は `fixed = false` で外している）にも漏れる。`RollingWheelSetDriving` / `RollingWheelSetPulling` で S8 が「冗長 7」を出し、omc（過剰と言わない）と食い違うのはこのため。検証用のモデル `Init.ModifierScope` と db のテストに記録。
- **MSL の全例題**（`mise run msl:funnel`、[msl-funnel.md](msl-funnel.md)）: 566 本: OK 369・NG 13（S5 1・S6 12）・GM-U 184・panic 0（S007・S008 と同じ本数）。S8 まで来た 369 本すべてで初期化系をマッチングできた（射影の失敗 0）。S8 の分布は合う 111・過剰 6（すべて `GM-W8004` の見込み。Error 0）・不足 252（S008 は 合う 111・過剰 3・不足 255。冗長と不足を両方持つ 3 本が過剰に移った）。検査 723.8 s（1 本 1.28 s）、全体 748 s、最大 RSS 7.1 GiB。
- **omc との照合**（`cargo xtask omc-compare --reuse`。omc は S008 の結果を再利用、[omc-compare.md](omc-compare.md)）: S5 382 / 382、S6 362 / 363（`AdvancedSolenoid`。上の限界）、**S8（過剰）352 / 355**（S008 は 354 / 355）。不一致 3 本はどれも Rumoca の制約で入力が欠けるか誤るもの: `NonCircularPipes`（inner/outer で初期方程式が消え、omc の冗長 2 本が globe の入力に無い）、`RollingWheelSetDriving` / `RollingWheelSetPulling`（修正の漏れで別名の両方が `fixed = true` になり、globe が冗長 7 を出す）。globe の `GM-W8004` の 3 本（`GenerationOfFMUs` 2 本、`CompareLineTrunks`）は omc も過剰で、冗長の本数（2・3・1）まで一致した。
- テストは workspace 全体で 232 本が通過（S008 は 216 本）。clippy 0 件、fmt の差分 0。

### S010（P2 の 6 回目、2026-10-19）で済んだこと

- 範囲は S010 の候補のうち **S9・S10（3）**、**離散変数の when の区別（1）**、**閉包の切り替えの費用（2）**。5 つのサブエージェントが所有するファイルを分けて同時に作業した（S9 の BLT と tearing、S9 の線形・非線形の判定、S10、閉包、S8 の when）。`Checker` のメモリ（4）は P4 のプロセス構成が決まってから、上流への報告（5）はユーザーが出す。
- **S9（代数ループ）**（[ADR-0012](adr/0012-loops-and-preflight.md)）: BLT（Hopcroft–Karp と再帰を使わない Tarjan 法）で大きさ 2 以上のブロックを見つけ、貪欲な tearing で反復変数を選び、非線形のループの反復変数に start 値が無ければ `GM-W9001`（警告）。ループがあれば `GM-I9001`（情報）。インデックス 2 以上のモデルは Pantelides 法の最高階の系で数える（状態選択の前の見込み）。線形・非線形は、フロントエンドが残差の式木を族の単位で広めに判定する（`DaeStructure::nonlinear`）。
- **S10（プリフライトの静的な部分）**（[ADR-0012](adr/0012-loops-and-preflight.md)）: パラメータと start 値を Rumoca の評価器で評価し、非有限は `GM-E10001` / `GM-E10003`、関数の中の assert の失敗は `GM-E10002`。experiment annotation は Rumoca が不正な値を黙って捨てるので、根のクラスの本文を読み直して生の値で判定する（`GM-E10004` / `GM-W10001` / `GM-I10001`）。t0 の残差・初期化のヤコビ行列の階数・ソルバの能力は P3 の実行系ができてから。
- **S8 の離散変数の when の区別**（[ADR-0011](adr/0011-init-system-matching.md) の S010 の追記）: when の中で決まる・when の外の式で決まる・`when initial()` で決まるを分け、初期化でも成り立つ離散の方程式を行に足した（MLS §8.6）。決まり方の分かる離散変数の不足と冗長は確定させた。
- **閉包の切り替えの費用**（[ADR-0005](adr/0005-lazy-query-db.md) の結果）: `Checker` に文書の和集合を持たせた。一度検査したモデルへ編集なしで戻るときは約 2.1 s → **49 ms**。Rumoca の Session にはライブラリの解決だけを残す道が無いので、文書の組を変えないようにした。閉包の違うモデルを初めて検査するときは今も約 1.7〜2.0 s。
- 検証用のモデル 20 本: `Loops.*` 4 本、`Preflight.*` 10 本、`Init.Discrete*` 6 本（[src/Modelica/README.md](../src/Modelica/README.md)）。
- **MSL の全例題**（`mise run msl:funnel`、[msl-funnel.md](msl-funnel.md)）: 566 本: OK 369・NG 13（S5 1・S6 12）・GM-U 184・panic 0（S009 と同じ本数）。S8 の分布は **合う 269**・過剰 6（すべて `GM-W8004`）・**不足 94**（S009 は 111・6・252。離散変数の区別で StateGraph などの「不足」が「合う」に移った）。S9 は 369 本すべてで解析し、代数ループのあるもの 226・非線形のループのあるもの 142・`GM-W9001` のあるもの 57（反復変数 188 個）、一番大きいループは `MultiBody.Examples.Loops.EngineV6` の 4584 本（最高階の系）。**ループの大きさの一覧は Rumoca の構造解析と、微分の要らない 215 本すべてで一致**。S10 は Error 0（experiment annotation の無い 16 本に `GM-I10001`）。検査 849.8 s（1 本 1.50 s。S009 は 1.28 s。後半に `msl:compare` を同時に回したので揺れを含む）、最大 RSS 7.1 GiB。
- **omc との照合**（`mise run msl:omc`、omc を回し直した。[omc-compare.md](omc-compare.md)）: S5 382 / 382、S6 362 / 363、S8（過剰）352 / 355（S009 と同じ）。**S9（非線形のループの有無）282 / 355（79.4%）、S9（start の無い非線形の反復変数の有無）312 / 355（87.9%）**。omc は別名の消去と式の変形を先にするので有無だけを比べた。食い違いの主な見込みは、線形の判定が方程式ごとの集合しか持たないこと（`v = R*i` の `R` がループの外で決まっても非線形とみなす。`ShowVariableResistor`）。
- テストは workspace 全体で 324 本が通過（S009 は 232 本）。clippy 0 件、fmt の差分 0。`msl:compare` は 20 / 20。

### P2 の完了条件の進み具合

| 完了条件 | 状態 |
| --- | --- |
| 空白だけの編集の再検査が 50 ms 未満で、下流が再実行されない | **済**（S005）。S0〜S6 の再検査が中央値 0.01 ms（最大 0.02 ms、リリースビルド、20 回）。`check`（Rumoca の flatten を含む）は再実行されない（salsa のイベントで確かめた） |
| MSL の全例題に判定が出て、panic は 0 件 | **済**（S005）。566 本すべてに判定。panic 0 件。S007 で S0〜S8 にして回し直した: OK 369・NG 13・GM-U 184・panic 0（S005 と同じ本数。S7・S8 の Error は 0） |
| S5 が omc と 95% 以上一致する | **済**（S008）。omc 1.27.1 の `checkModel` と、判定のある 382 本すべてで一致（100%）。方程式数と未知数が同じ数なのは 307 本（違いは `Clocked` のクロック変数など、両側に同じだけ付く）。S6 は 363 本中 362 本、S8 の過剰は 355 本中 354 本が一致。報告は [omc-compare.md](omc-compare.md)、方法は [ADR-0010](adr/0010-omc-comparison.md) |

## 次のセッション

**S011** の候補（P2 の S0〜S10 は、S10 の数値の部分を除いてそろった。P3 に進むかどうかを決める）:

1. **P3 の始め（実行系 `globe-mode-exec` と driver）**: Rumoca の実行の道具（`rumoca-sim` など）を `globe-mode-exec` から使うか、自前の評価器を作るかを決める（ADR）。そこで S10 の数値の部分（t0 の残差が有限か、**初期化のヤコビ行列の階数**: S009 の `Invalid.UngroundedLoop`、MSL の `AdvancedSolenoid`、ソルバの能力）も入れる。
2. **S9 を omc と照らして詰める**: `omc-compare` の S9 の照合（非線形のループの有無と、start の無い反復変数の有無。[omc-compare.md](omc-compare.md)）の食い違いを調べる。omc は別名の消去と式の変形を先にするので、globe のループが多めに出る見込み。tearing の選び方（start 値を優先）と、`abs`・`min`・`max` を非線形とみなす広めの判定の影響を切り分ける。状態選択（dummy derivative）の後の系で数えるかも決める。
3. **閉包の切り替えの残り**: ワークスペースの全ファイルを最初に一度だけ Session に入れておき、初めて検査するモデルも速くする（[ADR-0005](adr/0005-lazy-query-db.md) の結果）。
4. **`Checker` のメモリの上限**（P4 のプロセス構成が決まってから）: MSL の例題 566 本で最大 RSS 7.1 GiB。
5. **Rumoca の制約を上流に報告する**（ユーザーが出す）: ライブラリ間の extends（S004）、GM-U に振り分けたもの（ED001 の数え方、ET004 の次元など）、inner の修正が outer に伝わらない（S009）、外からの修正が部品の中の同名の変数に漏れる（S009）、**`phase_structural::analyze` が `algebraic_loops` を埋めない**（S010）。
6. **字面の 0 除算の振り分け**: `parameter Real T = 1 / a`（a = 0）は Rumoca が `ED019` で止め、[ADR-0007](adr/0007-unsupported-classification.md) のコードの規則で `GM-U0001`（未対応）になる。モデルの誤りなので、文言（"division by zero"）で GM-E に戻すかを決める。
7. **S8 の残り**: アルゴリズムの `when {…, initial()}` で読み出しを `pre` にする形（MSL の `Interaction1`。omc は何も言わない）、DAE の連続の方程式の中の `pre(d)`、クロックの区間の変数（`Unknown` のまま）。
8. **`class_index` の残り**: for や if の中の connect、継承したクラスの中の入れ子のクラス。

## 運用上の決まり

- 各セッションでは `doc/sessions/SNNN-<topic>/` に `slides.html` と `解説書.md` を作る（[sessions/README.md](sessions/README.md)）。
- 計画を変えたら、この文書と該当する ADR を同じコミットで直す。
