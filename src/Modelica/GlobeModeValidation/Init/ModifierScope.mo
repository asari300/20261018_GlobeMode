within GlobeModeValidation.Init;
model ModifierScope "外からの修正 part(theta1(fixed = true)) が、部品の中の同名の joint.theta1 にも漏れる（Rumoca v0.10.2 の制約の記録）"
  // MSL の Mechanics.MultiBody.Parts.RollingWheelSet の形。Part は自分の theta1 を fixed = true にし、
  // 中の Joint の theta1 は joint(theta1(fixed = false)) で fixed = false にして、等式 joint.theta1 = theta1 で結ぶ。
  // 例題（RollingWheelSetDriving）は Part をさらに part(theta1(fixed = true)) と修正する。
  //
  // 正しくは、この修正は part.theta1 だけに効き、part.joint.theta1 は fixed = false のまま（初期条件 2 / 2 で合う）。
  // Rumoca v0.10.2 は part.joint.theta1 と part.joint.der_theta1 にも fixed = true を付けてしまうので、
  // S8 の初期化系のマッチングが「冗長 2」（GM-W8004。別名の両方が fixed = true）を出す。
  // 外からの修正が無い形（Part をそのまま使う）では正しく 2 / 2 になる（S009 で確かめた。上流への報告の候補）。
  // db のテスト `modifier_leaks_into_subcomponents_in_the_compiler` が落ちたら Rumoca が直った印。
  model Joint "Joints.RollingWheelSet の形: fixed は書かない"
    parameter StateSelect stateSelect = StateSelect.prefer;
    Real theta1(start = 0, stateSelect = stateSelect);
    Real der_theta1(start = 0, stateSelect = stateSelect);
  equation
    der_theta1 = der(theta1);
    der(der_theta1) = -theta1;
  end Joint;
  model Part "Parts.RollingWheelSet の形"
    parameter StateSelect stateSelect = StateSelect.prefer;
    Real theta1(start = 0, fixed = true, stateSelect = stateSelect);
    Real der_theta1(start = 0, fixed = true, stateSelect = stateSelect);
    Joint joint(stateSelect = StateSelect.default, theta1(fixed = false), der_theta1(fixed = false));
  equation
    joint.theta1 = theta1;
    der_theta1 = der(theta1);
  end Part;
  Part part(theta1(fixed = true), der_theta1(fixed = true));
end ModifierScope;
