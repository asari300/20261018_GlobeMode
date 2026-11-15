function Scale
  input Real gain;
  input Real offset;
  output Real y = gain + offset;
end Scale;
function Twice
  input Real v;
  output Real y = Scale(v, v);
end Twice;
model Inlay
  Real x = Scale(2, 3) + sin(time);
end Inlay;
