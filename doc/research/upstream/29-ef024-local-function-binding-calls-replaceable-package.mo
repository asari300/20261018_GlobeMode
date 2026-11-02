// Repro for upstream draft 29 (S091). Self-contained.
// A function declared inside a model whose protected variable has a binding
// that calls a function of the model's replaceable package fails with EF024.
package Rep37
  package P
    function f
      input Real u;
      output Real y;
    algorithm
      y := 2*u;
    end f;
  end P;

  model M "the call is in the binding of a protected variable"
    replaceable package Medium = P;
    Real y;
  protected
    function g
      input Real u;
      output Real z;
    protected
      Real w = Medium.f(u);
    algorithm
      z := w;
    end g;
  equation
    y = g(time);
  end M;

  model M1 "the same with redeclare, as the libraries do"
    extends M(redeclare package Medium = P);
  end M1;

  model M0 "control: the call in the algorithm"
    replaceable package Medium = P;
    Real y;
  protected
    function g
      input Real u;
      output Real z;
    algorithm
      z := Medium.f(u);
    end g;
  equation
    y = g(time);
  end M0;

  model M2 "control: the package by its own name"
    Real y;
  protected
    function g
      input Real u;
      output Real z;
    protected
      Real w = P.f(u);
    algorithm
      z := w;
    end g;
  equation
    y = g(time);
  end M2;
end Rep37;
