# 下書き（未提出）05: 実行時のパラメータ上書きが、同名の修飾で渡された束縛に届かない

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [05-parameter-override-not-reaching-inlined-bindings.md](05-parameter-override-not-reaching-inlined-bindings.md)、再現は [.mo](05-parameter-override-not-reaching-inlined-bindings.mo)、1 枚の解説は [_jp.html](05-parameter-override-not-reaching-inlined-bindings_jp.html)。

判定（GlobeMode、S024。S162 で v0.10.4 で再確認）: **Rumoca のバグ**（誤りも警告も無く黙って効かない）。v0.10.4 でも再現する。

## 概要

`SimOptions::param_overrides`（調整できるパラメータの実行時の上書き）が、内側の部品が `signalSource(final f = f)` のような同名の修飾で受け取るパラメータには効かず、外側のパラメータだけが変わる。instantiate の段が外側の修飾の値を内側の束縛に書き込むので、DAE の上で内側が外側に繋がらない。上書きは警告無しに受け付けられ、結果は黙って誤る。時間事象の時刻は見える例の 1 つで、DAE の段が `time < startTime` を束縛の既定で畳む（`time_event_instant` / `constant_context()`）ので、静的な時間事象が既定の時刻のまま残る。

## 再現手順

CLI に実行時の上書きの指定は無い（`rumoca sim --help` に無い）。同じリリースの Python 束縛が `SimOptions::param_overrides` を使う（`Model.simulate(params=...)`）。

```python
import rumoca
m = rumoca.Session().load("05-parameter-override-not-reaching-inlined-bindings.mo", model="InlinedParameters")
r = m.simulate(1.0, params={"src.startTime": 0.5})
print(list(r["src.y"])[-1])   # t = 1 の src.y
```

MSL の形: 抵抗と接地の間の `SineVoltage V(V = 1, f = 1)` に `V.f = 2` を上書きし、`V.v(0.05)` を読む。

## 期待

`src.startTime = 0.5` で `y(1) = 0.5`（既定の 1 なら `y(1) = 0`）。MSL の形は `V.v(0.05) = sin(2π·2·0.05) = 0.588`。

## 実際

| Rumoca | `{}` | `{"src.startTime": 0.5}` | `{"src.startTime": 0.25}` |
| --- | ---: | ---: | ---: |
| v0.10.2（wheel） | 0.0 | 0.0 | 0.0 |
| v0.10.4（wheel、S162） | 0.0 | 0.0 | 0.0 |

誤りも警告も出ない。MSL の形は `V.v(0.05) = 0.309`（`f = 1` の値、GlobeMode S024）。ソースでは `crates/rumoca-phase-instantiate/src/mod_env.rs` の `resolve_single_part_ref_expr`（v0.10.4 では 787 行目）が v0.10.2・main と同一。

## 影響

MSL の `SineVoltage`・`StepVoltage` を使う例題【本数未確認】。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）。
- 2026-11-05（S162）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、2026-11-02 公開、2026-11-05 でも最新）の Python wheel（manylinux_2_28_x86_64）で再確認。main（`590ac739`）はビルドしていない【未確認】。
- MSL 4.1.0。Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

あり（S024、[ADR-0022](../../adr/0022-structural-parameter-overrides.md)）。上書きがあるときは、ライブラリだけの session を複製し、`Session::set_structural_overrides` で上書きを埋めて compile し直す。`CauerLowPassSC` で約 2 秒、0.4 GB 増える。この道の問題は下書き 06。

## 備考

- 時間事象の畳み込み（`constant_context()`）は同じ原因の現れの 1 つで、別のバグではない。
- 提出の順は「残り（02〜15）」の群。回避があるので急がない。
