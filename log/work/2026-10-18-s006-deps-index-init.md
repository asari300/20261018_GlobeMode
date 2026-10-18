# S006 deps-index-init

| | |
| --- | --- |
| 日付 | 2026-10-18 |
| ブランチ | `feature/s006-deps-index-init` |
| worktree | `.git-worktree/s006-deps-index-init` |
| Phase | P2 遅延クエリと検査（3 回目） |
| セッション文書 | [doc/sessions/S006-deps-index-init/](../../doc/sessions/S006-deps-index-init/解説書.md) |
| セッション記録 | [log/sessions/2026-10-18-c74fbe25-624e-4092-95b3-282cf3c8d554.md](../sessions/2026-10-18-c74fbe25-624e-4092-95b3-282cf3c8d554.md)（S005 と同じセッション） |

## 依頼

「S006 を進めよ。OpenModelica のサイトが重くてアクセスできないため、omc との照合は後回しにして先に進めたい。インストール後に再開するため、このような区切りごとにリマインドしてください。」

- roadmap の S006 の候補 7 つのうち、omc（5）を外し、`deps_closure`（1）、S7 の Pantelides 法（3）、S8 の初期化（4）を選んだ。
- **omc は未導入**。入れたら `msl:funnel` の omc 照合（S5 の一致率、S6 で NG の 12 本の確認）を再開する。

## 決めたこと

