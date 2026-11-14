use globe_mode_flat::{FlatModel, Project};

fn flat(src: &str, model: &str) -> FlatModel {
    Project::from_sources([("t.mo", src)])
        .flatten(model)
        .unwrap_or_else(|e| panic!("{e}"))
}

fn names(m: &FlatModel) -> Vec<&str> {
    m.var_names()
}

fn eqs(m: &FlatModel) -> Vec<String> {
    m.equations.iter().map(|e| e.text.clone()).collect()
}

fn err(src: &str, model: &str) -> String {
    Project::from_sources([("t.mo", src)])
        .flatten(model)
        .expect_err("should be unsupported")
        .kind
        .to_string()
}

#[test]
fn nested_extends_and_modifications() {
    let m = flat(
        "model A parameter Real k = 2; Real x(start = 1, unit = \"m\"); equation der(x) = -k * x; end A;
         model C extends A(k = 5); Real z = k + 1; end C;
         model B C c(k = 3, x(start = 9)); A a; Real y = a.x + c.z; end B;",
        "B",
    );
    // 外側の修飾が勝つ（c.k は B の 3、`extends A(k = 5)` の 5 ではない）
    let k = m.vars.iter().find(|v| v.name == "c.k").unwrap();
    assert_eq!(k.binding.as_deref(), Some("3"));
    assert_eq!(
        m.vars
            .iter()
            .find(|v| v.name == "a.k")
            .unwrap()
            .binding
            .as_deref(),
        Some("2")
    );
    let x = m.vars.iter().find(|v| v.name == "c.x").unwrap();
    assert_eq!(x.start.as_deref(), Some("9"));
    assert_eq!(x.attrs, vec![("unit".to_string(), "\"m\"".to_string())]);
    assert_eq!(names(&m), ["c.k", "c.x", "c.z", "a.k", "a.x", "y"]);
    // 方程式の名前の付け替え（部品の中の k は c.k になる）
    assert_eq!(
        eqs(&m),
        ["der(c.x) = - c.k * c.x", "der(a.x) = - a.k * a.x"]
    );
    // 束縛は変数に残る（Rumoca の flat と同じ）。DAE では方程式 2 つ
    let z = m.vars.iter().find(|v| v.name == "c.z").unwrap();
    assert_eq!(z.binding.as_deref(), Some("c.k + 1"));
    assert_eq!((m.equation_count(), m.binding_equations()), (2, 2));
}

#[test]
fn connect_potential_and_flow() {
    let src = "connector Pin Real v; flow Real i; end Pin;
        model R Pin p; Pin n; parameter Real r = 1; equation p.v - n.v = r * p.i; p.i + n.i = 0; end R;
        model G Pin p; equation p.v = 0; end G;
        model Top R r1; R r2; G g; Pin out;
        equation connect(r1.p, g.p); connect(r1.n, r2.p); connect(r2.n, out); end Top;
        model Open R r1; end Open;";
    let m = flat(src, "Top");
    let v = |n: &str| m.vars.iter().find(|v| v.name == n).unwrap();
    assert!(v("r1.p.i").flow && !v("r1.p.v").flow);
    let e = eqs(&m);
    // 部品の方程式 2+2+1、potential の等式 3、flow の和 3
    // 最上位のコネクタ out の flow は 0 になる（+1）
    assert_eq!(m.equation_count(), 5 + 3 + 3 + 1, "{e:#?}");
    assert!(e.contains(&"out.i = 0".to_string()));
    assert!(e.contains(&"r1.p.v = g.p.v".to_string()));
    assert!(e.contains(&"r1.p.i + g.p.i = 0".to_string()));
    // 外側のコネクタ `out` は符号が逆（内側の r2.n が + で out は -）
    assert!(e.contains(&"r2.n.i - out.i = 0".to_string()), "{e:#?}");
    // つなげなかった内側のコネクタの flow は 0
    let open = flat(src, "Open");
    assert!(eqs(&open).contains(&"r1.p.i = 0".to_string()));
    assert_eq!(open.equation_count(), 2 + 2);
}

