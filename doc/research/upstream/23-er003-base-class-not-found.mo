// Repro for S072 (ER003). Self-contained. Not an upstream draft: a base class
// that does not exist is an error in every tool (MLS 3.6 section 7.1). In the
// public libraries this comes from a dependency GlobeMode does not fetch
// (ClaRa extends SMArtInt.Internal.ClaRaDelay) or from an outdated MSL path in
// AixLib (Modelica.Media.Examples.Tests.Components.PartialTestModel2; the class
// is Modelica.Media.Examples.Utilities.PartialTestModel2 in MSL 4.0.0 and 4.1.0).
package Rep23

  package Present

    partial model Base

      Real x(start = 1);

    equation

      der(x) = -x;

    end Base;

  end Present;

  model M "extends a class that is not loaded"

    extends Missing.Base;

  end M;

  model M0 "control: the same with the class present"

    extends Present.Base;

  end M0;

end Rep23;