- **S006 の範囲は `deps_closure`・S7（Pantelides 法）・S8（初期化系）の 3 つ。** roadmap の候補 7 つのうち、omc との照合（5）はユーザーの指示で後回し（omc 未導入。OpenModelica のサイトが重くて取得できない）。`Checker` のメモリの上限（2）は IDE のプロセス構成（P4）が決まってから、Rumoca への上流報告（6）はユーザーが出すもの、`class_index` の残り（7）は import だけを `deps_closure` のついでに扱う。
- **`deps_closure` は「識別子の単純名の一致」で広めに見積もる（ファイル単位）。** アウトライン（extends・部品の型・import）だけでは、式の中の関数・定数・redeclare の参照を拾えない。そこで、閉包の中のファイルの本文（`source_shape`）に現れる識別子と、各ファイルが定義するクラスの単純名（`outline_shape` から。防火壁 1 の下流）を突き合わせ、固定点まで広げる。余計に入ることはあるが、足りないことは無い（別のトップレベルのパッケージを参照するときは必ずその名前を綴るため）。モデルがワークスペースに無い（ライブラリの中、または存在しない）ときは文書を 1 つも渡さない。
- **モデルを切り替えたときの Rumoca の再 resolve は新しいコストではない。** 文書の集合が変わると Rumoca は解決済みの状態を捨てるが、本文の編集でも同じことが起きるので、IDE の打鍵ごとのコストの種類は増えない。数字は A が測る。
- **S7 は Pantelides 法を `DaeStructure`（der の形の incidence と `state_refs`）の上で回す。** 状態 `x` は階数 0 と 1（`der(x)`）から、代数は階数 0 から始めて、最高階の未知数に完全マッチングができるまで方程式を微分する。結果は情報（`GM-I7001` に構造インデックスの見込みと微分の内訳、`GM-I7002` に状態選択の候補）で、実行は止めない。打ち切りだけ `GM-E7001`（S6 を通った構造では起きないはず）。S6 の `StructuralReport` から `GM-I7001` を外し、S7 の診断は `IndexReport` に一本化する。
- **S8 は数と属性だけで判定する（初期方程式の incidence は持たない）。** 最初の案は「要る初期条件 = 状態 + `fixed = false` のパラメータ、与えられた初期条件 = 初期方程式 + `fixed = true` の状態。多ければ `GM-E8001`、少なければ `GM-W8001` と `GM-W8002`、束縛の無いパラメータは `GM-W8003`。離散変数は数に入れない」だったが、下の 3 つの項目のとおり作業の途中で直した（最終の規則は [ADR-0008](../../doc/adr/0008-structural-index-and-init.md)）。初期化系のマッチングは S007 以降の候補。
- **S8 の「要る初期条件の数」は、宣言した状態の数ではなく S7 の動的な状態の数を使う（作業の途中で直した）。** C が A の配線後の CLI で見つけた: `Index.Pendulum`（状態 4、自由度 2、fixed 2）に「2 個足りない」、`Index.ParallelCapacitors`（状態 2、自由度 1）に「1 個足りない」と誤った警告が出た。インデックスが 2 以上の DAE では、拘束で自由度が減るため。`analyze_init_with_index(init, dynamic_states)` を足し、db の `init_report` は `index_report` の `dynamic_states` を渡す（S7 が打ち切りなら状態の数で代用）。
- **動的な状態の数の式も直した。** 最初の案「状態の数 − Σ(状態の階数 − 1)」は並列コンデンサ（`v1 = v2` を 1 回微分するだけで階数の上がる族が無い）で 2 になって誤り。正しくは「状態の数 + Pantelides が新しく作った微分のノードの数 − 方程式の微分回数の合計」（拡張した系の未知数 − 方程式 = 積分器の数）。振り子 4 + 2 − 4 = 2、並列コンデンサ 2 + 0 − 1 = 1、`HighIndexDuplicate` 1 + 1 − 2 = 0（構造的には vC が代数的に決まるので、`fixed = true` が 1 つ余る。最初は S8 の `GM-E8001` で止めたが、下の過剰の項目のとおり警告 `GM-W8004` に落とした）。状態選択の候補は「1 回以上微分した方程式が参照する状態」にし、`GM-I7002` は動的な状態 < 状態の数のときに出す。
- **S8 の過剰を Error にするのは確定できるときだけ（ファネルの結果で直した）。** 離散・代数の fixed を数える修正のあと、最初の全例題のファネルで MSL の正しい例題 3 本（`Electrical.Analog.Examples.GenerationOfFMUs` 8/6、`Lines.CompareLineTrunks` 60/57、`Thermal.HeatTransfer.Examples.GenerationOfFMUs` 4/3。いずれもインデックス 2 で、拘束でつながった状態すべてに `fixed = true`）が S8 の `GM-E8001` になった。インデックスの低減で状態が減るとき、余った fixed が拘束と矛盾しない（整合する冗長な初期条件）かは数値でしか決まらない。そこで `GM-E8001` は「離散変数が無く、動的な状態 = 状態の数」のときだけにし、それ以外は警告 `GM-W8004`（確定できない）にした。`Invalid.HighIndexDuplicate` も S8 で止まらず警告で通る（README と .mo のコメントを戻した）。離散変数の分の不足も同じ考えで柔らかい `GM-W8001` にし、`GM-W8002` を出さない（when の外の式で決まる離散変数は MLS 8.6 の暗黙の `pre(v) = v` で初期条件が要らないが、`InitStructure` では区別できない）。
- **`msl:funnel` の報告の置き場を `doc/msl-funnel.md` に固定した。** S005 では `doc/sessions/S005-check-queries/msl-funnel.md` に書いていたが、セッションの文書はそのセッションの間だけ編集するものなので、毎回動かすことになる。S005 の報告はそのまま残す。
- **並列化は S005 と同じ（1 つの worktree、所有ファイルを分けた Opus のサブエージェント 3 つ）。** A: `deps_closure` と db の S7・S8 の配線、CLI。B: Pantelides 法、`Index.*` の検証モデル、funnel の S7・S8 の列。C: S8 の抽出（frontend）と判定（analysis）、`Init.*` の検証モデル。共有の型（`ClassOutline.imports`、`InitStructure`、`IndexReport`、`InitReport`、コード）は先に凍結した。文書は D。
- **omc は未導入。** 入れたら `msl:funnel` の omc 照合（S5 の一致率 95% の判定、S6 で NG の MSL 12 本の確認）を再開する（次のセッションの計画に残す）。

## やったこと

