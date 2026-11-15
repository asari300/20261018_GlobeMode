# 下書き（未提出）38: 初期化を確かめてから回すと、準備済みの BDF の部品を 3 回 lease（初期化）し、初期化済みの状態を実行に渡せない

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [38-bdf-initialized-state-not-reusable.md](38-bdf-initialized-state-not-reusable.md)、モデルは [.mo](38-bdf-initialized-state-not-reusable.mo)、1 枚の解説は [_jp.html](38-bdf-initialized-state-not-reusable_jp.html)。

判定（GlobeMode、S359）: **API の要望**（小さい）。結果は誤っていない。公開の BDF の道（`build_simulation_with_stage_timing_and_lowered_model`・`check_prepared_initialization`・`run_prepared_simulation`）は、保持している FMI の部品を 3 回 lease し、Rumoca は lease 1 回を「初期化」と書いている。費用は**測っていない**。v0.10.2・v0.10.4・main のソースを読んで比べた。S344（2026-11-15）の同じ読みが GlobeMode の issue #45 の出どころ。

## 概要

BDF の実行がどこで失敗したか（組み立て・初期化・積分）を知りたい呼び手は、公開の 3 つの段をこの順に使う。

```rust
let (prepared, _timings) = rumoca_sim::build_simulation_with_stage_timing_and_lowered_model(&dae, &opts, |_| {}, |_lowered| {})?;
rumoca_sim::check_prepared_initialization(&prepared)?;        // 「初期化が失敗したか」
let result = rumoca_sim::run_prepared_simulation(&prepared)?; // 実行
```

どの段も最後は `MeRetainedComponent::lease` に行く。その doc は「complete an actual initialization handoff or restore source defaults and initialize the next run」、`lease_count` の doc は「Runs this component has been leased for, each one an initialization」（`crates/rumoca-solver/src/fmi_me/session.rs`）。

| # | 場所 | していること（`crates/rumoca-sim/src/diffsol.rs`、v0.10.2 / v0.10.4 / main） |
| --- | --- | --- |
| 1 | `build_simulation_artifact` の最後 | `drop(check_prepared_component(&prepared))`（241 / 236 / 223 行）。lease、BDF のプラグインを付ける（`into_session`）、session を捨てる。結果は捨てる |
| 2 | `check_prepared_initialization` | もう一度 `check_prepared_component`（285 / 302 / 280 行）。lease、プラグインを付ける、session を捨てる |
| 3 | `run_prepared_simulation` | `simulate_prepared`（264 / 281 / 276 行）。lease、プラグインを付ける、`run_to_stop` |

1 つめは v0.10.4・main では warm-up と書かれている: 「A prepared simulation for repeated runs: instantiated, with every lazily compiled native owner compiled now by one warm-up initialization, so no run pays a compile. The warm-up's own outcome is dropped, because the run that meets a failing initialization reports it.」（v0.10.4 はこのコメントと、warm-up を飛ばす 1 回きりの道 `instantiate_prepared_simulation` も持つ。公開の 1 回きりの `simulate` は `build_simulation` を通るので warm-up が残る。）

呼び手は 2 つめを外すと段を失う。`lease`・`into_session`・`run_to_stop` のエラーは、`simulate_prepared` から `at_stage` を呼ばずに `?` で出る。そのため BDF のプラグインの始動の失敗（`MeIntegrationError::Numerical`）は、`run_prepared_simulation` の呼び手に `SimFailureStage` 無しで届く。FMI のカーネルの初期化の失敗には `MeStage::Initialization` が付く（`crates/rumoca-solver/src/fmi_me/kernel.rs:461,470,501`、v0.10.4）。`check_prepared_initialization` は `into_session` を単独で回すので、そこでの失敗は初期化から来たと分かる。GlobeMode はこれで初期化の失敗と分類し、その失敗では `auto` が別のソルバに切り替えない。

## 再現手順

繰り返しの初期化の費用は、モデルでは見えない。上の呼び出しの形が要点。3 つの呼び出しを試すための、初期化が自明でないモデルは [.mo](38-bdf-initialized-state-not-reusable.mo)（`Rep38.A`: 硬い状態 1 つ、initial equation、代数ループ）。`rumoca sim 38-bdf-initialized-state-not-reusable.mo -m Rep38.A --solver bdf` は v0.10.2・v0.10.4 で回る（501 点）。CLI では、部品が何回初期化されたかは見えない。【測っていない。コンパイルもしていない】

## 期待

3 つの段が 1 回の初期化を共有できる口。形は Rumoca の設計に合うほうで。例:

1. `check_prepared_initialization` が初期化済みの状態を返す（保持する）、`run_prepared_simulation` がもう一度 lease せずにそこから続ける。または
2. `run_prepared_simulation` が、プラグインの始動のエラーに `SimFailureStage`（たとえば `Initialization`）を付ける。呼び手は失敗の場所を知るために 2 つめの段を要らなくなる。または
3. build の関数に warm-up を飛ばす版を足す（v0.10.4 の 1 回きりの道に、非公開の `instantiate_prepared_simulation` が既にある）。

## 実際

lease が 3 回。2 回目と 3 回目が初期方程式をもう一度解くのか、保持した状態を戻すだけなのかは追っていない。`MeKernelCell::restart`（`crates/rumoca-solver/src/fmi_me/instance.rs:201`）は、保持した pristine な FMU の状態（`scalar_pristine`）があればそれを戻す【それが解く手間を省くかは未確認】。

## v0.10.4・main での再確認（S359、2026-11-16）

最新のリリースは v0.10.4 のまま（タグのコミット `9c8c56fcd9f5ab9beb871f09bf5d9eed82971c54`）。3 つの段は v0.10.4 でも main（`b55eafce2475be1721324d56e1c51ddecfe7dbdb`、2026-11-13。そこでは `check_prepared_component` が observation と `ConstructionPurpose::PreparationCheck` を受ける）でも同じ。v0.10.4 の変更は上のコメントと 1 回きりの道: 1 回きりの `simulate_artifact` は warm-up を飛ばす。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`、GlobeMode の固定）、v0.10.4（<https://github.com/CogniPilot/rumoca/releases/tag/v0.10.4>）、main（`b55eafce`）。ソースを読んで比べた。
- Linux x86_64（WSL2、Ubuntu）。この下書きのために何もビルドしていない。

## GlobeMode 側の回避

なし。GlobeMode は 3 つの段をそのまま使う。2 つめが「失敗が初期化から来た」ことを教えるため（GlobeMode の [ADR-0014](../../adr/0014-session-route-wasm.md)、S267 の付記）。プラグインの始動の、段の無い失敗を出すモデルは見つからなかった（小さなモデル 13 本と MSL の `GenerationOfFMUs` を、GlobeMode の 2 つの道で回して、診断は型か段が付いていた）ので、この規則は作った型のエラーでしか試していない。大きなモデルでの費用の大きさは分からない（issue #45 も「実測の数字は無い」）。

## 備考

- 37（rk45 に lowering 済みのモデルを渡す口が無い）の初期化版。どちらも、公開の段が繰り返す仕事の再利用を頼む。
- Rumoca 自身の `auto` は、BDF の probe が作った準備済みの部品をそのまま回す（`simulate_probed`、`pub(crate)`）ので、初期化の確認 1 回と実行 1 回で済む。公開の段にはその再利用が無い。
- 出どころ: GlobeMode の issue #45 とそのコメント（S226、S344）、[S344 の解説書](../../sessions/S344-solver-budget-issues/解説書.md)、[ADR-0014](../../adr/0014-session-route-wasm.md)（S267 の付記）。
