# 下書き（未提出）06: `set_structural_overrides` が compile 後に ED020、未知の名前を黙殺、スカラだけ

対象: Rumoca（<https://github.com/CogniPilot/rumoca>）。英語の本文は [06-structural-overrides-ed020-and-scalar-only.md](06-structural-overrides-ed020-and-scalar-only.md)、再現は [.mo](06-structural-overrides-ed020-and-scalar-only.mo)、1 枚の解説は [_jp.html](06-structural-overrides-ed020-and-scalar-only_jp.html)。

判定（GlobeMode、S025。S162 で v0.10.4 のソースを読んで確認）: **Rumoca の API の制限**（誤りでなく設計の穴）。コードは v0.10.2 と同じなので、残っていると読める。実行はしていない【未確認】。

## 概要

`Session::set_structural_overrides`（rumoca-compile）には 3 つの制限がある。下書き 05 の上書きを同名の束縛に届かせる道として使って見つけた。

1. **compile の後だと ED020。** すでに compile した session で呼ぶと、flatten と DAE のキャッシュしか消さない。合成した上書きのソースが解決済みの木の `source_map` に入らず、次の compile が `DAE provenance references an unknown source`（ED020）で落ちる。上流の単体試験は最初の compile の前に上書きを置くので通る。
2. **未知の名前を黙殺する。** パラメータ名の綴りを誤っても、誤りも警告も出ない。
3. **スカラだけ。** `StructuralOverride` は `Bool`・`Int`・`Real` のみで、`root_modifications` は session の非公開の欄、`from_dotted` は添字を対象の経路に解決しない。配列全体（`a = 5`）を上書きすると配列の束縛がスカラのリテラルになり ED020 で終わる。同名の修飾で渡される配列の要素（添付の `src.k[1] = 0.2`）は内側の束縛に届かない。

## 再現手順

すべてライブラリ API（CLI に無い）。v0.10.4 の Python 束縛も構造的な再インスタンス化（`Model.with_params(recompile=True)`）は「compile API の道で、まだ使えない」と答える。

1. 任意のモデルを session で compile してから、`set_structural_overrides` に任意の上書きを渡してもう一度 compile する。ED020（項目 1）。
2. 添付の `ArrayParameters` で `src.k[1] = 0.2` を上書きし、`src.s` の t = 0.5 を読む（項目 3）。

## 期待

compile の後でも上書きが効く（効かないならそう答える）。未知の名前は知らせる。配列と要素は、対応するか、はっきりした誤りで断る。

## 実際

項目 1 は ED020。項目 3 は GlobeMode S025（2026-10-28）で `src.k[1] = 0.2` が内側の `s` の切り替えを元の 0.8 のままにし、`src.k[2] = 7` が `y` を 20 のままにした。診断は出ない。

v0.10.4（S162、ソースを読んだだけ）:

- `crates/rumoca-compile/src/session/structural_overrides.rs` は v0.10.2・v0.10.4・main（`590ac739`）でバイト単位で同一。`session_snapshot.rs`・`session_impl_queries.rs` も同一。
- `session_impl.rs` は v0.10.4 で変わった（標準の解決を診断つきで持ち、strict compile で再利用する）。`set_structural_overrides` はその持ち越しを消さないので、項目 1 は残ると読める【未確認】。

## 環境

- Rumoca v0.10.2（rev `276e82bd0fd48960ac201e782918d28bf19cf7ce`）。
- 2026-11-05（S162）に最新のリリース Rumoca v0.10.4（`9c8c56fc`、2026-11-02 公開、2026-11-05 でも最新）のソースを読んだ。main（`590ac739`）はビルドしていない【未確認】。
- MSL 4.1.0。Linux x86_64（WSL2、Ubuntu）。

## GlobeMode 側の回避

あり。項目 1 はライブラリだけの session を複製し、その複製で最初の compile の前に上書きを置く。項目 2・3 は変換の前に GlobeMode が自分で断る（`GM-E10006`）。判定は名前の形だけ（[ADR-0022](../../adr/0022-structural-parameter-overrides.md)）。名前を付け替える修飾（`final kk = k`）は見逃す。

## 備考

- 3 つは 1 つの issue でも 3 つでもよい（上流に任せる）。
- 実行時の上書き（下書き 05）が直れば、この道を使う理由は薄れる。