- **db（A）**: `deps_closure`（`src/Rust/mod/globe-mode-db/src/lib.rs`、`deps.rs`）。
  - クエリ `identifiers(file)`（`source_shape` から識別子の集合。文字列は除く）、`defined_classes(file)`（`outline_shape` からトップレベルのクラスの単純名）、`class_files(workspace)`（名前 → ファイル。構文の誤りのあるファイルは常に閉包に入れる）、`deps_closure(workspace, request)`（固定点。単純名の一致で広めに。モデルがワークスペースに無ければ空）。`check` は閉包のファイルだけを Rumoca に渡す。
  - 入れ子のクラスを `defined_classes` に入れない（`Components.mo` の `replaceable package Medium` が `MediumFlow.mo` の入れ子の `Medium` と同名で、無関係のファイルが入るため）。トップレベルだけで足りる根拠は doc コメントに書いた。
  - S7・S8 の配線: `index_report`（`dae_shape` → `analyze_index`）、`init_shape`（`check.init` の `without_ranges`。防火壁）、`init_report`（`init_shape` と `index_report` の `dynamic_states` → `analyze_init_with_index`）。`FullReport` に `index` / `init`。`check_model` は S0〜S8（S6 が regular のときだけ S7、S0〜S7 に Error が無いときだけ S8）。
  - テスト `tests/deps.rs`（閉包、防火壁、S7・S8 の配線、計測の無視指定 1 本）と `tests/stages.rs`（端から端まで 11 本）。
- **ir**: `ClassOutline.imports`（`import A.B.C;` / `import D = A.B.C;` / `import A.B.*;` / `import A.B.{C,D};`）、`Outline::defines`、`InitStructure`・`InitVariable`。
- **cli（A）**: `globe check` に S7・S8 の行と `--json` の `s7` / `s8` / `init`。新しい `globe deps <files> -m <model> [--json]`（例 `5 / 23 ファイル`）。
- **analysis（B）**: `pantelides.rs`（Pantelides 法。族ごとの階数つきノード、最高階だけをマッチング、der の形の Hopcroft–Karp から始め、交互路はスタック。`IndexReport`）。`StructuralReport::diagnostics` から `GM-I7001` を外した。`testutil.rs` に乱択の構造。
- **analysis・frontend（C）**: `globe-mode-analysis/src/init.rs`（`analyze_init`、`analyze_init_with_index`、`InitReport`）と `globe-mode-frontend/src/init.rs`（Rumoca の DAE から `InitStructure` を取り出す。明示された start の判定 `explicit_start`、`initial_parameter_values()` と `initial_discrete_values()` を初期方程式に足す）。テスト `globe-mode-frontend/tests/init.rs`。
- **core**: `codes` に `GM-I7002`、`GM-E7001`、`GM-E8001`、`GM-W8001`〜`GM-W8004`。
- **xtask（B）**: `funnel.rs` に S7・S8 の列。OK の判定を S0〜S8 に。集計に構造インデックスの分布、状態選択、S8 の分布、`GM-W8003`。
- **Modelica**: `Index.Pendulum`・`Index.ParallelCapacitors`（B）、`Init.Balanced`・`Init.OverDetermined`・`Init.UnderDetermined`・`Init.ParameterWithoutValue`・`Init.FreeParameter`（C）。`package.order` に `Index`・`Init`。`Invalid.HighIndexDuplicate` のコメントと `src/Modelica/README.md` の表を直した（S8 の `GM-E8001` で止まる）。
- **mise.toml**: `msl:funnel` の報告の置き場を `doc/msl-funnel.md` に固定。
- **文書（D）**:
  - ADR-0008（S7・S8）を書き、ADR の索引に足した。ADR-0005 に S006 の結果（`deps_closure` の設計、クエリの連鎖、計測とトレードオフ、テスト）を書き足した。
  - architecture（crate の表、§3.2 のクエリの図と表、検査の段階の S7・S8 の行、§7）、roadmap（S006 で済んだこと、P2 の進み具合、S007 の候補）、features（依存の閉包、S7、S8）、research/rumoca（S006 で調べた API）、README の Status、CLAUDE.md §2（`msl:funnel` の報告の置き場、S0〜S8）を更新した。
  - S006 のスライド（15 枚）と解説書。

## 検証

