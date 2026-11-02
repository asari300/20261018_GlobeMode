// Repro for upstream draft 25. Self-contained.
// Shape taken from IBPSA.Utilities.Math.Functions.isMonotonic (called by
// splineDerivatives; copies in Buildings and AixLib).
package Rep25
  function isIncreasing "Boolean output; one branch assigns and then loops"
    input Real x[:];
    output Boolean m;
  algorithm
    if x[1] > 0 then
      m := true;
    else
      m := true;
      for i in 1:size(x, 1) - 1 loop
        m := m and x[i] < x[i + 1];
      end for;
    end if;
  end isIncreasing;

  function sumOrOne "control: the same shape with a Real output"
    input Real x[:];
    output Real m;
  algorithm
    if x[1] > 0 then
      m := 1;
    else
      m := 1;
      for i in 1:size(x, 1) - 1 loop
        m := m + x[i];
      end for;
    end if;
  end sumOrOne;

  function isIncreasing0 "control: Boolean output, loop outside the if"
    input Real x[:];
    output Boolean m;
  algorithm
    m := true;
    if x[1] <= 0 then
      m := true;
    end if;
    for i in 1:size(x, 1) - 1 loop
      m := m and (x[1] > 0 or x[i] < x[i + 1]);
    end for;
  end isIncreasing0;

  model M
    Boolean b = isIncreasing({1, 2, 3}*(1 + time));
  end M;

  model M0
    Real r = sumOrOne({1, 2, 3}*(1 + time));
  end M0;

  model M1
    Boolean b = isIncreasing0({1, 2, 3}*(1 + time));
  end M1;
end Rep25;
