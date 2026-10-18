within GlobeModeValidation.Index;
model ParallelCapacitors "2 つのコンデンサを理想的に並列にした回路（インデックス 2。S7 の Pantelides 法の検証）"
  // 電流源 I から、2 つのコンデンサ C1, C2 を抵抗なしで並列につないだ回路。部品を使わずに式で書いたもの。
  // 状態は v1, v2 の 2 つ、代数変数は i1, i2, i の 3 つ。
  //
  // v1 = v2 は状態だけの拘束なので、der の形ではどの未知数とも対応づかない。Pantelides 法は
  // これを 1 回微分して（der(v1) = der(v2)）、構造インデックス 2 と見積もる（GM-I7001）。
  // 微分した拘束に der(v1) と der(v2) が現れるので、v1 と v2 が状態選択の候補になる（GM-I7002）。
  //
  // 初期条件（S8）: v1 = v2 があるので、自由に決められる電圧は 1 つ（動的な状態の見込みは 1）。
  // fixed は v1 だけにし、v2 は拘束から決める。両方を fixed にすると、v1 = v2 と合わせて初期条件が多すぎる。
  parameter Real C1 = 1e-3 "コンデンサ 1 の静電容量";
  parameter Real C2 = 2e-3 "コンデンサ 2 の静電容量";
  parameter Real I = 1 "電流源の電流";
  Real v1(start = 0, fixed = true) "コンデンサ 1 の電圧";
  Real v2(start = 0) "コンデンサ 2 の電圧（v1 = v2 で決まる）";
  Real i1 "コンデンサ 1 に流れる電流";
  Real i2 "コンデンサ 2 に流れる電流";
  Real i "電流源の電流";
equation
  C1 * der(v1) = i1;
  C2 * der(v2) = i2;
  v1 = v2;
  i1 + i2 = i;
  i = I;
  annotation (experiment(StopTime = 1));
end ParallelCapacitors;