- Rust のテスト: workspace 全体で 209 本が通過（analysis 68、db 7 + 11 + 16 + 11 + 13、frontend 23 + 20 + 9、ir 6、core 13、xtask 12）。clippy の警告 0 件、fmt の差分 0。`mise run check` の通し（`test_ci_plan.py`、fmt、clippy、test、cargo-deny 4 項目 ok、`uv sync --locked`）は exit 0。wasm32 の `cargo check` は core / ir / frontend / graph / db / analysis の 6 crate で通過。`mise run msl:compare` は 20 本中 20 本一致（両方とも成功 17 本）。
- `deps_closure`:
  - `deps_closure(GlobeModeValidation.Connect.RealCircuit)` = 5 / 23 ファイル（`package.mo`、`Connectors.mo`、`Components.mo`、`Connect/package.mo`、`Connect/RealCircuit.mo`）。`Index.Pendulum` は 3 / 23（`globe deps` で確かめた）。
  - 閉包だけの検査と全ファイルの検査が、検証用パッケージのモデルとブロック 20 本以上のすべてで一致。
  - 使わないファイル（`Invalid/StructurallySingular.mo`）の式を変えても `source_shape`・`deps_closure`・`check`・`structural` は 0 回。閉包の中のパラメータを変えると `check` 1 回、`index_report`・`init_report` は 0 回。
- 計測（release、MSL 4.1.0、各 5 回。`cargo test --release -p globe-mode-db --test deps -- --ignored --nocapture`）:

  | 場合 | 中央値 | 最大 |
  | --- | ---: | ---: |
  | MSL の読み込み | 3822 ms（1 回目 13096 ms） | – |
  | (a) MediumFlow の最初の検査 | 1963 ms | – |
  | (b) 使わないファイル（StructurallySingular.mo）の式を変えた後の再検査（`check` 0 回） | 0.19 ms | 0.35 ms |
  | (c) Components.mo（閉包の中）を変えた後 | 1815 ms | 1879 ms |
  | 参考: 全ファイルを渡して (c) と同じ編集（S005 の方式） | 1746 ms | 1856 ms |
  | (d) 編集の後に MediumFlow → RealCircuit | 1485 ms | 1543 ms |
  | (e) 編集なしで、閉包の違うモデルを初めて検査 | 1700 ms | 2851 ms |
  | 参考: 全ファイルを渡したまま (e) と同じモデル | 67 ms | 122 ms |

  - 得たもの: 使わないファイルの編集が約 1.8 s → 0.2 ms。失ったもの: 閉包の違うモデルへ初めて切り替えると 67 ms → 約 1.7 s（2 回目からは salsa が覚える）。ADR-0005 にトレードオフとして書いた。
- S7: テスト 17 本 + funnel 1 本。乱択 3000 件で「`index_reduction_needed` ⇔ `index >= 2`」、乱択 1916 件で微分回数が Pryce の Σ 法の最小オフセットと一致。
- 検証用モデル（`target/debug/globe check`。S8 は「与えた数 / 要る数」）:
  - `Index.Pendulum`: インデックス 3、微分 [1, 1, 0, 0, 2]、候補 x, y, vx, vy、動的な状態 2 / 4、S8 2 / 2、終了コード 0。
  - `Index.ParallelCapacitors`: インデックス 2（`v1 = v2` を 1 回）、動的な状態 1 / 2、S8 1 / 1、終了コード 0。
  - `Invalid.HighIndexDuplicate`: インデックス 2、動的な状態 0 / 1、S8 1 / 0。過剰の規則を直す前は `GM-E8001`（終了コード 1）だった。直した後は警告 `GM-W8004` だけで通る（コーディネータの報告。db の `tests/stages.rs` も合わせた）。
  - `Init.Balanced` 2 / 2、`Init.OverDetermined` 2 / 1 で `GM-E8001`（位置は `x = 2`、終了コード 1）、`Init.UnderDetermined` 0 / 2 で `GM-W8001`・`GM-W8002`（h2）、`Init.ParameterWithoutValue` 1 / 1 で `GM-W8003`（T）、`Init.FreeParameter` 2 / 2。
  - `Connect.RealCircuit` 0 / 1 で `GM-W8001`・`GM-W8002`（`capacitor.v`）、`Connect.BooleanSignals` 0 / 4 で柔らかい `GM-W8001` だけ。どちらも終了コード 0。
