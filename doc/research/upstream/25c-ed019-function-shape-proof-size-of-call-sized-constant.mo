// Repro for upstream draft 25 (third shape, S083). Self-contained.
// Shape taken from MSL 4.1.0 Modelica.Media.Incompressible.TableBased:
// `npolHeatCapacity = npol`,
// `final constant Real poly_Cp[:] = Polynomials.fitting(..., npolHeatCapacity)`
// (fitting declares `output Real p[n + 1]`), and `specificEntropy` declares
// `protected Integer npol = size(poly_Cp, 1) - 1`.
package Rep29
  function fit "output dimension depends on the input n"
    input Integer n;
    output Real p[n + 1];
  algorithm
    p := ones(n + 1);
  end fit;

  package P
    constant Integer n = 1;
    constant Integer nc = n;
    constant Real c[:] = fit(nc) "size comes from the call";
    constant Real d[:] = {1, 1} "control: literal binding";

    function f "Integer local bound to size(c, 1), named like the constant"
      input Real x;
      output Real y;
    protected
      Integer n = size(c, 1) - 1;
    algorithm
      y := n * x;
    end f;

    function f2 "the same, local with another name"
      input Real x;
      output Real y;
    protected
      Integer k = size(c, 1) - 1;
    algorithm
      y := k * x;
    end f2;

    function f0 "control: the same with d"
      input Real x;
      output Real y;
    protected
      Integer k = size(d, 1) - 1;
    algorithm
      y := k * x;
    end f0;

    function f1 "control: size(c, 1) without the Integer local"
      input Real x;
      output Real y;
    algorithm
      y := (size(c, 1) - 1) * x;
    end f1;
  end P;

  model M
    Real y = P.f(time);
  end M;

  model M2 "local not named like the constant: another message"
    Real y = P.f2(time);
  end M2;

  model M0 "control: literal binding"
    Real y = P.f0(time);
  end M0;

  model M1 "control: no Integer local"
    Real y = P.f1(time);
  end M1;
end Rep29;
