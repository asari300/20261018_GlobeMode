// Reduction of Rumoca issue draft 07. Pattern of MSL Fluid examples: an inner System with
// energyDynamics = FixedInitial, and a component that reads it through "outer System system".
// Expected: the initial equation p = ps_start (2 scalar equations) is active.
// Rumoca v0.10.2 evaluates the outer reference with the type default (Free) and drops it.
model OuterModification
  type Dynamics = enumeration(FixedInitial, Free);
  model System
    parameter Dynamics energyDynamics = Dynamics.Free;
    parameter Dynamics massDynamics = energyDynamics;
  end System;
  model Volume
    outer System system;
    parameter Dynamics massDynamics = system.massDynamics;
    parameter Real ps_start[2] = {1, 2};
    Real p[2](start = ps_start);
  initial equation
    if massDynamics == Dynamics.FixedInitial then
      p = ps_start;
    end if;
  equation
    der(p) = -p;
  end Volume;
  inner System system(energyDynamics = Dynamics.FixedInitial);
  Volume volume;
end OuterModification;
