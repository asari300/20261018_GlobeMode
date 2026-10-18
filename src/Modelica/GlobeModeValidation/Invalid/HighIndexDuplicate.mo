within GlobeModeValidation.Invalid;
model HighIndexDuplicate "電圧則を 2 回書き、オームの法則を書き忘れた RC 回路（S6 は通り、S7 の情報 GM-I7001。数値的な特異は後の段階で見つかる）"
  // RC 回路を部品を使わずに式で書いたもの。
  // キルヒホッフの電圧則を 2 回（別の形で）書き、オームの法則 vR = R * i を書き忘れている。
  // 方程式 3 本 = 未知数 3 個（vR、i、der(vC)）なので S5 は通る。
  // 状態 vC と der(vC) を 1 つの未知数にまとめると完全マッチングがある（vR = V - vC で der(vC) を決める見込み）ので、
  // S6（構造特異）も通る。vC を既知とみなす der の形では vR だけを含む方程式が 2 本あるので、
  // 微分インデックスが 1 より大きい見込みとして S7 の情報（GM-I7001）が出る。
  // 2 本の電圧則は同じ式なので、実際には数値的に特異。これはインデックスの低減（S7）以降で初めて分かる。
  parameter Real R = 100 "抵抗";
  parameter Real C = 1e-3 "静電容量";
  parameter Real V = 10 "電源の電圧";
  Real vR "抵抗の電圧";
  Real vC(start = 0, fixed = true) "コンデンサの電圧";
  Real i "回路を流れる電流";
equation
  C * der(vC) = i;
  V = vR + vC;
  vR = V - vC;
  annotation (experiment(StopTime = 1));
end HighIndexDuplicate;
