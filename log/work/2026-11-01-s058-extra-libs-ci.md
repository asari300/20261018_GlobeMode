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

## 検証

- `cargo test -p xtask` 46 本 OK（`libs` の試験 3 本・`parse_shard` を含む）、`cargo clippy -p xtask --all-targets -- -D warnings` OK、`cargo fmt -p xtask`。`mise run check` は親が回す。
- 手元（RAM 3 GB）: DeltaRobot の `fetch-library` → `msl-funnel`（21 本: OK 9・NG 7・GM-U 5・panic 0、最大 RSS 1.4 GiB）→ `msl-sim`（9 本実行、5 本が終わりまで実行できた）。Chemical は先頭 4 本（OK 0）。`--shard` と複数ルートのまとめ読みも確かめた。
- Actions（feature ブランチの push で起動。新しいワークフローは main に入るまで `gh workflow run` が 404 になるため、一時の `push` トリガーを足して試し、後で消した）:
  - **小さい 2 つ** run 41248143483（約 16 分、全ジョブ success、`libraries-report` も success）。DeltaRobot: 例題 21・OK 9・NG 7・GM-U 5・panic 0・実行 9 のうち 5 本実行できた（手元と一致）。Chemical: 例題 76・OK 12・NG 21・GM-U 43・panic 0・実行 12 のうち 0 本。表は [doc/libraries.md](../../doc/libraries.md)。
  - **全部の run** 41249259332（Buildings 6・AixLib 3・IBPSA 2・ClaRa 2・ThermofluidStream 1 の 14 ジョブ）は**起動しただけで待っていない**。結果は後で親が取り込む（`gh run download 41249259332 -n libraries` で表、`-n library-<名前>-<i>` で分割ごとの報告）。
- `check_docs.py`: 全デッキ OK / リンク 1237 件中の切れ 0 件。`render_slides.py`・セッション記録のコピー・main へのマージは親が行う。

## 残したこと

- 全部の run の結果の取り込みと `doc/libraries.md` の更新、失敗の原因の分類。
- 大きいライブラリの 1 ジョブの時間が 6 時間に収まるかは run の結果で確かめる。funnel と sim のステップは 170 分で切れ、funnel が間に合わないとその分割の JSON が無い（表の「分割」列で分かる）。足りなければ分割を増やす（`libraries.yml` の plan の表）。
- ClaRa の依存 SMArtInt、AixLib の依存 SDF・Modelica_DeviceDrivers は取得していない。TILMedia のライセンスは【未確認】。
- `Checker::new` がライブラリの構文の誤り 1 つで全体を断る（大きいライブラリで起きると、その分割は全部 `GM-E1003`）。frontend は並行セッションが編集中なので触っていない。
- `doc/sessions/README.md` の S058 の行は末尾に足した（並びは親が直す）。状態は「作業中」。
