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
- **読めないファイルを除く**: `Checker::new` は公開ライブラリの 1 つの誤りで全体を断る（ThermofluidStream の `Contact.mo` の構文の誤り、`BaseClasses` の `package.mo` 欠け、ClaRa の `within` の大文字小文字（PKG-010）と 1483 ファイルの BOM）。BOM は `fetch-library` が外し、ほかは 1 ファイルずつ読んで確かめて、まとめたルートから除く。診断（PKG-0xx）に出たファイルも除いて読み直す（上限 20 回）。
- **大きいライブラリは分割**: Buildings 4・AixLib 2・ほかは 1（最初は 6・3・2・2 にしたが、最初の run で Buildings の 1 分割（135 本）の funnel が 143 s だったので減らした）。funnel が JSON を最後にまとめて書くので、分割の単位を小さくして、間に合わなかった分割だけが欠けるようにした。各分割は MSL を読み直す（手元 22〜127 s）。
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
  - **全部の run** 41250636800（Buildings 4・AixLib 2・IBPSA・ClaRa・ThermofluidStream 各 1 の 9 ジョブ。最初の全部の run 41249259332 は、ClaRa（BOM）・ThermofluidStream（構文の誤りと PKG-006）・AixLib で Rumoca の読み込みが全体を断ったので取り消して、除いて読む仕組みを足してから回し直した）は**起動しただけで待っていない**。結果は後で親が取り込む（`gh run download 41250636800 -n libraries` で表、`-n library-<名前>-<i>` で分割ごとの報告）。
- `check_docs.py`: 全デッキ OK / リンク 1237 件中の切れ 0 件。`render_slides.py`・セッション記録のコピー・main へのマージは親が行う。

## 残したこと

- 全部の run の結果の取り込みと `doc/libraries.md` の更新、失敗の原因の分類。
- 大きいライブラリの 1 ジョブの時間が 6 時間に収まるかは run の結果で確かめる。funnel と sim のステップは 170 分で切れ、funnel が間に合わないとその分割の JSON が無い（表の「分割」列で分かる）。足りなければ分割を増やす（`libraries.yml` の plan の表）。
- ClaRa の依存 SMArtInt、AixLib の依存 SDF・Modelica_DeviceDrivers は取得していない。TILMedia のライセンスは【未確認】。
- `Checker::new` がライブラリの構文の誤り・配置の誤り（PKG-006・PKG-010）・BOM 1 つで全体を断る。xtask が除いて読む迂回をしたが、frontend は並行セッションが編集中なので触っていない（IDE で開くときは同じ問題が起きる）。Buildings・AixLib・IBPSA で除くファイルが多いと、20 回の読み直しの上限に当たりうる【未確認】。
- `doc/sessions/README.md` の S058 の行は末尾に足した（並びは親が直す）。状態は「作業中」。

## 追記（親、2026-11-01）

- 全部の run（41250636800、全ジョブ success。取り込みは親）: Buildings 例題 809・OK 119・GM-U 193・panic 4・実行できた 88/119（検査 1:13）、AixLib 642・119・163・0・98/119、IBPSA 457・88・135・4・66/88、ThermofluidStream 122・4・115・0・2/4、ClaRa 5・0（S1 で全部 NG）。7 ライブラリ合わせて例題 2,132 本、OK 351 本、実行できた 264 本。 `doc/libraries.md` に 2 つの run の表を合わせた。
