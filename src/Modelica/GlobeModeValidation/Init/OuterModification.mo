within GlobeModeValidation.Init;
model OuterModification "inner の修正が outer に伝わらない（Rumoca v0.10.2 の制約の記録。S8 は初期方程式 0 / 2 になる）"
  // MSL の Fluid の例題の形: inner の System で energyDynamics を FixedInitial に修正し、部品（outer System system）が
  // それを読んで、initial equation の if で初期条件を与える。
  //
  // 正しくは、初期方程式 p = ps_start（2 本）が有効で、S8 は 2 / 2 で合う。
  // Rumoca v0.10.2 は outer の参照を inner の修正ではなく型の既定（Free）で評価するので、if の中の初期方程式が消え、
  // S8 は 0 / 2（不足。GM-W8001）になる。型の既定を FixedInitial にすれば 2 / 2 になる（S009 で確かめた。
  // 上流への報告の候補）。globe の判定ではなく Rumoca の制約なので、Rumoca が直ればこのモデルの S8 は合うようになる
  // （db のテスト `outer_modification_is_lost_by_the_compiler` が落ちたら、この記録を直す）。
  type Dynamics = enumeration(FixedInitial, Free);
  model System
    parameter Dynamics energyDynamics = Dynamics.Free;
    parameter Dynamics massDynamics = energyDynamics;
  end System;
  model Volume
    outer System system;
    parameter Dynamics massDynamics = system.massDynamics;
    parameter Real ps_start[2] = {1, 2};
    Real p[2](start = ps_start);
  initial equation
    if massDynamics == Dynamics.FixedInitial then
      p = ps_start;
    end if;
  equation
    der(p) = -p;
  end Volume;
  inner System system(energyDynamics = Dynamics.FixedInitial);
  Volume volume;
end OuterModification;
