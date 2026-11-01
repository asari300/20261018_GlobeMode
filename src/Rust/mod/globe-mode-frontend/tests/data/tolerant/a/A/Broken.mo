// わざと構文を誤ったファイル（Checker::new が断らずに読み飛ばす試験）
within A;
model Broken
  Real x
equation
  x = 1;
end Broken;
