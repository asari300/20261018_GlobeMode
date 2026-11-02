// Repro for upstream draft 25 (`splineDerivatives`, S083). Self-contained.
// Shape taken from IBPSA (and AixLib, Buildings)
// Utilities.Math.Functions.splineDerivatives: the output array is defined
// element by element in a `for` loop, and a later `if` whose branch is only
// a `for` loop overwrites elements ("Ensure monotonicity").
package Rep33
  function f "if branch with only a for loop, after an element-wise loop"
    input Real y[:];
    input Boolean b;
    output Real d[size(y, 1)];
  algorithm
    for i in 1:size(y, 1) loop
      d[i] := y[i];
    end for;
    if b then
      for i in 1:size(y, 1) loop
        d[i] := 0;
      end for;
    end if;
  end f;

  function f0 "control: whole-array assignment before the if"
    input Real y[:];
    input Boolean b;
    output Real d[size(y, 1)];
  algorithm
    d := y;
    if b then
      for i in 1:size(y, 1) loop
        d[i] := 0;
      end for;
    end if;
  end f0;

  function f1 "control: the branch is an assignment"
    input Real y[:];
    input Boolean b;
    output Real d[size(y, 1)];
  algorithm
    for i in 1:size(y, 1) loop
      d[i] := y[i];
    end for;
    if b then
      d := zeros(size(y, 1));
    end if;
  end f1;

  model M
    parameter Real d[3] = f({1, 2, 3}, false);
    Real z = d[2] * time;
  end M;

  model M0 "control: d := y first"
    parameter Real d[3] = f0({1, 2, 3}, false);
    Real z = d[2] * time;
  end M0;

  model M1 "control: assignment in the branch"
    parameter Real d[3] = f1({1, 2, 3}, false);
    Real z = d[2] * time;
  end M1;
end Rep33;
