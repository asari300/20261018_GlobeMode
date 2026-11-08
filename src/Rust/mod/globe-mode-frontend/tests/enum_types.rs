//! S224: FMU の Export が使う、Enumeration 型の変数と String パラメータの取り出し。

use globe_mode_frontend::{Checker, Source};

const MODEL: &str = "
package P
  type Speed = enumeration(slow, fast, 'x y');
  model M
    parameter Speed speed = Speed.fast;
    parameter String label = \"a&b\";
    parameter String computed = label + \"!\";
    parameter Real k = 1;
    Real x(start = 1, fixed = true);
  equation
    der(x) = -k * x;
  end M;
end P;";

#[test]
fn enum_types_and_string_parameters() {
    let checker = Checker::new(&[]).unwrap();
    let src = [Source {
        name: "m.mo",
        text: MODEL,
    }];
    let m = checker.compile_retaining(&src, "P.M").unwrap();
    let t = m.enum_type_of("speed").unwrap();
    eprintln!("{t:?}");
    assert_eq!(t.literals, ["slow", "fast", "x y"]);
    assert!(m.enum_type_of("k").is_none());
    assert_eq!(
        m.string_parameters(),
        [("label".to_string(), "a&b".to_string())]
    );
}
