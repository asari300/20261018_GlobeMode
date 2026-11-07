# S213 `fmu-runtime-issues` のレビュー（ponytail-audit、issue #40）

| | |
| --- | --- |
| 日付 | 2026-11-07 |
| ブランチ | `feature/s213-fmu-runtime-issues` |
| 対象 | `src/Rust/mod/globe-mode-fmu/`（実行時）と `globe-mode-fmi/src/export.rs`（ビルド）。ponytail は読んだだけで、ビルドも実行もしていない |
| レビュアー | ponytail（Claude Code の `/ponytail-audit` 5.3.2、2026-11-07。ユーザーが issue #40 に起票） |
| 依頼の文面 | Export した FMU のランタイムの 4 点を、いまの main で 1 つずつ再現してから直す |
| 作業記録 | `log/work/2026-11-07-s213-fmu-runtime-issues.md` |

## 指摘（原文のまま）

issue #40 の「問題」の 4 項目（付記の GUID は数えない）。

| # | 重さ | 場所 | 指摘 | 採否 | 理由・直したコミット |
| --- | --- | --- | --- | --- | --- |
| 1 | 中 | `globe-mode-fmu/src/lib.rs:22`・`384-438`・`733-754` | `extern "C"` の入口（`instantiate`・`rebuild`・`fmi2NewDiscreteStates`）が `catch_unwind` なしでコンパイラを呼ぶ。`resources/` の `.mo` でコンパイラが panic すると FMPy・Dymola・Simulink がセッションごと落ちる（モジュールのコメントはそれを受け入れている） | 持ち越し（再現しない） | panic する入力が見つからない。ファイル末尾の理由を見る |
| 2 | 中 | `lib.rs:248-267`・`529-533` | `fmi2SetReal` は状態を直接書くが、`ExitInitializationMode` の `rebuild()` はパラメータの上書きと入力だけからインスタンスを作り直す。FMPy の `simulate_fmu(start_values={'x': 5, 'k': 2})` や `start_time != 0` で `x` が既定に戻り、エラーは出ない | 採用（再現した） | `Instance.states`（添字 → 値）を足し、作り直した後に置き直す |
| 3 | 低 | `globe-mode-fmi/src/export.rs:177` | `Command::new(CC)` は `CC="ccache cc"` や `CC="gcc -m64"` を 1 つのプログラム名と見て「cc を起動できない」 | 採用（再現した） | `host_cc`（空白で分け、最初が実行ファイル、残りが引数） |
| 4 | 低 | `lib.rs:273-284` | `read_resource` は Windows で `file:///C:/x` を `/C:/x` にする | 採用（再現した） | `resource_path`（ドライブ文字の前の `/` を落とす） |

## 結論

- 採用 3 件・持ち越し 1 件・不採用 0 件。
- 1 の「UB」は正確でない: Rust 2024 では `extern "C"` を越える unwind は未定義動作でなく abort する（モジュールの冒頭のコメントのとおり）。落ちるのは事実だが、panic する入力が見つからなかった（根拠は作業記録）。
- 2 は指摘のとおり。ただし Reset は FMI 2.0 の規格で「新しく作り直したのと同じ」なので、Reset のあとに開始値が既定に戻るのは正しい。直したのは「`SetReal` で与えた状態が、パラメータの変更などによる作り直しで消える」こと。
