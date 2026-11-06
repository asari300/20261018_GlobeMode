// Repro for upstream draft 25e (fifth shape, S083). Self-contained.
// Shape taken from MSL 4.1.0 Modelica.Media.R134a: the record
// Common.EOSIdealCoeff declares `parameter Integer nc = 5;
// parameter Real[nc] a`, and the function fid_R134a declares
// `protected Modelica.Media.R134a.R134aData.Ideal id` (a record that extends it).
package Rep31

  record Coeff "array dimension is a component of the record"

    parameter Integer nc = 2;
    parameter Real[nc] a = {1, 2};

  end Coeff;

  record CoeffC "the same with constant components"

    constant Integer nc = 2;
    constant Real[nc] a = {1, 2};

  end CoeffC;

  record Coeff0 "control: literal dimension"

    parameter Real[2] a = {1, 2};

  end Coeff0;

  function f "record as a protected local"

    input Real x;
    output Real y;
  protected
    Coeff id;

  algorithm

    y := id.a[1] * x;

  end f;

  function fC "record with constant components as a protected local"

    input Real x;
    output Real y;
  protected
    CoeffC id;

  algorithm

    y := id.a[1] * x;

  end fC;

  function f0 "control: record with a literal dimension"

    input Real x;
    output Real y;
  protected
    Coeff0 id;

  algorithm

    y := id.a[1] * x;

  end f0;

  model M

    Real y = f(time);

  end M;

  model MC "constant components: the same error"

    Real y = fC(time);

  end MC;

  model M0 "control: literal dimension"

    Real y = f0(time);

  end M0;

  model M1 "control: the same record as a model component"

    Coeff id;
    Real y = id.a[1] * time;

  end M1;

end Rep31;
