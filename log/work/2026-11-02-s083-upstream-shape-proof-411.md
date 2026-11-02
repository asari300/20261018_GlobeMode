# S083 `upstream-shape-proof-411`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s083-upstream-shape-proof-411` |
| worktree | `.git-worktree/s083-upstream-shape-proof-411` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S083-upstream-shape-proof-411/` |
| セッション記録 | （親が写す） |

## 依頼

S080 で【未確認】のまま残った (1) MSL 4.1.0 で出る `function shape proof` の 12 本が 25b と同じ形か、(2) `splineDerivatives` の形、(3) `axis 1 is variable-size` の 5 本、(4) 時間があれば CLI の ER002 を 1 つ、を二分法で最小にし、下書き 25 への追記（`25c` などの枝番）か新しい下書きにする。文書だけ。main へのマージ・push は親が行う。

## 決めたこと

- 例題は run 41365960542 の artifact `library-*` の `*-funnel.json` から取った（`libraries` の artifact は表だけで JSON が無い）。`dependent extent` 12・`axis 1` 5・`splineDerivatives` 25。
- 失敗の行は Rumoca の CLI で出した（`globe check` はファイルしか指さない）。MSL の例題は `MODELICAPATH=data/msl rumoca compile "data/msl/Modelica 4.1.0/package.mo" -m <例題>`。ライブラリのパッケージ全体を CLI で読むのは IBPSA で 13 分を超えたので止め、ライブラリの例題は `globe check` のファイルの指す先と、ライブラリの写しの 1 行の修正で確かめた。
- 25 は形ごとに分けず、既存の下書きに 1 つの節（表）として足した。出すときに分けるかはユーザーが決める。
- ER002 は新しい番号 27。規定（MLS §7.1・§5.3.2）で継いだ要素を引けるので Rumoca の穴と判定。
- 依頼の既定のまま進めた: 番号の振り方、omc 1.27.2 の `checkModel` で受理を見ること。

## やったこと

二分法の経過（どこまで削ると消えるか）:

### 1. `dependent extent ...` の 12 本

| 例題 | 失敗の行（CLI） | 確かめ | 最小の形と消える条件 |
| --- | --- | --- | --- |
| TFS `SimpleLiquidWater` | `myMedia/Interfaces.mo`（globe） | 写しの 2323 行の `X[nX]` → `X[:]` で S0〜S10 OK | 25b と同じ |
| TFS `TestGlycol`・`InverseIncompressible_sh_T`（MSL の同名も） | MSL `Polynomials.mo` 137 行 `output Real p[n + 1]` | Glycol47 の関数を 1 つずつ呼ぶと `specificEntropy` だけがこのメッセージ（ほかは別の ED019 `function element assignment`、ED008） | 自己完結で: `if has then ... else`・`s_T`・`poly[1:npol]`・`y[size(u,1)]`・extends の修正・`final` を外しても出る。`Integer npol = size(poly, 1) - 1` の局所変数を外す、`Real` にする、束縛を数の配列にすると消える。局所変数の名前を定数と同じ（`npol`）にし、別名 `npolC = npol` を経ると同じメッセージ、違う名前だと `dependent extents cannot call runtime function`。→ 25c |
| IBPSA・AixLib・Buildings `OccupancySchedule` | `OccupancySchedule.mo`（globe） | IBPSA の写しで `input Integer nRow` を前に移すとこの ED019 は消える（`multi-output equation` で止まる） | 関数を外に出す・位置引数・数の引数でも出る。`n` を前に、`x[:]` で消える。extends でも出る。呼ぶモデルに parameter `n` があり、それを渡すと通った（理由は追っていない）。→ 25d |
| AixLib 膨張弁 2・圧縮機 2、TFS `R134a1`・`R134a2`（MSL の同名も） | MSL `Media/R134a.mo` 53 行 `parameter Real[nc] a` | MSL の写しで record の次元を数（`Real[5]`・`Real[21]`）にすると MSL の `R134a1`・`R134a2` はこの ED019 を抜ける（ED008 `state_h` で止まる）。AixLib の 4 本は globe が MSL の `R134a.mo` を指す | extends の修正が無くても出る。`constant` でも、修飾の無い `Integer` でも出る。数の次元、モデルの成分なら通る。→ 25e |

### 2. `splineDerivatives`（25 本）

自己完結の写し（`isMonotonic` を Boolean に置き換え）で再現。最初の assert の if を外しても出る。「Ensure monotonicity」の if と、出力を for で定める中の if/elseif/else の両方が要る（どちらかを外すと消える）。中の if/elseif は外しても出る。最小は「for で `d[i] := y[i]`、続けて `if b then for ... d[i] := 0; end for; end if;`」。最初を `d := y` にする、枝を代入にすると消える。→ 25g

### 3. `axis 1 is variable-size`（5 本）

- IBPSA・AixLib・Buildings `SteamTemperatureEnthalpyInversion`: `Steam.mo` の `temperature_ph` の `constant Real b[:]` が後で宣言する `TMean`・`TSD` を読む。`a` を外しても（`{c, 2}`）出る。`c` を前に、数の束縛で消える。`b[2]` にすると別の ED020（`missing function value definition`）。→ 25f
- TFS `MoistAir`・`MoistAir2`（MSL の `ReferenceAir.MoistAir`・`MoistAir2` も）: MSL `ReferenceMoistAir.mo` 213 行 `protected MassFraction[:] X;` を `X := massFractionSaturation(state)` で決める。束縛 `= g(x)` でも出る。`Real[2] X` で消える。→ 25h

### 4. ER002

S080 で見えた形は、`package Q extends P; end Q;` の `Q.B`（P の入れ子のモデル）を成分の型にするだけで出る。置き換えできない別名、根からの名前、外の package、Q に定数・修正を足しても出る。`P.B`、`replaceable package A = Q` なら通る。継いだ関数 `Q.f(time)` は通る。→ 27

足したファイル:

- `doc/research/upstream/25c`〜`25h`（`.mo` 6 つ）、`27-er002-inherited-class-through-extending-package.md`・`.mo`
- `doc/research/upstream/25-ed019-function-conditional-boolean-loop.md` に節「The remaining ED019 messages in MSL 4.1.0 runs (S083)」、`README.md` に 27 の行と S083 の節
- `doc/research/libraries-2026-11-01.md` に「ED019 の残り（S083）」
- `doc/sessions/S083-upstream-shape-proof-411/`、索引の 1 行

## 検証

- コードの検査: Rust のコードは触っていないので回していない。`globe` は `cargo build -p globe-mode-cli --release`（target-shared、3 分 18 秒）
- 再現: `globe check` と Rumoca v0.10.2 の CLI で 25 本のコードが一致（失敗 12・対照 13）。omc 1.27.2 の `checkModel` は 25 本とも completed successfully
- `check_docs.py`: デッキ全件 OK（S083 を含む）/ 文書 353 件・相対リンク 1,713 件中の切れ 0 件
- セッション記録の `--check`: 親が写す
- ステージした差分の漏洩検査（メール・ホームのパス・トークン）: 0 件

## 残したこと

- 25 の形を Rumoca のソースのどこで断るかは追っていない（25c の名前の捕捉、25d の呼ぶ側の `n` で通る理由も【未確認】）
- ER002 で止まるライブラリの例題の本数は数えていない。CLI の EF024・ET004 は見ていない
- `render_slides.py` は 1 回試したが 110 秒の上限で出力が無く終わった（Chromium の時間切れと見て CI に任せた）
