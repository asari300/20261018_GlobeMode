# S204 `fmi-import-first-slice`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s204-fmi-import-first-slice` |
| worktree | `.git-worktree/s204-fmi-import-first-slice` |
| Phase | P5 |
| セッション文書 | `doc/sessions/S204-fmi-import-first-slice/` |
| セッション記録 | （親が写す） |

## 依頼

ADR-0026（FMI Import の置き方）について、ユーザーが 2026-11-06 に「A、B 両方選択」と決めた。親の解釈は「案 A（自前の C ABI）を既定にし、案 B（crate `fmi`）を任意の feature として並べる」。この解釈で ADR を Accepted にし、FMI 2.0・3.0 の ME か CS のどちらか 1 つを Reference-FMUs で通し、自分で Export した FMU を自分で Import する往復の試験を 1 本、案 B の骨組み、CLI の最小の口、Linux・Windows・macOS の workflow、を半日〜1 日の大きさで。

## 決めたこと

- **CS を選んだ**（ME ではなく）。Reference-FMUs が CS の参照の CSV を同梱しているので、「参照結果と突き合わせる」が全桁で成立する。ME は積分器の差が出るので次の WP。
- **置き場は新しい crate `globe-mode-fmi-import`**（`globe-mode-fmi` の feature にしない）。Export・XML の利用者と wasm32 の検査に影響させないため。
- **案 B は入れなかった**。試した事実（2026-11-06）: `fmi` 0.8.0 は zip 8.6.0 を引き（勧告 RUSTSEC-2026-0367・0368・0369・0429。FMU の展開に使う）、`bzip2-1.0.6` が deny.toml の許可に無く、Cargo.lock が 67 増え、FMI 3.0 は Reference-FMUs で `Instantiate` が NULL（resourcePath の末尾に区切りが無い）。2.0 は動き参照の CSV と全桁で一致した。`/add-dependency` の「問題があれば入れずに事実を ADR に書く」に従った。ユーザーの判断を仰ぐ点は ADR-0026 の (g)。A/B を同じ trait で並べる設計は残した（実験した 90 行ほどの実装は保存していない）。
- **`globe fmu run` は `--trust` を要求しない**。標準エラーに 1 行の警告を出す。S057 の「P5 で決める」の答え。UI は確認のダイアログ（次の WP）。
- **2.0 の logger**: 可変長引数の C 関数は安定版の Rust で定義できないので、名前のついた引数だけを受ける非可変長の関数を渡した（FMPy の ctypes と同じ）。
- 展開の失敗は黙って飛ばさない。`modelIdentifier` のパス区切りも読み込み前に拒む。

## やったこと

- `src/Rust/mod/globe-mode-fmi-import/`（新）: `lib.rs`（`Fmu`・`FmuCs`・`run_cs`・`read_model_description`・展開）、`abi2.rs`・`abi3.rs`（C ABI。`unsafe` はここだけ、各ブロックに `// SAFETY:`）、試験 `tests/reference.rs`・`tests/unpack.rs`。
- `globe-mode-cli`: `src/fmu.rs`（`info`・`run`）、`main.rs` は `mod fmu;`・`FmuCommand::Import(#[command(flatten)])`・match の 1 本・doc コメント 1 行だけ。`tests/fmu_import.rs`（往復を含む）。
- `.github/workflows/fmu-import.yml`（手動。Linux・Windows・macOS）。
- 文書: ADR-0026（Accepted、S204 の決定 (f)〜(k)）、ADR README、licenses.md、features.md、roadmap.md、architecture.md、CHANGELOG、セッションの索引、スライドと解説書。

## 検証

- Reference-FMUs（Linux x86_64）の BouncingBall・Dahlquist・VanDerPol を 2.0・3.0 で、参照の CSV と同じ刻みで回して全行・全列の相対差 0（BouncingBall 301 行、Dahlquist 101 行、VanDerPol 2001 行）。
- 往復（Export した Decay を `globe fmu run` で読む）: x(1) − e⁻¹ = 7.8e-16（2.0・3.0）。
- 展開の試験: `..` のエントリ、シンボリックリンク、`modelIdentifier` の `../`、この OS 向けのライブラリが無い、がどれもエラー。
- コードの検査・`check_docs.py`・CI・`fmu-import.yml` の run: 最後の節に書く。

## 残したこと

ADR-0026 の (k): ME、ワーカーのプロセスの中での読み込み、facade と UI の「FMU を開く」、マスター・SE、案 B（`fmi` が zip 9 以上に上がったら）。`fmu-import.yml` の Windows・macOS の結果は run のあとで追記する。
