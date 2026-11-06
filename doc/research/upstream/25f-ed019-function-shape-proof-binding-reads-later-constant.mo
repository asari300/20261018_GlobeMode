// Repro for upstream draft 25f (sixth shape, S083). Self-contained.
// Shape taken from IBPSA (and AixLib, Buildings) Media.Steam.temperature_ph:
// `protected constant Real b[:] = {-a[1]*TSD/a[3]+TMean, ...}` is declared
// before the constants `TMean` and `TSD` that its binding reads.
package Rep32

  function f "b[:] reads a constant declared after it"

    input Real x;
    output Real y;
  protected
    constant Real b[:] = {c, 2};
    constant Real c = 3;

  algorithm

    y := b[1] + b[2] * x;

  end f;

  function f0 "control: c declared first"

    input Real x;
    output Real y;
  protected
    constant Real c = 3;
    constant Real b[:] = {c, 2};

  algorithm

    y := b[1] + b[2] * x;

  end f0;

  function f1 "control: literal binding"

    input Real x;
    output Real y;
  protected
    constant Real b[:] = {3, 2};

  algorithm

    y := b[1] + b[2] * x;

  end f1;

  model M

    Real y = f(time);

  end M;

  model M0 "control: declaration order"

    Real y = f0(time);

  end M0;

  model M1 "control: literal binding"

    Real y = f1(time);

  end M1;

end Rep32;
