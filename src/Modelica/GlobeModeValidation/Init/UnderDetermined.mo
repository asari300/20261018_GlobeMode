within GlobeModeValidation.Init;
model UnderDetermined "初期条件を 1 つも与えていない（S8 の警告 GM-W8001 と GM-W8002。検査は通る）"
  // 2 つのタンクの水位 h1、h2 を部品を使わずに式で書いたもの。状態は 2 つ。
  // fixed も initial equation も無いので、要る初期条件 2 に対して与えられた初期条件 0。S8 の GM-W8001（Warning）が 1 件。
  // h1 には start = 1 があるので推定値に使えるが、h2 には start が無い。h2 の宣言に S8 の GM-W8002（Warning）が 1 件。
  // 警告なので検査は通る（不足分は start 値、書いていなければ既定の 0 を推定値として使う見込み）。
  parameter Real A = 1 "断面積";
  parameter Real q = 0.2 "上のタンクへの流入量";
  parameter Real c = 0.5 "流出の係数";
  Real h1(start = 1) "上のタンクの水位";
  Real h2 "下のタンクの水位";
equation
  A * der(h1) = q - c * h1;
  A * der(h2) = c * h1 - c * h2;
  annotation (experiment(StopTime = 10));
end UnderDetermined;
