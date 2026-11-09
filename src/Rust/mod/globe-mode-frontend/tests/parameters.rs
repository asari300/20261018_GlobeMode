//! S248: パラメータのダイアログが使う、flat モデルのパラメータ宣言（型・入れ子の名前・修飾で変えた既定値・単位）。

use globe_mode_frontend::{Checker, Source};

const MODEL: &str = "
package P
  type Speed = enumeration(slow, fast, 'x y');
  model Part
    parameter Real k(unit = \"1/s\", min = 0, max = 10) = 1 \"rate\";
    parameter Integer n = 3;
    parameter Boolean on = true;
    parameter Speed speed = Speed.slow;
    parameter String label = \"a\";
    parameter Real[2] v = {1, 2};
    parameter Real twice = 2 * k;
    Real x(start = 1, fixed = true);
  equation
    der(x) = -k * x;
  end Part;
  model M
    Part c(k = -5, speed = Speed.'x y', n = 7);
    Part d;
    parameter Real top = 4;
  end M;
end P;";

#[test]
fn parameters_are_nested_typed_and_carry_modifications() {
    let checker = Checker::new(&[]).unwrap();
    let src = [Source {
        name: "m.mo",
        text: MODEL,
    }];
    let m = checker.compile_retaining(&src, "P.M").unwrap();
    let get = |n: &str| {
        m.parameters()
            .iter()
            .find(|p| p.name == n)
            .unwrap_or_else(|| panic!("{n}: {:#?}", m.parameters()))
            .clone()
    };

    let k = get("c.k");
    assert_eq!(
        (k.ty.as_str(), k.value, k.min, k.max),
        ("Real", Some(-5.0), Some(0.0), Some(10.0))
    );
    assert_eq!(k.unit.as_deref(), Some("1/s"));
    assert_eq!(k.description.as_deref(), Some("rate"));
    assert_eq!(get("d.k").value, Some(1.0));
    assert_eq!(get("top").value, Some(4.0));

    let n = get("c.n");
    assert_eq!((n.ty.as_str(), n.value), ("Integer", Some(7.0)));
    let on = get("d.on");
    assert_eq!((on.ty.as_str(), on.value), ("Boolean", Some(1.0)));
    let sp = get("c.speed");
    assert_eq!((sp.ty.as_str(), sp.value), ("Enumeration", Some(3.0)));
    assert_eq!(sp.literals, ["slow", "fast", "x y"]);
    assert_eq!(get("d.speed").value, Some(1.0));
    let s = get("d.label");
    assert_eq!((s.ty.as_str(), s.value), ("String", None));
    let v = get("d.v");
    assert_eq!(v.dims, [2]);
    // 式の既定値は評価せず、式の字面を返す
    let t = get("d.twice");
    assert_eq!((t.value, t.default.as_deref()), (None, Some("2 * d.k")));
    assert!(m.parameters().iter().all(|p| p.name != "c.x"));
}
