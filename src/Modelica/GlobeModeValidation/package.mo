within;
package GlobeModeValidation "GlobeMode の検証用ライブラリ（connect グラフと検査のテスト）"
  annotation (
    uses(Modelica(version="4.1.0")),
    version="0.0.1",
    Documentation(info="<html>
<p>GlobeMode の検証用のモデル。connect の型（Real の potential / flow、Real と Boolean の信号、
Medium を持つ流体ポート）ごとに、connect グラフと検査を試す。</p>
<ul>
<li><code>Connectors</code>: テスト用のコネクタ</li>
<li><code>Components</code>: テスト用の部品</li>
<li><code>Connect</code>: connect の型ごとのテストモデル</li>
<li><code>Index</code>: 微分インデックスが 1 より大きい正しいモデル（S7 の検証）</li>
<li><code>Init</code>: 初期化系の検証モデル（S8）</li>
<li><code>Invalid</code>: わざと誤ったモデル（検査で止まることを確かめる）</li>
</ul>
<p>Medium を使うモデル（<code>Connect.MediumFlow</code>）は MSL 4.1.0 の Modelica.Media を使う。</p>
</html>"));
end GlobeModeValidation;
