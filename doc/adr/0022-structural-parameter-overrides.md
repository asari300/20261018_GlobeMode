# ADR-0022 パラメータの上書きは、上書きを組み込んでコンパイルし直す

- 状態: Accepted（2026-10-28、S024）
- 関係: [ADR-0013](0013-exec-backend.md)（実行系）、[ADR-0014](0014-session-route-wasm.md)（session の道と wasm）

## 背景

- `globe simulate --param NAME=VALUE`（wasm の `settings.parameters` も同じ）は、これまで実行の段だけで上書きしていた（`globe-mode-exec` の `param_overrides` を Rumoca の `SimOptions` に渡す）。コンパイルは既定値のまま。
- S023 で、`StepVoltage` の `startTime` を上書きしても時間イベントの時刻が動かないことを見つけた。S024 で調べると、時間イベントに限らない。Rumoca の instantiate は、同じ名前の修飾（`SineVoltage V(f = 1)` の中の `signalSource(final f = f)` など）で外側の値をそのまま内側の束縛に書き込むので、実行時に `V.f` を変えても内側の `signalSource.f` は 1 のまま。**`--param V.f=2` が黙って無視され、誤った結果を返す**（`SineVoltage` で V.v(0.05) が 0.309、正しくは 0.588）。
- DAE の時点で外側と内側の依存は消えているので、DAE を見て「効かない上書き」を見分けることはできない（警告だけの案は、名前の形の推測になる）。
- Rumoca には、上書きを根の修飾として組み込んでコンパイルする `Session::set_structural_overrides` がある（rumoca-compile の `session/structural_overrides.rs`）。ソースを書き換えたのと同じ結果になる。ただし、同じ session で一度コンパイルした後に呼ぶと ED020（来歴が未知のソースを指す）で落ちる。ライブラリだけを読んだ session（`Checker` の `pristine`）を複製してから使えば落ちない。
- 費用（WSL、release）: 複製 0.24〜0.33 s、上書きを組み込んだコンパイル 1.6〜3.0 s（`CauerLowPassSC` の既定のコンパイルは 2.2〜2.3 s）。

## 決定

**上書きが 1 つでもあれば、ライブラリだけの session を複製し、利用者のファイルを足して `set_structural_overrides` を呼んでからコンパイルする。上書きが無ければ今まで通り。**

- 実行の段の `param_overrides` もそのまま渡す。名前の誤りや調整できないパラメータの拒否（GM-E10006）と、結果に残す `SimRun.parameters` をそのまま使うため。組み込んだ後なので、実行時の値はコンパイルの既定値と同じになる。
- Rumoca の上書きの型（`Real`・`Int`・`Bool`）は、既定のコンパイルの変数の型から選ぶ（`Real` を Integer のパラメータに渡すと型の誤りで落ちる）。
- 上書きを組み込んだコンパイルが落ちたら（`final` のパラメータなど）、その診断をコンパイルの失敗として返す。既定のコンパイルと実行時の上書きに黙って戻さない。
- 検査の段（S0〜S9）は既定値のモデルで行う。上書きの値で変わるのは、プリフライト（S10）と実行。

## 結果

- 上書きのある実行は、コンパイル 1 回分（MSL の回路で約 2 s）遅くなる。上書きの無い実行は変わらない。
- 上書きの意味が「ソースの修飾を書き換えたのと同じ」になる。同じ名前の修飾で写された値、時間イベントの時刻、それらに依るパラメータまで届く。
- 上流の候補 16（時間イベントの時刻の畳み込み）は根の原因ではなかった。根は同じ名前の修飾の書き込み（候補 18）で、複製しないと落ちる ED020 も候補 17 として残す。詳しくは [research/msl-sim-failures.md](../research/msl-sim-failures.md) の「S024 で直したもの」。
- 覆す条件: Rumoca が実行時の上書きを内側の束縛と時間イベントまで届けるようになれば、再コンパイルをやめて実行の段だけに戻す。
