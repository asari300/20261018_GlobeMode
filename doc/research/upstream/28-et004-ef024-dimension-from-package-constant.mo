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

  // Added in S108: a record declared inside the model, whose dimension reads
  // the model's replaceable package, fails with EF024 even when the package
  // is redeclared (AixLib `Fluid.Storage.TwoPhaseSeparator`, `mXi[Medium.nXi]`).
  model C5
    replaceable package Medium = P;
    record R
      Real x[Medium.n];
    end R;
    R r;
  equation
    r.x = ones(2)*time;
  end C5;

  model M5 "a local record whose dimension reads the redeclared package"
    C5 c(redeclare package Medium = P);
  end M5;
end Rep36;