- MSL の S8: ChuaCircuit 3 / 3、Rotational.First（インデックス 3、動的な状態 4 / 8）4 / 4、PID_Controller（インデックス 3、動的な状態 6 / 10、離散 3）6 / 9 で柔らかい `GM-W8001` だけ。`DemonstrateContinuousSignalExtrema` 21 / 21、`DemonstrateSignalExtrema` 11 / 11。
- MSL の全例題（`mise run msl:funnel`、S0〜S8）:
  - 最初の実行（過剰の規則を直す前）: OK 366・NG 16（S5 1・S6 12・S8 3）・GM-U 184・panic 0。S7 を回した 369 本（インデックス 1 が 215・2 が 58・3 が 95・4 以上が 1（`Blocks.Examples.FilterWithDifferentiation`、微分 14 本・最大 3 回）・打ち切り 0、状態選択が要る 154）。S8 の 369 本（合う 111・過剰 3・不足 255・入力なし 0、`GM-W8003` あり 2（`Fluid.Examples.InverseParameterization`、`Magnetic.QuasiStatic.FundamentalWave.Examples.ExampleUtilities.SpacePhasorLimiter`））。Rumoca との照合 381 / 381。検査 881.8 s（1 本 1.56 s）、最大 RSS 7.1 GiB。S8 の NG 3 本は MSL の正しい例題（`Electrical.Analog.Examples.GenerationOfFMUs` 8 / 6、`Electrical.Analog.Examples.Lines.CompareLineTrunks` 60 / 57、`Thermal.HeatTransfer.Examples.GenerationOfFMUs` 4 / 3）で、過剰の規則を直した（「決めたこと」）。
  - 最終の実行（規則を直した後）: 566 本: OK 369（うち S7 の情報 154）・NG 13（S5 1・S6 12・S7 0・S8 0）・GM-U 184（82 / 52 / 50）・panic 0。OK・NG の本数は S005 と同じ。S7 を回した 369 本の構造インデックスは 1 が 215・2 が 58・3 が 95・4 以上が 1・打ち切り 0、状態選択が要るもの 154。S8 の 369 本は合う 111・過剰の見込み 3（`GM-W8004`。Error 0）・不足 255・入力なし 0、`GM-W8003` あり 2。S6 と Rumoca の照合 381 / 381。検査 833.0 s（1 本 1.47 s）、最大 RSS 7.2 GiB。
- `check_docs.py`（締めで回し直した）: デッキ 7 件 OK（`_template` を含む）/ 文書 60 件・相対リンク 251 件中の切れ 0 件。
- Playwright（`render_slides.py`、全デッキ）: デッキ 7 件 / 問題 0 件。S006 は 15 枚 × 3 表示。スクリーンショットを目で見て、S8 の図の矢印が箱を横切っていたのを直し、題名や表の見出しの折り返しを詰めた。ファネルの数字を入れた後に S006 だけ描き直して問題 0 件（S05・S10・S13 を目で確かめた）。
- セッション記録の `--check`: 検査 3 件 / 取りこぼしのあるファイル 0 件（S005 と同じセッション ID なので、同じファイルを S006 の時点の写しで置き換えた）。
- ステージした差分の漏洩検査: 0 件（49 ファイル、+12314 / −254 行）。

## 残したこと

- **omc は未導入**。入れたら `msl:funnel` の omc 照合を再開する: S5 の一致率（P2 の完了条件「95% 以上」）、S6 で NG の MSL 12 本の確認、GM-U0003 の一覧の確かめ（ADR-0007 の C）。
- 閉包の違うモデルへの切り替えが約 1.7 s（Rumoca の再 resolve）。和らげる案（`Checker` に文書の和集合を持たせる、など）は S007 の候補。
- S8 は数と属性だけの近似。初期方程式の incidence（どの初期方程式がどの変数を決めるか）は見ていない。数が合っていても、決まらない変数や重複した初期方程式は見つけられない。
- 離散変数が when の外の式で決まるか、when の中で決まるかを区別していない（`GM-W8004` と柔らかい `GM-W8001` で逃がしている）。
- インデックスの低減で状態が減るときの余った fixed（整合する冗長な初期条件か）は、数値でしか決まらないので `GM-W8004` にとどめている。初期化系のマッチングで精密にする余地がある。
- S7 の結果を Rumoca 自身のインデックス低減（`rumoca-phase-structural`）と照合していない。
- `ClassOutline.imports` は保持するだけ（`deps_closure` は識別子で import の名前も拾う）。`class_index` の残り（for や if の中の connect、継承したクラスの中の入れ子のクラス）は未着手。
- `Checker` のメモリの上限（P4 のプロセス構成が決まってから）、Rumoca の制約の上流報告（ユーザーが出す）。
- `msl-funnel` は CI に入れていない。
