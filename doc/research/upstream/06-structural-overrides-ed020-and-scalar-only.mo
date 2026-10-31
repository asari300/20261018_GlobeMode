// Reduction of Rumoca issue draft 06 (item 3). k is passed down with a same-name modifier (final k = k).
// Overriding src.k[1] to 0.2 should move the switch s of the inner block from t = 0.8 to t = 0.2.
model ArrayParameters
  block Inner
    parameter Real k[2] = {10, 20};
    Real y = k[2];
    Real s = if time < k[1] then 0 else 1;
  end Inner;

  block Outer
    parameter Real k[2] = {10, 20};
    Inner i(final k = k);
    Real y = i.y;
    Real s = i.s;
  end Outer;

  parameter Real a[3] = {1, 2, 3};
  Outer src(k = {0.8, 20});
  Real y_a = a[2];
  annotation (experiment(StopTime = 1));
end ArrayParameters;
