// Repro for S072 (ER023). Self-contained. Not an upstream draft: the
// rejection follows MLS 3.6 section 4.6 (a record may only contain components
// of specialized class record or type; an external object is a class).
// Shape taken from ClaRa.Basics.Records.IComGas_L2 (fluidPointer_bulk) with
// TILMedia.Internals.TILMediaExternalObject.
package Rep22
  class Obj
    extends ExternalObject;
    function constructor
      input Integer n;
      output Obj obj;
    external "C" obj = rep22_open(n);
    end constructor;
    function destructor
      input Obj obj;
    external "C" rep22_close(obj);
    end destructor;
  end Obj;

  record ICom "record with an external-object component (ClaRa iCom)"
    Obj pointer;
    Real p;
  end ICom;

  record ICom0 "control: the same record without the external object"
    Real p;
  end ICom0;

  model M
    parameter Obj o = Obj(1);
    ICom com(pointer = o, p = 1);
    Real y = com.p*time;
  end M;

  model M0
    ICom0 com(p = 1);
    Real y = com.p*time;
  end M0;
end Rep22;
