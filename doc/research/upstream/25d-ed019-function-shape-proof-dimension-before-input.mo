// Repro for upstream draft 25 (fourth shape, S083). Self-contained.
// Shape taken from IBPSA (and AixLib, Buildings)
// Controls.SetPoints.OccupancySchedule: the protected function getOutput
// declares `input Real occupancy[nRow]` before `input Integer nRow`, and the
// block is used as a component in the example.
package Rep30
  function f "input dimension is an Integer input declared after it"
    input Real x[n];
    input Integer n;
    output Real y;
  algorithm
    y := sum(x);
  end f;

  function g "control: the Integer input comes first"
    input Integer n;
    input Real x[n];
    output Real y;
  algorithm
    y := sum(x);
  end g;

  function h "control: x[:]"
    input Real x[:];
    input Integer n;
    output Real y;
  algorithm
    y := sum(x);
  end h;

  block B
    parameter Real a[:] = {1, 2};
    Real y = f(a, size(a, 1)) * time;
  end B;

  block Bg
    parameter Real a[:] = {1, 2};
    Real y = g(size(a, 1), a) * time;
  end Bg;

  block Bh
    parameter Real a[:] = {1, 2};
    Real y = h(a, size(a, 1)) * time;
  end Bh;

  model M "the block as a component"
    B b;
  end M;

  model M0 "control: n first"
    Bg b;
  end M0;

  model M1 "control: x[:]"
    Bh b;
  end M1;

  model M2 "control: the same block, extended instead of a component"
    extends B;
  end M2;
end Rep30;
