// Repro for upstream draft 30 (S091). Self-contained.
// A record parameter with `y[:]` and `eta[size(y, 1)]`, bound to a function
// call that returns the record, fails with ET004 on `size(y, 1)`.
package Rep38

  record R

    parameter Real y[:];
    parameter Real eta[size(y, 1)];

  end R;

  function f

    input Real k;
    output R r(y = {0, 1}, eta = zeros(2));

  algorithm

    r.eta := k * ones(2);

  end f;

  model M "record parameter bound to a function call"

    parameter R p = f(2);
    Real z = p.eta[1] * time;

  end M;

  model M0 "control: the record fields given directly"

    parameter R p(y = {0, 1}, eta = {2, 2});
    Real z = p.eta[1] * time;

  end M0;

end Rep38;
