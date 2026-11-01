// Repro for upstream draft 17. Self-contained, no MSL needed.
package Rep17
  partial package Base
    replaceable partial function product "Multiply two vectors"
      input Real x[2];
      input Real y[2];
      output Real z[2];
    end product;

    replaceable partial function sum2 "Control: same shape, other name"
      input Real x[2];
      output Real z[2];
    end sum2;
  end Base;

  package One
    extends Base;

    redeclare function extends product "ok in OpenModelica"
    algorithm
      z := {x[1]*y[1] - x[2]*y[2], x[1]*y[2] + x[2]*y[1]};
    end product;

    redeclare function extends sum2
    algorithm
      z := x;
    end sum2;
  end One;

  model M
    Real a[2] = {1, 2};
    Real b[2] = One.product(a, a);
    Real c[2] = One.sum2(a);
  end M;
end Rep17;
