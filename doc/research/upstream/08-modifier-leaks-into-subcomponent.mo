// Reduction of Rumoca issue draft 08. Shape of MSL Mechanics.MultiBody.Parts.RollingWheelSet.
// Part fixes its own theta1 and leaves joint.theta1 free (fixed = false), tied by joint.theta1 = theta1.
// The modifier part(theta1(fixed = true)) must only affect part.theta1.
// Rumoca v0.10.2 also marks part.joint.theta1 and part.joint.der_theta1 fixed = true.
model ModifierScope
  model Joint
    parameter StateSelect stateSelect = StateSelect.prefer;
    Real theta1(start = 0, stateSelect = stateSelect);
    Real der_theta1(start = 0, stateSelect = stateSelect);
  equation
    der_theta1 = der(theta1);
    der(der_theta1) = -theta1;
  end Joint;
  model Part
    parameter StateSelect stateSelect = StateSelect.prefer;
    Real theta1(start = 0, fixed = true, stateSelect = stateSelect);
    Real der_theta1(start = 0, fixed = true, stateSelect = stateSelect);
    Joint joint(stateSelect = StateSelect.default, theta1(fixed = false), der_theta1(fixed = false));
  equation
    joint.theta1 = theta1;
    der_theta1 = der(theta1);
  end Part;
  Part part(theta1(fixed = true), der_theta1(fixed = true));
end ModifierScope;