#[test]
fn arrays_and_for() {
    let m = flat(
        "model Cell Real x(start = 0); equation der(x) = 1; end Cell;
         model M parameter Integer n = 3; Real y[n]; Cell c[n]; Real w[2, 2];
         equation for i in 1:n loop y[i] = c[i].x * i; end for; w = {{1, 2}, {3, 4}};
         end M;",
        "M",
    );
    assert_eq!(names(&m), ["n", "y", "c[1].x", "c[2].x", "c[3].x", "w"]);
    assert_eq!(m.vars[1].dims, [3]);
    assert_eq!(m.vars[5].dims, [2, 2]);
    let e = eqs(&m);
    assert!(e.contains(&"y[2] = c[2].x * 2".to_string()), "{e:#?}");
    // c の 3 + for の 3 + 配列の等式（w は 4 つ）
    assert_eq!(m.equation_count(), 3 + 3 + 4);
}

#[test]
fn connect_in_for_and_if_kept_as_text() {
    let m = flat(
        "connector P Real v; flow Real i; end P;
         model Node P p; end Node;
         model Chain parameter Integer n = 3; Node a[n];
         equation for i in 1:n - 1 loop connect(a[i].p, a[i + 1].p); end for;
           if n > 2 then a[1].p.v = 0; else a[1].p.v = 1; end if;
         end Chain;",
        "Chain",
    );
    let e = eqs(&m);
    assert!(e.contains(&"a[1].p.v = a[2].p.v".to_string()), "{e:#?}");
    assert!(e.iter().any(|t| t.starts_with("if n > 2 then")), "{e:#?}");
    assert!(m.equation_count() >= 3);
}

#[test]
fn declined_forms() {
    let conn = "connector P Real v; flow Real i; stream Real h; end P;";
    assert_eq!(
        err("model A replaceable Real x; end A;", "A"),
        "replaceable"
    );
    assert_eq!(err("model A inner Real x; end A;", "A"), "inner-outer");
    assert_eq!(
        err(
            "model A parameter Boolean b = true; Real x if b; end A;",
            "A"
        ),
        "conditional-component"
    );
    assert_eq!(
        err(
            "model B Real x; end B; model A extends B(redeclare Real x); end A;",
            "A"
        ),
        "redeclare"
    );
    assert_eq!(
        err(
            &format!(
                "{conn} model N P p; end N; model A N a; N b; equation connect(a.p, b.p); end A;"
            ),
            "A"
        ),
        "stream"
    );
    assert_eq!(err("model A Real x[:]; end A;", "A"), "unsized-array");
    let m = flat(
        "model A parameter Real x[:] = {1, 2, 3}; Real y[:, :] = {{1, 2}, {3, 4}}; end A;",
        "A",
    );
    assert_eq!(
        (m.vars[0].dims.clone(), m.vars[1].dims.clone()),
        (vec![3], vec![2, 2])
    );
    assert_eq!(
        err("model A Real x; Real y[x]; end A;", "A"),
        "non-constant-dim"
    );
    assert_eq!(
        err("model A Real x; equation x = Missing.f(2); end A;", "A"),
        "unresolved"
    );
}

#[test]
fn when_is_listed_apart() {
    let m = flat(
        "model W Real x(start = 0); Integer n(start = 0);
         equation der(x) = 1; when x > 1 then n = pre(n) + 1; end when; end W;",
        "W",
    );
    assert_eq!((m.equation_count(), m.when_equations.len()), (1, 1));
    assert_eq!(m.when_equations[0].count, 1);
}

#[test]
fn types_and_parameter_dims_from_outside() {
    let m = flat(
        "type Volt = Real(unit = \"V\", start = 5);
         connector In = input Real;
         model S parameter Integer n = 2; Real v[n]; end S;
         model T S s(n = 4); Volt u; In i; Boolean b = true; end T;",
        "T",
    );
    assert_eq!(m.vars.iter().find(|v| v.name == "s.v").unwrap().dims, [4]);
    let u = m.vars.iter().find(|v| v.name == "u").unwrap();
    assert_eq!(u.start.as_deref(), Some("5"));
    assert_eq!(u.attrs, vec![("unit".to_string(), "\"V\"".to_string())]);
    let i = m.vars.iter().find(|v| v.name == "i").unwrap();
    assert_eq!(i.causality, globe_mode_flat::Causality::Input);
    let b = m.vars.iter().find(|v| v.name == "b").unwrap();
    assert_eq!(b.binding.as_deref(), Some("true"));
    assert_eq!(b.kind, globe_mode_flat::VarKind::Discrete);
}
