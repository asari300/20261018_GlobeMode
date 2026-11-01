// Repro for upstream draft 20. Needs only the Modelica Standard Library.
model Rep20Load
  // MSL function called in a parameter binding (IBPSA/Buildings use this
  // for weather-file and table paths).
  parameter String fileName = Modelica.Utilities.Files.loadResource(
    "modelica://Modelica/Resources/Data/Tables/test.txt");
  Real x = 1;
end Rep20Load;

model Rep20Load0
  // Control: no function call.
  parameter String fileName = "x";
  Real x = 1;
end Rep20Load0;

package Rep20TwoExtends
  // Self-contained shape of MSL Files.loadResource: an interface function
  // and an implementation function, both extended by the public function.
  partial function PartialF
    input String uri;
    output String path;
  end PartialF;

  function Impl
    input String uri;
    output String path;
  algorithm
    path := uri;
  end Impl;

  function f
    extends PartialF;
    extends Impl;
  end f;

  model M
    parameter String s = f("a");
    Real x = 1;
  end M;
end Rep20TwoExtends;
