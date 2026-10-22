# ADR（設計判断の記録）

| 番号 | 題名 | 状態 |
| --- | --- | --- |
| [0001](0001-license-policy.md) | ライセンス方針 | Accepted |
| [0002](0002-rumoca-adapter.md) | Rumoca をアダプタ経由の git 依存で使う | Accepted |
| [0003](0003-toolchain.md) | ツールチェーン（mise・uv・cargo） | Accepted |
| [0004](0004-ui-tauri-svelte.md) | UI は Tauri 2 + Svelte 5 | Accepted |
| [0005](0005-lazy-query-db.md) | salsa による遅延評価とシミュレーション前のアサーション | Accepted |
| [0006](0006-incremental-ci.md) | CI は差分だけを検査し、大きな変更のときだけ全体を検査する | Accepted |
| [0007](0007-unsupported-classification.md) | 「コンパイラが未対応」（GM-U）を、コードの規則と既知の制約の一覧で振り分ける | Accepted |
| [0008](0008-local-llm-reviewer.md) | ローカル LLM は副レビュアーとし、スキルから OpenAI 互換 API で呼ぶ | Accepted |
| [0009](0009-structural-index-and-init.md) | S7 は Pantelides 法で構造インデックスを見積もり、S8 は初期条件の数と属性で判定する | Accepted |
| [0010](0010-omc-comparison.md) | omc との照合は別プロセスで行い、checkModel の数と translateModel の文言で突き合わせる | Accepted |
| [0011](0011-init-system-matching.md) | S8 は初期化系（DAE とその微分 + 初期条件）をマッチングして、冗長な初期条件と初期条件の無い未知数を名指しする | Accepted |
| [0012](0012-loops-and-preflight.md) | S9 は BLT と貪欲な tearing で非線形の代数ループの反復変数を見る。S10 は静的な部分（パラメータと start 値の評価、experiment annotation）から始める | Accepted |
| [0013](0013-exec-backend.md) | P3 の実行系は、Rumoca の実行系（rumoca-sim）を globe-mode-exec に閉じ込めて使う。結果は中立の型で返し、MAP-LIB の参照結果と矩形の管で比べる | Accepted |
| [0014](0014-session-route-wasm.md) | wasm32 と自前の積分器は、Rumoca の FMI ME 風の host に数値積分のプラグインを差す session の道で回す。ネイティブの bdf・dopri45・auto は型付きの道のまま（wasm-bindgen、auto の切り替え、パラメータの上書き、MAT v4） | Accepted |
| [0015](0015-auto-and-reference-tolerance.md) | `auto` は方向微分が作れないときに切り替えない。MSL の突き合わせは参照結果と同じ許容誤差（注釈の 0.1 倍、`--tolerance-scale`）で回す（ADR-0014 (d) の一部を覆す） | Accepted |
| [0016](0016-preflight-on-build-lowering.md) | プリフライトは組み立ての lowering の結果で行い（lowering は 1 回）、実行系の中の段（プリフライト・lowering・組み立て・初期化・積分）を `run_with_progress` で知らせる | Accepted |
| [0017](0017-lowering-limit-and-hard-timeout-retry.md) | MSL の実行（`msl-sim`）では lowering の段に別の上限（150 s）を掛け、外から止めたもの（その上限で止めたものを除く）も時間切れと同じく回し直す。親は段ごとの秒を記録する | Accepted |
| [0018](0018-no-preflight-limit.md) | プリフライトに上限を掛けない。DrumBoiler で詰まるのはプリフライトの評価ではなく、組み立ての段も通る Rumoca の実行時オブジェクトの組み立て（根の条件の構造解析）なので、上流に回す（ADR-0017 の結果の見込みを取り下げる） | Accepted |
| [0019](0019-rpi-aarch64-environment.md) | Raspberry Pi（aarch64）を開発と MSL の全体実行の機械にする。`fetch-rumoca` は `target_arch` で配布物を選び、メモリは swap 8 GB で補う。elixir（erlang の glibc）は入れず、CI は x86_64 のまま | Accepted |
| [0020](0020-init-resettle-check.md) | プリフライトで初期化系を 2 回解き、2 回目で値が動いたら警告 GM-W10004 を出す（Rumoca は初期化方程式が読む関係式の反転で解き直さず、残差の評価を公開していない）。実行は変えない | Accepted |

書式: `NNNN-<題名>.md`。章は「状態 / 背景 / 決定 / 結果」の4つ。決定を覆すときは古い ADR を消さず、状態を Superseded にして新しい番号で書く。
