// Repro for upstream draft 19. Self-contained, no MSL needed.
// A: stateSelect from an if-expression over a parameter of a sibling
//    component (IBPSA ConservationEquation: medium.preferredMediumStates).
// B: stateSelect forwarded as a parameter through a modification
//    (MSL Mechanics.MultiBody.Joints.Revolute passes its parameter
//    stateSelect to phi and w).
model Rep19Props

  parameter Boolean preferredMediumStates = false;
  Real T(start = 300);

equation

  der(T) = -T + 1;

end Rep19Props;

model Rep19A

  Rep19Props medium(
    T(
      stateSelect = if medium.preferredMediumStates then StateSelect.prefer else StateSelect.default
    )
  );

end Rep19A;

model Rep19A0

  // Control: literal value.
  Rep19Props medium(T(stateSelect = StateSelect.prefer));

end Rep19A0;

model Rep19Joint

  parameter StateSelect stateSelect = StateSelect.prefer;
  Real phi(start = 0, stateSelect = stateSelect);

equation

  der(phi) = 1 - phi;

end Rep19Joint;

model Rep19B

  parameter StateSelect ss = StateSelect.always;
  Rep19Joint j(stateSelect = ss);

end Rep19B;

model Rep19B0

  // Control: literal value.
  Rep19Joint j(stateSelect = StateSelect.always);

end Rep19B0;
