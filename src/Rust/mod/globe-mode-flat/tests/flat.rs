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
           if time > 1 then a[1].p.v = 0; else a[1].p.v = 1; end if;
           if n > 2 then a[2].p.v = 0; elseif n > 1 then a[2].p.v = 1; end if;
           if n > 5 then a[3].p.v = 0; end if;
         end Chain;",
        "Chain",
    );
    let e = eqs(&m);
    assert!(e.contains(&"a[1].p.v = a[2].p.v".to_string()), "{e:#?}");
    assert!(
        e.iter().any(|t| t.starts_with("if time > 1 then")),
        "{e:#?}"
    );
    // 定数の条件の `if` は選んだ枝だけ（偽で else が無いものは消える）
    assert!(e.contains(&"a[2].p.v = 0".to_string()), "{e:#?}");
    assert!(
        !e.iter()
            .any(|t| t.contains("n > ") || t.starts_with("a[3].p.v =")),
        "{e:#?}"
    );
    assert!(m.equation_count() >= 3);
}

#[test]
fn declined_forms() {
    let conn = "connector P Real v; flow Real i; stream Real h; end P;";
    // inner が無い outer、定数に評価できない条件
    assert_eq!(err("model A outer Real x; end A;", "A"), "inner-outer");
    assert_eq!(
        err("model A input Boolean b; Real x if b; end A;", "A"),
        "conditional-component"
    );
    // 型が決まらない redeclare（長い形のクラス）は断る
    assert_eq!(
        err(
            "model B replaceable model M Real x; end M; M m; end B; model A extends B(redeclare model M Real y; end M); end A;",
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

#[test]
fn conditional_component_and_connect() {
    let src = "connector P Real v; flow Real i; end P;
        model R P p; P n; equation p.v - n.v = p.i; p.i + n.i = 0; end R;
        model Top
          parameter Boolean use = false; parameter Integer n = 2;
          R a; R b if use; R c if n > 1; P out; P hp if use;
        equation connect(a.n, b.p); connect(a.p, hp); connect(a.n, c.p); connect(c.n, out);
        end Top;";
    let m = flat(src, "Top");
    let n = names(&m);
    // 偽の b と hp は消え、真の c は普通の宣言になる
    assert!(n.contains(&"c.p.v") && !n.iter().any(|x| x.starts_with("b.") || x.starts_with("hp")));
    let e = eqs(&m);
    // b・hp につながる connect は落ち、a.n と c.p だけがつながる
    assert!(e.contains(&"a.n.v = c.p.v".to_string()), "{e:#?}");
    assert!(
        !e.iter().any(|x| x.contains("b.") || x.contains("hp")),
        "{e:#?}"
    );
}

#[test]
fn inner_outer_alias() {
    let m = flat(
        "model Env parameter Real g = 9.81; end Env;
         model Body outer Env env; Real x; equation der(x) = -env.g; end Body;
         model Mid Body body; end Mid;
         model Top inner Env env(g = 3); Mid mid; Body b; end Top;",
        "Top",
    );
    // outer の env は部品にならず、参照が外側の inner（env）に付け替わる
    assert_eq!(names(&m), ["env.g", "mid.body.x", "b.x"]);
    assert_eq!(eqs(&m), ["der(mid.body.x) = - env.g", "der(b.x) = - env.g"]);
    // 一番近い inner が勝つ
    let m = flat(
        "model Env parameter Real g = 9.81; end Env;
         model Body outer Env env; Real x; equation der(x) = -env.g; end Body;
         model Mid inner Env env(g = 1); Body body; end Mid;
         model Top inner Env env(g = 3); Mid mid; end Top;",
        "Top",
    );
    assert_eq!(eqs(&m), ["der(mid.body.x) = - mid.env.g"]);
}

#[test]
fn record_binding_and_constant_dims() {
    let m = flat(
        "record R parameter Real a = 1; parameter Integer n = 2; end R;
         type Mode = enumeration(A, B);
         model Top
           parameter R r1 = R(a = 5, n = 3);
           parameter R r2 = r1;
           parameter Mode mode = Mode.B;
           parameter Integer k = if mode == Mode.B then 4 else 5;
           Real x[k]; Real y[r1.n];
           parameter Real t[:] = {1, 2}; parameter Real u[:, :] = [1, 2, 3; 4, 5, 6];
           parameter Real w[:] = 1:3;
         end Top;",
        "Top",
    );
    let v = |n: &str| m.vars.iter().find(|v| v.name == n).unwrap().clone();
    assert_eq!(v("r1.a").binding.as_deref(), Some("5"));
    assert_eq!(v("r2.a").binding.as_deref(), Some("r1.a"));
    assert_eq!(v("r2.n").binding.as_deref(), Some("r1.n"));
    assert_eq!(v("x").dims, [4]);
    assert_eq!(v("y").dims, [3]);
    assert_eq!(v("u").dims, [2, 3]);
    assert_eq!(v("w").dims, [3]);
}

const SRC: &str = "model Src parameter Real a = 1; Real y; equation y = a; end Src;
    model Src2 extends Src; parameter Real b = 3; end Src2;
    model Base replaceable Src s(a = 2); Real z; equation z = s.y; end Base;";

#[test]
fn redeclare_component() {
    // 既定の型（`replaceable` は印だけ）
    let m = flat(&format!("{SRC} model T Base b; end T;"), "T");
    assert_eq!(names(&m), ["b.s.a", "b.s.y", "b.z"]);
    // `extends B(redeclare T x(…))`: 型を差し替え、元の修飾（a = 2）は残り、新しい修飾（b = 7）が足される
    let m = flat(
        &format!("{SRC} model D extends Base(redeclare Src2 s(b = 7)); end D;"),
        "D",
    );
    assert_eq!(names(&m), ["s.a", "s.y", "s.b", "z"]);
    let v = |n: &str| m.vars.iter().find(|v| v.name == n).unwrap().binding.clone();
    assert_eq!((v("s.a"), v("s.b")), (Some("2".into()), Some("7".into())));
    assert_eq!(eqs(&m), ["s.y = s.a", "z = s.y"]);
    // 部品の修飾の中の redeclare と、short class の中の redeclare。外側が勝つ
    let m = flat(
        &format!(
            "{SRC} model D extends Base(redeclare Src2 s(b = 7)); end D; model Dd = D;
             model T D d1; Dd d2; D d3(redeclare Src s); Base b(redeclare Src2 s); end T;"
        ),
        "T",
    );
    let n = names(&m);
    assert!(n.contains(&"d1.s.b") && n.contains(&"d2.s.b") && n.contains(&"b.s.b"));
    assert!(!n.contains(&"d3.s.b"));
    // 型の別名（単位つき Real）への差し替え
    let m = flat(
        "type Volt = Real(unit = \"V\"); model R replaceable Real v; end R; model S R r(redeclare Volt v); end S;",
        "S",
    );
    assert_eq!(
        m.vars[0].attrs,
        vec![("unit".to_string(), "\"V\"".to_string())]
    );
}

#[test]
fn redeclare_package_and_class_extends() {
    let lib = "package PM replaceable model BP Real h; end BP; end PM;
        package P2 extends PM; redeclare model extends BP Real g; end BP; end P2;
        model Pipe replaceable package M = PM; M.BP bp; end Pipe;
        model Cyl replaceable model C = PM.BP; C c; end Cyl;";
    // 既定の package
    let m = flat(&format!("{lib} model T Pipe p; end T;"), "T");
    assert_eq!(names(&m), ["p.bp.h"]);
    // `redeclare package M = P2`（P2.BP は class extends で h と g を持つ）
    let m = flat(
        &format!("{lib} model T Pipe p(redeclare package M = P2); end T;"),
        "T",
    );
    assert_eq!(names(&m), ["p.bp.h", "p.bp.g"]);
    // 入れ子の部品へは、`redeclare package M = M` で運ぶ
    let m = flat(
        &format!(
            "{lib} model Out replaceable package M = PM; Pipe q(redeclare package M = M); end Out;
             model T Out o(redeclare package M = P2); end T;"
        ),
        "T",
    );
    assert_eq!(names(&m), ["o.q.bp.h", "o.q.bp.g"]);
    // `redeclare model C = X`、関数の差し替えは平らにしない
    let m = flat(
        &format!(
            "{lib} model T Cyl y(redeclare model C = P2.BP, redeclare function f = g); end T;"
        ),
        "T",
    );
    assert_eq!(names(&m), ["y.c.h", "y.c.g"]);
}

#[test]
fn redeclare_package_members_follow_the_heir() {
    // 受け継いだクラス BP の中の `TS` と `n` は、差し替え後のパッケージ P2 のもの（`redeclare record TS` と `extends PM(n = 2)`）
    let lib = "package PM constant Integer n = 1; replaceable record TS Real a; end TS;
          replaceable model BP TS state; Real x[n]; end BP; end PM;
        package P2 extends PM(n = 2); redeclare record TS Real a; Real b; end TS; end P2;
        model Pipe replaceable package M = PM; M.BP bp; Real y[M.n]; end Pipe;";
    let m = flat(
        &format!("{lib} model T Pipe p(redeclare package M = P2); end T;"),
        "T",
    );
    assert_eq!(names(&m), ["p.bp.state.a", "p.bp.state.b", "p.bp.x", "p.y"]);
    assert_eq!(m.vars[2].dims, vec![2]);
    assert_eq!(m.vars[3].dims, vec![2]);
    let m = flat(&format!("{lib} model T Pipe p; end T;"), "T");
    assert_eq!(names(&m), ["p.bp.state.a", "p.bp.x", "p.y"]);
    assert_eq!(m.vars[1].dims, vec![1]);
}

#[test]
fn array_type_and_type_extends_builtin() {
    // `type V3 = Real[3]` は使う側の添字の後ろに足す。長い形の `type T extends String;` は組み込みの型として扱う
    let m = flat(
        "type V3 = Real[3](each unit = \"1\"); type L extends String; end L; model A V3 a; V3 b[2]; parameter L s = \"x\"; end A;",
        "A",
    );
    assert_eq!(names(&m), ["a", "b", "s"]);
    assert_eq!(
        (
            m.vars[0].dims.clone(),
            m.vars[1].dims.clone(),
            m.vars[2].ty.as_str()
        ),
        (vec![3], vec![2, 3], "String")
    );
}

#[test]
fn alias_package_selects_heir_members() {
    // `package Medium = G` の別名経由で `Medium.BP` を使う（受け継いだ BP の中の `TS` は TB のもの）
    let lib = "partial package PM replaceable record TS end TS; replaceable model BP TS state; end BP; end PM;
        partial package TB extends PM; redeclare record extends TS Real T; end TS; redeclare model extends BP end BP; end TB;
        package G = TB;";
    let m = flat(
        &format!("{lib} model T package Medium = G; Medium.BP medium; end T;"),
        "T",
    );
    assert_eq!(names(&m), ["medium.state.T"]);
}

#[test]
fn package_extends_redeclares_record() {
    // `extends PM(redeclare record TS = R2)`（TableBased と同じ形）で決まった TS を、別名のパッケージ経由でも使う
    let lib = "record R2 Real T; Real p; end R2;
        partial package PM replaceable record TS end TS; replaceable model BP TS state; end BP; end PM;
        partial package TB extends PM(redeclare record TS = R2); end TB;
        package G = TB;";
    let m = flat(
        &format!("{lib} model T package Medium = G; Medium.BP medium; end T;"),
        "T",
    );
    assert_eq!(names(&m), ["medium.state.T", "medium.state.p"]);
}

/// S311: 変数・方程式が元のソースの位置（ファイル名とバイト範囲）を運ぶ
#[test]
fn origins_follow_nesting_extends_connect_and_for() {
    let src = "connector Pin Real v; flow Real i; end Pin;
model Base Real b; equation b = 1; end Base;
model Cell extends Base; Pin p; equation p.v = b; end Cell;
model Top Cell c[2]; Pin q; Real y[2];
equation for k in 1:2 loop y[k] = c[k].b; end for; connect(c[1].p, q); end Top;";
    let m = flat(src, "Top");
    let at = |o: &Option<globe_mode_flat::Origin>| {
        let o = o.as_ref().expect("origin");
        assert_eq!(&*o.file, "t.mo");
        src[o.start as usize..o.end as usize]
            .trim_end_matches(';')
            .to_string()
    };
    // 入れ子: c[2].b は Base の宣言（extends で受け継いだ宣言は基底のファイル内の位置）
    let var = |n: &str| m.vars.iter().find(|v| v.name == n).unwrap();
    assert_eq!(at(&var("c[2].b").origin), "b");
    assert_eq!(at(&var("y").origin), "y[2]");
    // extends の方程式（Base の `b = 1`）、入れ子の方程式、for（展開した 2 本とも中の方程式）
    let on = |text: &str| {
        m.equations
            .iter()
            .filter(|e| e.text == text)
            .map(|e| at(&e.origin))
            .collect::<Vec<_>>()
    };
    assert_eq!(on("c[1].b = 1"), ["b = 1"]);
    assert_eq!(on("c[2].p.v = c[2].b"), ["p.v = b"]);
    assert_eq!(on("y[2] = c[2].b"), ["y[k] = c[k].b"]);
    // connect から作った potential・flow の方程式は connect の文
    assert_eq!(on("c[1].p.v = q.v"), ["connect(c[1].p, q)"]);
    assert_eq!(on("c[1].p.i - q.i = 0"), ["connect(c[1].p, q)"]);
    // つなげなかったコネクタの flow = 0 は flow 変数の宣言
    assert_eq!(on("c[2].p.i = 0"), ["i"]); // コネクタの flow 変数の宣言
}

#[test]
fn huge_component_array_is_refused_before_allocating() {
    // #109: 添字の文字列を作る前に断る（積が MAX_VARS を超える・usize を溢れる）
    for dims in ["100000, 100000", "4294967296, 4294967296, 4294967296"] {
        let src = format!("model R Real x; equation x = 1; end R; model M R r[{dims}]; end M;");
        assert_eq!(err(&src, "M"), "too-large");
    }
}
