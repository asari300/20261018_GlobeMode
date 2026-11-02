// Repro for upstream draft 25 (second shape, S080). Self-contained.
// Shape taken from MSL 4.0.0 Modelica.Media.Interfaces.PartialSimpleMedium:
// specificEnthalpy_pTX declares `input MassFraction X[nX]` and BaseProperties
// calls it; ConstantPropertyLiquidWater extends the package with modifiers
// that change nS (and so nX). MSL 4.1.0 declares `X[:]` and compiles.
package Rep27
  package P
    constant Integer n = 1;
    function f "input dimension is the package constant n"
      input Real x[n];
      output Real y;
    algorithm
      y := sum(x);
    end f;
    function g "control: the same function with x[:]"
      input Real x[:];
      output Real y;
    algorithm
      y := sum(x);
    end g;
    model B
      Real x[n];
      Real y;
    equation
      x = fill(time, n);
      y = f(x);
    end B;
    model Bg "control: calls g"
      Real x[n];
      Real y;
    equation
      x = fill(time, n);
      y = g(x);
    end Bg;
  end P;

  package Q "modifies the constant the dimension depends on"
    extends P(n = 2);
  end Q;

  package Q0 "control: extends without a modifier"
    extends P;
  end Q0;

  model M
    replaceable package Medium = Q;
    Medium.B b;
  end M;

  model M0 "control: no modifier"
    replaceable package Medium = Q0;
    Medium.B b;
  end M0;

  model M1 "control: x[:] in the called function"
    replaceable package Medium = Q;
    Medium.Bg b;
  end M1;
end Rep27;
