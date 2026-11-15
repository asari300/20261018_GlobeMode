function Scale
  input Real gain;
  input Real offset;
  output Real y = gain + offset;
end Scale;
model Inlay
  Real x = Scale(2, 3) + sin(time);
end Inlay;
