# S213 `fmu-runtime-issues`

| | |
| --- | --- |
| 日付 | 2026-11-07 |
| ブランチ | `feature/s213-fmu-runtime-issues` |
| worktree | `.git-worktree/s213-fmu-runtime-issues` |
| Phase | P5 |
| セッション文書 | `doc/sessions/S213-fmu-runtime-issues/` |
| セッション記録 | [2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md](../sessions/2026-11-02-a2580a13-3889-4662-9db4-a45dc1ca6772.md) |

## 依頼

issue #40（ponytail-audit、2026-11-07）＝ Export した FMU の実行時の 4 点を、いまの main で 1 点ずつ再現してから直す。再現しなければ理由を書いて直さない。

## 決めたこと

- **#1（panic）は直さない（再現しない）**。Rust 2024 の `extern "C"` は unwind を abort にする（UB ではない）。panic する入力を探した: 範囲外の valueReference（`fmi2GetReal`・`fmi2GetInteger` は 0〜39、`fmi2SetReal` は `u32::MAX` まで、3.0 の `fmi3GetBoolean` は 0〜19）、`EnterInitializationMode` の前の `NewDiscreteStates`・`EnterEventMode`、`SetTime(NaN)` のあとの `GetEventIndicators`、`SetContinuousStates(NaN)`、状態 0 個、空のモデル、`1/0` の微分、NaN の start、代数ループ、`when`、`for` 内の NaN。すべて `fmi2Error` か NULL で返った（診断は標準エラーに出る）。MSL の全例題で panic は 0 本（`doc/msl-funnel.md`）。コンパイラ側は `globe-mode-exec` が lowering を `catch_unwind` で包んでいる。
  - 包むなら `each`・`states`・`instantiate`・`rebuild` の 4 か所で済み、約 15 行。panic する入力が見つかったら足す。
- **#2** は Reset ではなく「作り直し」が原因。FMI 2.0 の `fmi2Reset` は新しく作り直したのと同じ（既定に戻るのが正しい）。直したのは、`SetReal` で状態に書いた値（start 値）が、同じ初期化の間にパラメータ・開始時刻の変更で `rebuild` が走ると消えること。`inputs` と同じ表（`states`）を持ち、作り直した後に置き直す。
- **#3** は `globe-mode-fmi/src/export.rs` の `build`（`write_fmu` ではない）の 1 か所。`cc` crate は入っていないので `split_whitespace`。S212 が触る `write_fmu` には触れていない。
- **#4** は `read_resource` の URI → パスを `resource_path` に切り出した。`modelDescription.xml` のパスの書き込みは実行時には無い（`sources` の相対パスは zip の中の名前で、`export.rs` が `\` を `/` にしている）。

## やったこと

- `globe-mode-fmu/src/lib.rs`: `Instance.states`、`resource_path`（と単体試験）。
- `globe-mode-fmu/tests/fmpy.rs`: `state_start_values_survive_a_rebuild`。
- `globe-mode-fmi/src/export.rs`: `host_cc`（と単体試験）。

## 検証

- 直す前の失敗: `state_start_values_survive_a_rebuild`（`[1.0, 2.0]` ≠ `[5.0, 2.0]`）、`resource_path_from_uri`（`/C:/Users/u/r`）、`cc_with_arguments_is_split_on_whitespace`（`"ccache  gcc -m64"` が 1 つのプログラム名）。
- コードの検査（fmt / clippy / test / deny）: `mise run check` が通った（`cargo test -p globe-mode-fmu` は FMPy の 12 本を含めて 12 passed、`globe-mode-fmi` の export の単体 2 本）。負荷が高く（load 15〜26）FMPy の試験だけで約 25 分かかった
- `check_docs.py`: 文書 849 件 / 相対リンク 4136 件 / 切れ 0 件。`render_slides.py`: デッキ 1 件 / 問題 0 件
- セッション記録の `--check`: 取りこぼし 0 件（親が写した）
- features.md・roadmap.md は変えていない（計画は変わらず、挙動の修正のため）。
- CI: fmu-export・ci.yml full（記入）

## 残したこと

- #1: panic する入力が見つかったら、`each`・`states`・`instantiate`・`rebuild` を `catch_unwind` で包む。
- 付記の GUID（モデル名だけの `DefaultHasher`、`globe-mode-frontend/src/fmi_export.rs`）は数えていない。モデルを編集しても変わらない（P5a の FMU）。`globe-mode-fmu`（P5b）の `guid` は文面の FNV-1a で別物。
