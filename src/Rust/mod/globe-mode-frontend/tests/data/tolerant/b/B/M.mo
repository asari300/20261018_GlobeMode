within B;
model M
  import A.Units.T;
  T x(start = 1, fixed = true);
equation
  der(x) = -x;
end M;
