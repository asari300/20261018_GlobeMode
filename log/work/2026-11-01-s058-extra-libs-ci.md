# S058 `extra-libs-ci`

| | |
| --- | --- |
| 日付 | 2026-11-01 |
| ブランチ | `feature/s058-extra-libs-ci` |
| worktree | `.git-worktree/s058-extra-libs-ci` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S058-extra-libs-ci/` |
| セッション記録 | 親のセッションが写す（このサブエージェントは写さない） |

## 依頼

MSL で回している検査（`msl-funnel` の S0〜S10 と `msl-sim` の実行）を、Buildings・AixLib・IBPSA・ClaRa・ThermofluidStream・DeltaRobot・Chemical にも GitHub Actions で回す。結果を待たずに進め、本筋の CI（ci.yml の push）を優先する。実行に失敗しても全モデルを検査する。

## 決めたこと

- **取得は commit 固定のアーカイブ**（`archive/<commit>.zip`、sha256 で確かめる）。release の zip があるのは Buildings だけで、IBPSA の 4.0.0 と ClaRa は tag が無いか古い。書き方をそろえるため全部 commit で固定した（[research/libraries-2026-11-01.md](../../doc/research/libraries-2026-11-01.md)）。
- **置き方は MSL と同じ**（`data/libs/<名前>/<パッケージ>/package.mo`）。ClaRa と TILMedia はリポジトリの根がパッケージなので、表の `subdir` が空のとき根をそのまま `<パッケージ>/` に置く。DeltaRobot は `Modelica_models/DeltaRobot`。
- **xtask の引数**: `--library <DIR>`（何度でも。MSL のほかに読む）、`--package <根>`（既定 `Modelica`）、`--examples-in <段>`（既定 `Examples`。DeltaRobot は `Scenarios`・`Tests`）、`--shard i/n`（msl-funnel のみ）。MSL は従来どおり `--msl`（`data/msl`）で、`--library` に MSL は渡さない（指示の「MSL も渡す」とは違う。`--msl` を省くと MSL の例題の規則と報告が使えなくなるため）。引数なしの出力は変えていない（`cargo test -p xtask` 46 本）。
- **複数のルートは 1 つのディレクトリにまとめて読む**: `Checker::new(&[msl, lib])` では、ライブラリの中の `import Modelica.Units.SI` が解決できず全例題が `GM-E1001`（unresolved import）になった（2 つの順序とも、Chemical と DeltaRobot で再現）。MSL とライブラリの直下のエントリへの symlink を並べた `target/library-roots/<名前>/` を作って 1 つのルートとして読むと解決した。`globe-mode-frontend` は並行セッションが触るので直さず、xtask 側で迂回した。原因（Rumoca の Session が別々のルートの間で名前を引けない）の確認は【未確認】。
- **大きいライブラリは分割**: Buildings 6・AixLib 3・IBPSA 2・ClaRa 2・ほかは 1。funnel が JSON を最後にまとめて書くので、分割の単位を小さくして、間に合わなかった分割だけが欠けるようにした。各分割は MSL を読み直す（手元 22〜127 s）。
- **ci.yml を優先**: 各ジョブの最初に `gh run list --workflow ci.yml --status in_progress|queued` が空になるまで 60 s 待つ（上限 30 分）。`concurrency` の group は `libraries` で、ci.yml の group（`ci-<ref>`）とは別。毎週は日曜 21:00 UTC（月曜 06:00 JST）、ci.yml の 03:00 JST の後。
- **失敗扱いにしない**: funnel と sim のステップは `continue-on-error: true`、`fail-fast: false`、ジョブ 360 分（funnel と sim のステップに 170 分ずつ）。artifact は `if: always()`。
- 参照結果（MAP-LIB）は MSL のものだけなので sim は参照なしで走る。`msl-sim` はそのまま動いた（`注意: data/msl-ref に参照結果が無い` を出すだけで、`RefCheck::NotFetched` を数える）。「参照なしのモード」は足していない。

## やったこと

- `src/Rust/xtask/src/libs.rs`（新規）: ライブラリの表、`fetch-library`、`libraries-report`、試験。
- `src/Rust/xtask/src/funnel.rs`・`sim.rs`・`main.rs`: 上の引数、`make_checker` のまとめ読み、報告の見出し、JSON の `package`・`libraries`、`#![recursion_limit = "256"]`（`json!` が 128 を超えた）。
- `.github/workflows/libraries.yml`（新規）、`.gitignore`（`data/libs/`）、`CLAUDE.md` §2、`doc/research/libraries-2026-11-01.md`、`doc/roadmap.md`、`doc/sessions/`。
