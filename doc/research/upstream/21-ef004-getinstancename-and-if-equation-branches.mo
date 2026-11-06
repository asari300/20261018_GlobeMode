// Repro for upstream draft 21. Self-contained, no MSL needed.
// A: getInstanceName() in a parameter binding (ThermofluidStream
//    "final parameter String name = getInstanceName();") and in an
//    assert message (IBPSA Boundary_pT). Rep21A2 and Rep21A3 (assert in an
//    equation / initial equation section) compile; Rep21A (binding) fails.
model Rep21A

  final parameter String name = getInstanceName();
  Real x = 1;

end Rep21A;

model Rep21A2

  parameter Real p = 1;
  Real x = 1;

equation

  assert(p > 0, "In " + getInstanceName() + ": p must be positive");

end Rep21A2;

model Rep21A3

  parameter Real p = 1;
  Real x = 1;

initial equation

  assert(p > 0, "In " + getInstanceName() + ": p must be positive");

end Rep21A3;

model Rep21AOuter

  Rep21A a;

end Rep21AOuter;

// B: Chemical.Processes.* (Processes.mo:1174). The first branch of a dynamic
//    if-equation contains a product of a field of a component array
//    (substrates.n_flow, an array of Real) with a vector. Rumoca says
//    "dynamic if-equation branches have different scalar row counts"
//    when the array size is not 1.
record Rep21Rec

  Real n_flow = 1;

end Rep21Rec;

model Rep21B

  parameter Integer n = 2;
  Rep21Rec s[n];
  Real q(start = 1);
  Real hMix;

equation

  der(q) = -q;
  if noEvent(q > 0.5) then
    hMix * (s.n_flow * ones(n)) = 0;
  else
    hMix = 0;
  end if;

end Rep21B;

model Rep21B1 = Rep21B(n = 1); // Control: n = 1 is accepted.
model Rep21B3 = Rep21B(n = 3);

model Rep21B4

  // Same equation outside an if-equation: no EF004, but the row count is
  // wrong too (ED001: 4 equations, 3 unknowns; omc: 3 and 3).
  parameter Integer n = 2;
  Rep21Rec s[n];
  Real hMix;

equation

  hMix * (s.n_flow * ones(n)) = 0;

end Rep21B4;
