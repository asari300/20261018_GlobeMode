// Repro for upstream draft 28 (S091). Self-contained.
// An array dimension that is a constant of another package fails (ET004),
// or EF024 through a replaceable package that is not redeclared.
// It passes only when the package reaches the model through `redeclare package`.
package Rep36
  package P
    constant Integer n = 2;
  end P;

  model M "dimension is a constant of another package"
    Real x[P.n];
  equation
    x = ones(2)*time;
  end M;

  model M1 "the same through a replaceable package (default, not redeclared)"
    replaceable package Medium = P;
    Real x[Medium.n];
  equation
    x = ones(2)*time;
  end M1;

  model M2 "a parameter bound to the package constant"
    parameter Integer m = P.n;
    Real x[m];
  equation
    x = ones(2)*time;
  end M2;

  model C
    replaceable package Medium = P;
    Real x[Medium.n];
  equation
    x = ones(2)*time;
  end C;

  model M3 "a component that uses the default of its replaceable package"
    C c;
  end M3;

  model M0 "control: a constant of the model itself"
    constant Integer n = 2;
    Real x[n];
  equation
    x = ones(2)*time;
  end M0;

  model M4 "control: the same component with redeclare"
    C c(redeclare package Medium = P);
  end M4;
end Rep36;
