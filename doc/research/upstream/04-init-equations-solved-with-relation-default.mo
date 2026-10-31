// Reduction of Rumoca issue draft 04. Initial equation der(x) = 0 reads u, and u depends on the
// relation v < 0. At t = 0, v = -0.5 so u must be 1 and x(0) must be 1.
// Rumoca v0.10.2 solves the initialization with the relation default (false), giving u = 2, x(0) = 2.
model InitRelationBranch
  parameter Real T = 0.1 "filter time constant";
  Real s(start = 0, fixed = true) "position";
  Real v "velocity, negative at t = 0";
  Real u "input chosen by a relation (1 at t = 0)";
  Real x(start = 0) "first-order filter state, steady-state initialized (correct x(0) = 1)";
equation
  der(s) = v;
  v = -0.5 * cos(s);
  u = if v < 0 then 1 else 2;
  der(x) = (u - x) / T;
initial equation
  der(x) = 0;
  annotation (experiment(StopTime = 1));
end InitRelationBranch;
