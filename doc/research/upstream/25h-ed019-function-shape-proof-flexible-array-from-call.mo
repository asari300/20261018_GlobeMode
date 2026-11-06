// Repro for upstream draft 25h (seventh shape, S083). Self-contained.
// Shape taken from MSL 4.1.0 Modelica.Media.Air.ReferenceMoistAir.Xsaturation:
// `protected MassFraction[:] X;` is sized only by the assignment
// `X := massFractionSaturation(state);`.
package Rep34

  function g

    input Real x;
    output Real X[2];

  algorithm

    X := {x, 1 - x};

  end g;

  function f "flexible protected array sized by an assignment"

    input Real x;
    output Real y;
  protected
    Real[:] X;

  algorithm

    X := g(x);
    y := X[1];

  end f;

  function f0 "control: the size is declared"

    input Real x;
    output Real y;
  protected
    Real[2] X;

  algorithm

    X := g(x);
    y := X[1];

  end f0;

  function f1 "the same with a binding: also fails"

    input Real x;
    output Real y;
  protected
    Real[:] X = g(x);

  algorithm

    y := X[1];

  end f1;

  model M

    Real y = f(time);

  end M;

  model M0 "control: X[2]"

    Real y = f0(time);

  end M0;

  model M1 "binding instead of assignment: also fails"

    Real y = f1(time);

  end M1;

end Rep34;
