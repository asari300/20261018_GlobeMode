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

/// 宣言が後ろの `inner` に `outer` 経由で `connect` する（S361。`StateGraph` の `stateGraphRoot`）。
/// 接続集合は階層をまたいで 1 つ: flow の和は 1 本（部品ごとに作らない）、つないだ内側の flow は 0 にしない
#[test]
fn outer_connect_to_later_inner() {
    let m = flat(
        "connector P Real v; flow Real i; end P;
         model Root P p; end Root;
         model Port P p; end Port;
         model Step outer Root root; Port port; equation connect(port.p, root.p); end Step;
         model Top Step s1; Step s2; inner Root root; end Top;",
        "Top",
    );
    assert_eq!(
        eqs(&m),
        [
            "s1.port.p.i + root.p.i + s2.port.p.i = 0",
            "s1.port.p.v = root.p.v",
            "s1.port.p.v = s2.port.p.v"
        ]
    );
}

/// `inner outer` は実体を持ち、中の部品の `outer` はそれにつながる。同じ階層の `connect` の端だけ外側の `inner` につながる（S361。`StateGraph` の `CompositeStep`）
#[test]
fn inner_outer_is_instantiated_and_bridged() {
    let m = flat(
        "connector P Real v; flow Real i; end P;
         model Root parameter Real k = 1; P p; end Root;
         model Leaf outer Root root; P q; equation connect(q, root.p); end Leaf;
         model Comp inner outer Root root(k = 2); P q; Leaf l; equation connect(q, root.p); end Comp;
         model Top Comp c; inner Root root; end Top;",
        "Top",
    );
    let n = names(&m);
    assert!(n.contains(&"c.root.k") && n.contains(&"root.k"), "{n:?}");
    let e = eqs(&m);
    // 葉は `c.root`（実体）につながり、`c` 自身の `connect` は外側の `root`（Top）につながる。`l.q`・`c.q` は外側のコネクタなので符号が逆（持ち越しても変わらない）
    assert!(
        e.contains(&"-c.l.q.i + c.root.p.i = 0".to_string()),
        "{e:#?}"
    );
    assert!(e.contains(&"-c.q.i + root.p.i = 0".to_string()), "{e:#?}");
}

/// `cardinality(c)` は `c` が `connect` の端に出てきた数。全部の `connect` がそろってから `if` の枝を選ぶ（S361）
#[test]
fn cardinality_selects_if_branch() {
    let m = flat(
        "connector P Real v; flow Real i; end P;
         model A P p; P q[2];
         equation
           for k in 1:2 loop if cardinality(q[k]) == 0 then q[k].v = 0; end if; end for;
           if cardinality(p) < 1 then p.v = 1; else p.v = 2; end if;
         end A;
         model Top A a; A b; equation connect(a.q[1], b.q[2]); connect(a.p, b.p); end Top;",
        "Top",
    );
    let e = eqs(&m);
    // 未接続の `a.q[2]`・`b.q[1]` だけ `= 0`。`a.p`・`b.p` はつながっているので `else` の枝
    for x in ["a.q[2].v = 0", "b.q[1].v = 0", "a.p.v = 2", "b.p.v = 2"] {
        assert!(e.contains(&x.to_string()), "{x}: {e:#?}");
    }
    assert!(
        !e.iter()
            .any(|x| x.starts_with("if ") || x.contains("a.p.v = 1") || x.contains("a.q[1].v = 0")),
        "{e:#?}"
    );
}

/// 差し替えた部品にも、元の宣言の `constrainedby C(nu = nu)` の修飾が掛かる（S361。`Clocked` の `combinator`）。明示した修飾が勝つ
#[test]
fn redeclare_keeps_constraining_modification() {
    let src = |redecl: &str| {
        format!(
            "partial model PM parameter Integer nu = 0; Boolean u[nu]; end PM;
             model And extends PM; end And;
             partial model PLC parameter Integer nu = 2; replaceable PM c constrainedby PM(nu = nu); end PLC;
             model Conj extends PLC(redeclare And c{redecl}); end Conj;"
        )
    };
    let dims = |m: &FlatModel| {
        m.vars
            .iter()
            .find(|v| v.name == "c.u")
            .unwrap()
            .dims
            .clone()
    };
    assert_eq!(dims(&flat(&src(""), "Conj")), [2]);
    assert_eq!(dims(&flat(&src("(nu = 3)"), "Conj")), [3]);
}

/// 内包の中の `inStream` は添字ごとに展開する（S361。`MultiPort` の `sum({… inStream(ports_b[j].h_outflow) for j in 1:n})`）
#[test]
fn in_stream_in_comprehension() {
    let m = flat(
        "connector P Real v; flow Real i; stream Real h; end P;
         model N P p[2]; Real s; equation s = sum({inStream(p[k].h) for k in 1:2}); end N;
         model A N n; end A;",
        "A",
    );
    assert!(
        eqs(&m).contains(&"n.s = sum({n.p[1].h, n.p[2].h})".to_string()),
        "{:#?}",
        eqs(&m)
    );
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

#[test]
fn overconstrained_connections_drop_broken_edges_and_non_root_ifs() {
    // `Reference` は `equalityConstraint` を持つ（大きさ 0）。端子を輪にくくる（閉路）と、辺が 1 本切れて等式が 1 本減る。
    // 根は Source（確定）なので、Ground（潜在的な根）の `if isRoot` の等式は無い
    let src = "record Reference Real gamma;
          function equalityConstraint input Reference r1; input Reference r2; output Real residue[0]; algorithm end equalityConstraint;
        end Reference;
        connector Pin Real v; flow Real i; Reference reference; end Pin;
        model Two Pin p; Pin n; equation Connections.branch(p.reference, n.reference); p.v - n.v = 1; p.i + n.i = 0; end Two;
        model Src Pin p; equation Connections.root(p.reference); p.v = 1; end Src;
        model Gnd Pin p; equation Connections.potentialRoot(p.reference, 256);
          if Connections.isRoot(p.reference) then p.reference.gamma = 0; end if; p.v = 0; end Gnd;
        model Top Src s; Gnd g; Two a; Two b;
        equation connect(s.p, a.p); connect(a.n, b.p); connect(b.n, g.p); connect(g.p, s.p); end Top;";
    let m = flat(src, "Top");
    let e = eqs(&m);
    // 接続集合は 4 端子と 2 端子の 2 つ。gamma の等式は (4 - 1) + (2 - 1) = 4 本だが、閉路の辺が 1 本切れて 3 本
    let gammas = e.iter().filter(|t| t.contains("reference.gamma")).count();
    assert_eq!(gammas, 3, "{e:#?}");
    assert!(
        !e.iter().any(|t| t.starts_with("if Connections.isRoot")),
        "{e:#?}"
    );
}

#[test]
fn broken_edge_gets_equality_constraint_of_its_residue_size() {
    // 閉路の辺が 1 本切れると、`residue[n]` の大きさの `zeros(n) = equalityConstraint(a, b)` になる（n = 0 なら作らない。S334）
    let src = |n: usize| {
        format!(
            "record Reference Real gamma;
          function equalityConstraint input Reference r1; input Reference r2; output Real residue[{n}]; algorithm end equalityConstraint;
        end Reference;
        connector Pin Real v; flow Real i; Reference reference; end Pin;
        model Two Pin p; Pin n; equation Connections.branch(p.reference, n.reference); p.v - n.v = 1; p.i + n.i = 0; end Two;
        model Src Pin p; equation Connections.root(p.reference); p.v = 1; end Src;
        model Top Src s; Two a; Two b;
        equation connect(s.p, a.p); connect(a.n, b.p); connect(b.n, s.p); end Top;"
        )
    };
    let (m0, m3) = (flat(&src(0), "Top"), flat(&src(3), "Top"));
    assert_eq!(
        m3.equation_count(),
        m0.equation_count() + 3,
        "{:#?}",
        eqs(&m3)
    );
    assert!(
        eqs(&m3)
            .iter()
            .any(|t| t.starts_with("zeros(3) = Reference.equalityConstraint(")),
        "{:#?}",
        eqs(&m3)
    );
}

#[test]
fn user_function_with_array_output_gives_equation_size() {
    // 左辺が配列を返す関数の呼び出し（`Frames.angularVelocity2(R) = …`）は、出力の大きさの本数（S334）
    let src = "function w2 input Real x; output Real w[3]; algorithm w := {x, x, x}; end w2;
        model M Real a; Real b[3]; equation w2(a) = b + w2(a); end M;";
    assert_eq!(flat(src, "M").equation_count(), 3);
}

const OC: &str = "record Ref Real g;
      function equalityConstraint input Ref r1; input Ref r2; output Real residue[0]; algorithm end equalityConstraint;
    end Ref;
    connector Fr Ref r; Real x; flow Real f; end Fr;
    model World Fr b; equation Connections.root(b.r); b.x = 0; end World;
    model Body Fr a; Real q; Real w; parameter Boolean useQ = true;
      equation Connections.potentialRoot(a.r, 10);
      if not Connections.isRoot(a.r) then q = 0; w = 0;
      elseif useQ then der(q) = 1; w = q;
      else w = 2; end if;
      initial equation if not Connections.isRoot(a.r) then q = 5; else q = 6; end if;
    end Body;
    model Joint Fr a; Fr b; equation Connections.branch(a.r, b.r); a.f + b.f = 0;
      if Connections.rooted(a.r) then b.x = a.x + 1; else a.x = b.x - 1; end if;
    end Joint;";

#[test]
fn is_root_if_with_elseif_keeps_only_the_chosen_branch() {
    // 世界（確定した根）につなぐと Body は根でない: `q = 0; w = 0` だけで `der(q)` は無い
    let m = flat(
        &format!("{OC} model Top World world; Body b; equation connect(world.b, b.a); end Top;"),
        "Top",
    );
    let e = eqs(&m);
    assert!(
        e.contains(&"b.q = 0".to_string()) && !e.iter().any(|t| t.contains("der(b.q)")),
        "{e:#?}"
    );
    // `initial equation` の `if` も同じに選ぶ
    let ie: Vec<_> = m
        .initial_equations
        .iter()
        .map(|e| e.text.as_str())
        .collect();
    assert_eq!(ie, ["b.q = 5"]);
    // 単独の Body は根（接続が無いので Rumoca の既定と同じ）: `elseif useQ` の枝
    let m = flat(&format!("{OC} model Top Body b; end Top;"), "Top");
    let e = eqs(&m);
    assert!(
        e.contains(&"der(b.q) = 1".to_string()) && !e.contains(&"b.q = 0".to_string()),
        "{e:#?}"
    );
    assert_eq!(m.initial_equations[0].text, "b.q = 6");
}

#[test]
fn rooted_follows_the_branch_direction_in_the_spanning_tree() {
    // 世界 - j.a ... j.b - body: `rooted(j.a.r)`（a が根に近い）は真、向きを逆につなぐと偽
    let top =
        |c: &str| format!("{OC} model Top World world; Joint j; Body b; equation {c} end Top;");
    let e = eqs(&flat(
        &top("connect(world.b, j.a); connect(j.b, b.a);"),
        "Top",
    ));
    assert!(e.iter().any(|t| t.starts_with("j.b.x = j.a.x")), "{e:#?}");
    assert!(!e.iter().any(|t| t.starts_with("j.a.x = j.b.x")), "{e:#?}");
    let e = eqs(&flat(
        &top("connect(world.b, j.b); connect(j.a, b.a);"),
        "Top",
    ));
    assert!(e.iter().any(|t| t.starts_with("j.a.x = j.b.x")), "{e:#?}");
    assert!(!e.iter().any(|t| t.starts_with("j.b.x = j.a.x")), "{e:#?}");
}

#[test]
fn potential_root_with_lower_priority_number_becomes_the_root() {
    let src = "record Ref Real g;
          function equalityConstraint input Ref r1; input Ref r2; output Real residue[0]; algorithm end equalityConstraint;
        end Ref;
        connector Fr Ref r; Real x; flow Real f; end Fr;
        model P1 Fr a; equation Connections.potentialRoot(a.r, 5); if Connections.isRoot(a.r) then a.x = 1; end if; end P1;
        model P2 Fr a; equation Connections.potentialRoot(a.r, 3); if Connections.isRoot(a.r) then a.x = 2; end if; end P2;
        model Top P1 p; P2 q; equation connect(p.a, q.a); end Top;";
    let e = eqs(&flat(src, "Top"));
    assert!(
        e.contains(&"q.a.x = 2".to_string()) && !e.contains(&"p.a.x = 1".to_string()),
        "{e:#?}"
    );
}

#[test]
fn connection_sets_are_merged_by_leaf_variable() {
    // `connect(x.p, q.a)` と、丸ごとの `connect(q, w)` は、葉の変数 `q.a.i` で同じ集合になる（flow の和は 1 本）
    let src = "connector Pin Real v; flow Real i; end Pin;
        connector Port Pin a; end Port;
        model Two Pin p; Pin n; equation p.v - n.v = 1; p.i + n.i = 0; end Two;
        model Top Two x; Port q; Port w; protected Pin hidden; equation connect(x.p, q.a); connect(q, w); end Top;";
    let e = eqs(&flat(src, "Top"));
    let sums: Vec<_> = e.iter().filter(|t| t.contains("q.a.i")).collect();
    assert_eq!(sums.len(), 2, "{e:#?}"); // 3 端子の flow の和 + 外側のコネクタ q の flow = 0
    assert!(
        e.iter()
            .any(|t| t.contains("x.p.i") && t.contains("q.a.i") && t.contains("w.a.i")),
        "{e:#?}"
    );
    // `protected` で、どこにもつながっていないコネクタの flow は 0
    assert!(e.contains(&"hidden.i = 0".to_string()), "{e:#?}");
}

#[test]
fn product_shapes_count_scalars() {
    // ベクトルの内積は 1 本、行列 × ベクトルは行の数、`.*` は要素ごと
    let src = "model M Real u[3]; Real v[3]; Real A[2,3]; Real y[2];
        equation u * v = 0; A * v = y; u .* v = {1, 2, 3}; end M;";
    let m = flat(src, "M");
    assert_eq!(m.equation_count(), 1 + 2 + 3, "{:#?}", eqs(&m));
}

#[test]
fn matrix_of_vectors_and_tuple_of_arrays_count_scalars() {
    let src = "function f input Real x; output Real a; output Real b[2]; algorithm a := x; b := {x, x}; end f;
        model M Real y[3]; Real u1[1]; Real u2[2]; Real r; Real s[2];
        equation [y] = [u1; u2]; (r, s) = f(1); u1 = {1}; u2 = {1, 2}; end M;";
    let m = flat(src, "M");
    assert_eq!(m.equation_count(), 3 + 3 + 1 + 2, "{:#?}", eqs(&m));
}

#[test]
fn if_on_parameter_and_package_constant_picks_branch() {
    let src = "package Svc package Machine final constant Real small = 2.2250738585072014e-308; end Machine; end Svc;
        package K final constant Real small = Svc.Machine.small; end K;
        model P parameter Real m = 0; Real T; Real x;
          equation if m > K.small then x = m * der(T); else x = 0; end if; T = time; end P;
        model Top P p(m = 0); P q(m = 2); end Top;";
    let e = eqs(&flat(src, "Top"));
    assert!(e.contains(&"p.x = 0".to_string()), "{e:#?}");
    assert!(e.contains(&"q.x = q.m * der(q.T)".to_string()), "{e:#?}");
}

#[test]
fn if_expression_on_parameter_keeps_only_the_taken_branch() {
    let src = "model P parameter Boolean use = false; Real x; Real y; Real z;
          equation y = if use then -der(x) else 0; z = if time > 1 then 1 else 2; x = time; end P;";
    let e = eqs(&flat(src, "P"));
    assert!(e.contains(&"y = 0".to_string()), "{e:#?}");
    // 実行時の条件はそのまま
    assert!(
        e.contains(&"z = if time > 1 then 1 else 2".to_string()),
        "{e:#?}"
    );
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

/// #111: 自分を部品に持つモデルは、スタックを溢れさせず「深すぎる」で断る
#[test]
fn self_containing_model_is_refused_not_overflowed() {
    assert_eq!(err("model C C a; end C;", "C"), "deep-instantiation");
    // `redeclare` で自分に戻す循環
    assert_eq!(
        err(
            "model C replaceable model M = D; M m; end C;
             model D replaceable model M = D; M m; end D;
             model E C c(redeclare model M = C); end E;",
            "E"
        ),
        "deep-instantiation"
    );
}

#[test]
fn record_bound_to_later_sibling_and_function_value() {
    // S324: 宣言が後ろにある兄弟のレコード、レコードを返す関数、配列のレコード、外部オブジェクト
    let m = flat(
        "record P parameter Real a = 1; Real b[2]; end P;
         model H P p; end H;
         function mk output P y; algorithm y.a := 2; end mk;
         class Tab extends ExternalObject; function constructor input Real x; output Tab t; external \"C\"; end constructor;
           function destructor input Tab t; external \"C\"; end destructor; end Tab;
         model Top
           H q(p = d.p);
           P f = mk();
           P arr[2] = {P(a = 3), P(a = 4)};
           P src2[2]; P arr2[2] = src2;
           parameter Tab t = Tab(3);
           H d;
         end Top;",
        "Top",
    );
    let b = |n: &str| {
        m.vars
            .iter()
            .find(|v| v.name == n)
            .unwrap_or_else(|| panic!("{n}"))
            .binding
            .clone()
    };
    // `q.p = d.p` は宣言が後ろの `d` を、型の欄ごとに束縛する
    assert_eq!(b("q.p.a").as_deref(), Some("d.p.a"));
    assert_eq!(b("q.p.b").as_deref(), Some("d.p.b"));
    // 関数の戻り値のレコードは評価せず、欄ごとに `f().a`
    assert_eq!(b("f.a").as_deref(), Some("mk().a"));
    assert_eq!(b("f.b").as_deref(), Some("mk().b"));
    // 配列のレコード: リテラルの要素ごと、配列の変数は `x[k].f`
    assert_eq!(b("arr[2].a").as_deref(), Some("4"));
    assert_eq!(b("arr2[1].a").as_deref(), Some("src2[1].a"));
    // 外部オブジェクトは 1 つの変数として残す
    let t = m.vars.iter().find(|v| v.name == "t").unwrap();
    assert_eq!(t.binding.as_deref(), Some("Tab(3)"));
    assert!(m.vars.iter().all(|v| !v.name.starts_with("t.")));
}

#[test]
fn connect_whole_arrays() {
    // S324: 部品の配列の同名のコネクタ・配列のコネクタを、要素ごとの接続にする
    let m = flat(
        "connector Pin Real v; flow Real i; end Pin;
         model R Pin p; Pin n; equation p.v - n.v = p.i; p.i + n.i = 0; end R;
         model Multi parameter Integer m = 2; Pin pin[m]; end Multi;
         model Rs parameter Integer m = 2; Multi pp(m = m); Multi pn(m = m); R r[m];
           equation connect(pp.pin, r.p); connect(r.n, pn.pin); end Rs;
         model Top Rs s; end Top;",
        "Top",
    );
    let e = eqs(&m).join("\n");
    // 要素ごとの電位の等式と flow の和（`s.r[1].p` と `s.pp.pin[1]` など）
    assert!(e.contains("s.r[1].p.i"), "{e}");
    assert!(e.contains("s.pp.pin[2].v"), "{e}");
    assert!(!e.contains("s.pp.pin.v"), "{e}");
}

#[test]
fn modification_of_component_array_is_split_per_element() {
    // S329: `c[n](a = {…})` は要素ごとの修飾、`each` は分けない、`for` の内包は `(…)[k]`
    let m = flat(
        "model S parameter Real len = 0; parameter Real w = 0; parameter Real r[3] = {0,0,0};
           parameter Boolean u = false; Real h if u; end S;
         model Top
           parameter Integer n = 2;
           S c[n](len = {5, 6}, each w = 7, u = fill(true, n), r = {{1,2,3} for i in 1:n});
           S d[2, 2](len = {{1, 2}, {3, 4}});
         end Top;",
        "Top",
    );
    let b = |n: &str| {
        m.vars
            .iter()
            .find(|v| v.name == n)
            .unwrap_or_else(|| panic!("{n}"))
            .binding
            .clone()
    };
    assert_eq!(b("c[2].len").as_deref(), Some("6"));
    assert_eq!(b("c[1].w").as_deref(), Some("7"));
    // `fill(true, n)` は要素ごとに `true`: 条件つきの `h` が残る
    assert_eq!(b("c[2].u").as_deref(), Some("true"));
    assert!(m.vars.iter().any(|v| v.name == "c[2].h"));
    assert!(m.vars.iter().all(|v| v.name != "d[1,1].h"));
    assert!(b("c[2].r").unwrap().ends_with(")[2]"));
    assert!(b("d[2,1].len").unwrap().ends_with("[2,1]"));
    // 項の数が合わない `{…}` は断る
    assert_eq!(
        err(
            "model S parameter Real a = 0; end S; model T S c[3](a = {1, 2}); end T;",
            "T"
        ),
        "array-modification"
    );
}

#[test]
fn complex_operators_expand_to_re_and_im() {
    // S329: `Complex` の `+ - * /` は評価せず、欄 `re`・`im` ごとの式にする
    let m = flat(
        "operator record Complex Real re; Real im; end Complex;
         connector P Complex V; end P;
         model Top
           parameter Boolean f = true;
           P p; P n;
           Complex d = p.V - n.V;
           Complex e = 2*d;
           Complex g = (if f then d else -d)/(e + 1);
           Complex h = Complex(1, 2)*d;
           Complex k[2] = {d, e};
           Complex m = k[2];
         end Top;",
        "Top",
    );
    let b = |n: &str| {
        m.vars
            .iter()
            .find(|v| v.name == n)
            .unwrap_or_else(|| panic!("{n}"))
            .binding
            .clone()
    };
    assert_eq!(b("d.re").as_deref(), Some("p.V.re - n.V.re"));
    assert_eq!(b("d.im").as_deref(), Some("p.V.im - n.V.im"));
    assert_eq!(b("e.re").as_deref(), Some("2*d.re"));
    assert_eq!(b("e.im").as_deref(), Some("2*d.im"));
    assert_eq!(b("h.re").as_deref(), Some("1*d.re - 2*d.im"));
    assert!(b("g.re").unwrap().contains("if f then d.re else -d.re"));
    assert_eq!(b("m.re").as_deref(), Some("k[2].re"));
}

#[test]
fn conditions_on_strings_and_later_inner_fields_and_record_if() {
    // S329: 文字列の比較、宣言が後ろの `inner` の欄、定数に決まる `if` の式（レコード）
    let m = flat(
        "record P Real a = 0; end P;
         model W parameter Boolean on = true; end W;
         model Sub outer W w; parameter Boolean an = true; parameter String c = \"Y\";
           Real s if w.on and an; Real t if c <> \"D\"; Real u if c == \"D\"; end Sub;
         model Top parameter Boolean sel = true; P p = if sel then P(a = 1) else P(a = 2);
           Sub sub; inner W w(on = true); end Top;",
        "Top",
    );
    let has = |n: &str| m.vars.iter().any(|v| v.name == n);
    assert!(has("sub.s") && has("sub.t") && !has("sub.u"));
    assert_eq!(
        m.vars
            .iter()
            .find(|v| v.name == "p.a")
            .unwrap()
            .binding
            .as_deref(),
        Some("1")
    );
}

#[test]
fn unsized_dims_from_comprehension_function_args_and_record_fields() {
    // `[:]` の大きさを束縛の式から決める（S335）: 添字つきの参照・内包・`vector`・関数の整数の引数（既定値も）・
    // 宣言が後ろの部品の欄（`rr.t`）・レコードの束縛 `r = rr` の欄
    let src = "function mk input Integer n = 4; output Real v[n]; algorithm for i in 1:n loop v[i] := i; end for; end mk;
        record R parameter Real t[:] = {1, 2, 3}; parameter Integer k = 2; parameter Real u[k]; end R;
        model Q parameter R r; end Q;
        model M parameter Real table[:] = {0, 1, 2};
          parameter Integer n = size(table, 1);
          parameter Real a[:, :] = if n > 0 then [table[1], 1.0; table, {mod(i + 1, 2.0) for i in 1:n}] else [0.0, 1.0];
          parameter Real bb[:] = vector([zeros(max(0, 5 - n), 1); table]);
          parameter Real f[:] = mk(3);
          parameter Real g[:] = mk();
          Q q(r = rr);
          R rr(t = {1, 2, 3, 4});
        end M;";
    let m = flat(src, "M");
    let dims = |n: &str| m.vars.iter().find(|v| v.name == n).unwrap().dims.clone();
    assert_eq!(dims("a"), [4, 2]);
    assert_eq!(dims("bb"), [5]);
    assert_eq!((dims("f"), dims("g")), (vec![3], vec![4]));
    assert_eq!(dims("q.r.t"), [4]);
    // 宣言の場所で修飾された欄を使う大きさ（`u[k]` の `k`）は、宣言が後ろの部品では決めない
    let src = "record R parameter Integer k = 2; parameter Real u[:] = fill(0.0, k); end R;
        model Q parameter R r; end Q;
        model M Q q(r = rr); R rr(k = 3); end M;";
    assert_eq!(err(src, "M"), "unsized-array");
}

#[test]
fn record_arrays_bound_through_record_fields_and_2d_connector_arrays() {
    // S367: `r = other` の欄がレコードの配列（`rc[n] = d.rc`）、`anyTrue`（S370 から関数を実行する。MSL と同じ本体）と parameter の行列 `k[i, 1]` で選ぶ `if`、2 次元の部品の配列の `connect`
    let m = flat(
        "connector P Real v; flow Real i; end P;
         model Res P p; P n; parameter Real R = 1; equation p.v - n.v = R * p.i; p.i + n.i = 0; end Res;
         record RC parameter Real R = 1; end RC;
         record D parameter Integer n = 2; parameter RC rc[n] = {RC(R = 2), RC(R = 3)}; end D;
         model In D d; RC rc[d.n] = d.rc; end In;
         package Modelica package Math package BooleanVectors
           function anyTrue input Boolean b[:]; output Boolean r = size(b, 1) > 0 and max(b); algorithm end anyTrue;
         end BooleanVectors; end Math; end Modelica;
         model Sl Res r[3]; P q[3]; equation connect(r[3:-1:2].p, q[1:2]); end Sl;
         model Top import Modelica.Math.BooleanVectors.anyTrue; D d; In c(d = d); Sl sl;
           parameter Integer k[:, 2] = [1, 2];
           RC pick[1, 2] = {{if anyTrue({a == k[i, 1] and b == k[i, 2] for i in 1:size(k, 1)}) then RC(R = 7) else RC(R = 8) for b in 1:2} for a in 1:1};
           Res r[2, 2]; P q[2, 2];
         equation connect(r.p, q); end Top;",
        "Top",
    );
    let b = |n: &str| m.vars.iter().find(|v| v.name == n).unwrap().binding.clone();
    assert_eq!(b("c.rc[2].R").as_deref(), Some("c.d.rc[2].R"));
    assert_eq!(b("pick[1,1].R").as_deref(), Some("8"));
    assert_eq!(b("pick[1,2].R").as_deref(), Some("7"));
    // 部品の 2 次元の配列 `r.p` と `q[2, 2]` が 4 要素ずつ対応する（potential 4 本）
    assert_eq!(
        eqs(&m).iter().filter(|e| e.ends_with("q[1,1].v")).count(),
        1
    );
    // 範囲の切り出し `r[3:-1:2].p` と `q[1:2]` は要素ごとに（r[3]-q[1]、r[2]-q[2]）
    assert!(eqs(&m).contains(&"sl.r[3].p.v = sl.q[1].v".to_string()));
    assert!(eqs(&m).contains(&"sl.r[2].p.v = sl.q[2].v".to_string()));
}

#[test]
fn complex_arrays_expand_elementwise() {
    // S336: `Complex` の配列の束縛は要素ごとに。内包 `{… for k in 1:n}`、部品の配列を通した欄の差、関数の戻り値の行列
    let m = flat(
        "operator record Complex Real re; Real im; end Complex;
         connector Pin Complex v; Complex i; end Pin;
         connector Plug parameter Integer m = 2; Pin pin[m]; end Plug;
         function mat input Integer m; output Complex y[m, m]; algorithm end mat;
         record R parameter Real d = 1; end R;
         model E R r; end E;
         model Top
           parameter Integer n = 2;
           parameter Boolean c[n] = {true, false};
           Plug p; Plug q;
           Complex u[n];
           Complex w[n] = {if c[k] then u[k] else -u[k] for k in 1:n};
           Complex v[n] = p.pin.v - q.pin.v;
           Complex i[n] = p.pin.i;
           parameter Complex s[n, n] = mat(n);
           parameter Complex k[n] = fill(Complex(1, 0), n);
           E e[n](r = {R(d = 2*k) for k in 1:n});
         end Top;",
        "Top",
    );
    let b = |n: &str| {
        m.vars
            .iter()
            .find(|v| v.name == n)
            .unwrap_or_else(|| panic!("{n}"))
            .binding
            .clone()
    };
    // 条件 `c[k]` は parameter の配列の要素で、評価できるので選んだ枝だけを印字する（スカラーの parameter と同じ。S367）
    assert_eq!(b("w[1].re").as_deref(), Some("u[1].re"));
    assert_eq!(b("w[2].re").as_deref(), Some("-u[2].re"));
    assert_eq!(
        b("v[2].im").as_deref(),
        Some("p.pin[2].v.im - q.pin[2].v.im")
    );
    assert_eq!(b("i[2].im").as_deref(), Some("p.pin[2].i.im"));
    assert_eq!(b("k[2].re").as_deref(), Some("1"));
    assert_eq!(b("s[2,1].re").as_deref(), Some("(mat(n))[2,1].re"));
    assert_eq!(b("e[2].r.d").as_deref(), Some("2 * 2"));
}

#[test]
fn comprehension_equation_counts_its_range() {
    // S336: 左辺が式の `a + b = {… for k in 1:n}` は n 本（Rumoca の flat と同じ。これまでは項が 1 つと見て 1 本）
    let m = flat(
        "model Top
           parameter Integer n = 3;
           Real a[n]; Real b[n];
         equation
           a + b = {2*k for k in 1:n};
         end Top;",
        "Top",
    );
    assert_eq!(m.equations.iter().map(|e| e.count).sum::<usize>(), 3);
}

#[test]
fn record_array_equation_counts_one_per_element() {
    // S337: `Complex aw[3]` は `aw[k].re`・`aw[k].im` に展開される。`aw = {… for …}` は Rumoca と同じく 3 本（左辺が変数でない）
    let m = flat(
        "operator record Complex Real re; Real im; end Complex;
         model Top
           Complex aw[3];
           Complex s;
         equation
           aw = {Complex(i, 0) for i in 1:3};
           s = aw[1];
         end Top;",
        "Top",
    );
    assert_eq!(m.equation_count(), 4);
}

// ---- S339: 5 回目のレビューの issue（#121〜#127・#130）

/// 時間を計って返す（計算量の試験。余裕を見た上限で、増え方が変わったことだけを見る）
fn timed<T>(f: impl FnOnce() -> T) -> (T, std::time::Duration) {
    let t = std::time::Instant::now();
    let v = f();
    let d = t.elapsed();
    eprintln!("timed: {d:?}");
    (v, d)
}

#[test]
fn complex_product_chain_is_refused_not_exponential() {
    // #121: `*` は両方の被演算子の実部・虚部を実部にも虚部にも使うので、a を 1 個足すごとに式が約 2 倍になる。
    // 式の長さが上限を超えたら `composite-binding` で断る（a が 40 個で 2^40 文字になる前に）
    let cx = "operator record Complex Real re; Real im; end Complex;";
    let src = |n: usize| {
        let z = vec!["a"; n].join("*");
        format!("{cx} model A Complex a(re = 1, im = 2); Complex z = {z}; end A;")
    };
    let (e, t) = timed(|| err(&src(40), "A"));
    assert_eq!(e, "composite-binding");
    assert!(t.as_secs() < 20, "{t:?}");
    // 小さい式は今までどおり欄ごとに展開する
    let m = flat(&src(4), "A");
    assert!(
        m.vars
            .iter()
            .any(|v| v.name == "z.re" && v.binding.is_some())
    );
}

#[test]
fn initial_if_is_root_twice_keeps_other_initial_equations() {
    // #122: `initial equation` の `if Connections.isRoot` が 2 つ以上あり、先の枝の方程式の数が 1 でないと、後の位置がずれて別の方程式を上書きした
    let src = "record Ref Real g;
          function equalityConstraint input Ref r1; input Ref r2; output Real residue[0]; algorithm end equalityConstraint;
        end Ref;
        connector Fr Ref r; Real x; flow Real f; end Fr;
        model World Fr b; equation Connections.root(b.r); b.x = 0; end World;
        model Body Fr a; Real q;
          equation Connections.potentialRoot(a.r, 10); a.f = 0; der(q) = 1;
          initial equation if Connections.isRoot(a.r) then q = 6; end if;
        end Body;
        model Top World world; Body b1; Body b2; Real z;
          equation connect(world.b, b1.a); z = 1;
          initial equation z = 3;
        end Top;";
    let m = flat(src, "Top");
    let ie: Vec<_> = m
        .initial_equations
        .iter()
        .map(|e| e.text.as_str())
        .collect();
    assert_eq!(ie, ["b2.q = 6", "z = 3"]);
}

#[test]
fn user_record_named_complex_is_not_expanded() {
    // #123: 名前だけが `Complex` で欄が `re`・`im` でないレコードは、演算子の展開に入らず断る（束縛が黙って落ちていた）
    let src = "record Complex Real a; Real b; end Complex;
        model M Complex u(a = 1, b = 2); Complex v(a = 3, b = 4); Boolean c(start = true);
          Complex z = if c then u else v;
        equation c = time > 0.5;
        end M;";
    assert_eq!(err(src, "M"), "composite-binding");
}

#[test]
fn array_component_modification_is_linear() {
    // #124: 部品の配列への `{…}` の修飾が、要素ごとに項の全部を作り直して O(n²) だった
    let n = 6000;
    let items: Vec<String> = (0..n).map(|i| i.to_string()).collect();
    let src = format!(
        "model N parameter Real v; end N; model M N n[{n}](v = {{{}}}); end M;",
        items.join(", ")
    );
    let (m, t) = timed(|| flat(&src, "M"));
    let v = m.vars.iter().find(|v| v.name == "n[6000].v").unwrap();
    assert_eq!(v.binding.as_deref(), Some("5999"));
    assert!(t.as_secs() < 8, "{t:?}");
}

#[test]
fn is_root_on_one_big_component_is_not_cubic() {
    // #125: `isRoot` の判定が呼ぶたびに成分の全部を走査し、1 つの成分に N 個の部品があると O(N³) だった
    let n = 1200;
    let src = format!(
        "record Ref Real g;
           function equalityConstraint input Ref r1; input Ref r2; output Real residue[0]; algorithm end equalityConstraint;
         end Ref;
         connector Fr Ref r; Real x; flow Real f; end Fr;
         model Body Fr a; Real q;
           equation Connections.potentialRoot(a.r, 10);
           if Connections.isRoot(a.r) then q = 1; else q = 0; end if;
         end Body;
         model Top Body b[{n}]; equation for i in 1:{} loop connect(b[i].a, b[i+1].a); end for; end Top;",
        n - 1
    );
    let (m, t) = timed(|| flat(&src, "Top"));
    // 根は 1 つだけ（その部品の `q = 1`）
    let ones = m
        .equations
        .iter()
        .filter(|e| e.text.ends_with(".q = 1"))
        .count();
    assert_eq!(ones, 1);
    assert!(t.as_secs() < 10, "{t:?}");
}

#[test]
fn array_connector_connect_is_not_quadratic() {
    // #127: `connect(p, q)`（配列のコネクタ 1 行）の葉の探索が線形で O(N²) だった
    let n = 6000;
    let src = format!(
        "connector P Real v; flow Real i; end P;
         model M P p[{n}]; P q[{n}]; equation connect(p, q); end M;"
    );
    let (m, t) = timed(|| flat(&src, "M"));
    assert!(
        m.equations
            .iter()
            .any(|e| e.text == "p[6000].v = q[6000].v")
    );
    // 直す前は N = 3000 で 6.7 s、2 倍で 4 倍（N = 6000 は 25 s 前後と見積もった）。直したあとは配列の展開そのものが残る（3 s 前後）
    assert!(t.as_secs() < 12, "{t:?}");
}

#[test]
fn huge_output_dims_do_not_overflow_the_equation_count() {
    // #130: 関数の戻り値の次元の積（`by_dims`・`equalityConstraint` の戻り値）を検査なしで取り、debug は panic・release は巻き戻った
    let src = "function f input Real x; output Real y[10000000000, 10000000000]; algorithm y := zeros(1,1); end f;
        function g input Real x; output Real y[10000000000, 10000000000]; algorithm y := zeros(1,1); end g;
        model M Real a; equation f(a) = g(a); end M;";
    let m = flat(src, "M");
    assert_eq!(m.equations.len(), 1);
    // `equalityConstraint` の戻り値が溢れる大きさでも、過剰決定の節点は 0 本（作らない）として扱う
    let oc = "record R Real g;
          function equalityConstraint input R r1; input R r2; output Real residue[10000000000, 10000000000]; algorithm end equalityConstraint;
        end R;
        model M R r[2]; end M;";
    flat(oc, "M");
}

#[test]
fn stream_sets_and_in_stream() {
    // 2 本の接続は相手の値、3 本は流量の重みの混合、つながないコネクタは自分。外側のコネクタの端には `x = 集合の混合` が 1 本（S355）
    let conn = "connector P Real v; flow Real i; stream Real h; end P;";
    let m = flat(
        &format!(
            "{conn} model N P p; Real s = inStream(p.h); end N;
             model W P a; P b; N n1; N n2; N n3; equation connect(a, n1.p); connect(b, n2.p); connect(a, n3.p); end W;
             model A W w; N n; N u; equation connect(w.a, n.p); connect(w.b, u.p); end A;"
        ),
        "A",
    );
    let eq = |lhs: &str| -> String {
        let e: Vec<_> = m
            .equations
            .iter()
            .filter(|e| e.text.starts_with(lhs))
            .collect();
        assert_eq!(e.len(), 1, "{lhs}: {e:?}");
        e[0].text.clone()
    };
    let bind = |n: &str| -> String {
        m.vars
            .iter()
            .find(|v| v.name == n)
            .unwrap()
            .binding
            .clone()
            .unwrap()
    };
    // w の外側の端 a は n1.p・n3.p（内側）と同じ集合。a.h = 内側 2 つの混合
    assert_eq!(
        eq("w.a.h ="),
        "w.a.h = ((max(-w.n1.p.i, 1e-7) * w.n1.p.h + max(-w.n3.p.i, 1e-7) * w.n3.p.h) / (max(-w.n1.p.i, 1e-7) + max(-w.n3.p.i, 1e-7)))"
    );
    // n.p は A の集合 {w.a（内側）, n.p（内側）}。n の inStream は w.a.h、w.b と u.p も同じ形
    assert_eq!(bind("n.s"), "w.a.h");
    assert_eq!(bind("u.s"), "w.b.h");
    // w.n1.p の inStream は w の集合 {w.a（外側）, w.n1.p, w.n3.p（内側）} から w.n1.p を除いたもの: 外側の a は 1 段上の inStream(w.a.h)
    assert_eq!(
        bind("w.n1.s"),
        "((max(w.a.i, 1e-7) * n.p.h + max(-w.n3.p.i, 1e-7) * w.n3.p.h) / (max(w.a.i, 1e-7) + max(-w.n3.p.i, 1e-7)))"
    );
    // 方程式の数: 外側の端の stream は 1 本ずつ（w.a・w.b）
    assert_eq!(eq("w.b.h =").matches("w.n2.p.h").count(), 1);
    // つながない内側の端は自分
    let m = flat(
        &format!("{conn} model N P p; Real s = inStream(p.h); end N; model A N n; end A;"),
        "A",
    );
    let s = m.vars.iter().find(|v| v.name == "n.s").unwrap();
    assert_eq!(s.binding.as_deref(), Some("n.p.h"));
}

/// `.history/` のような隠しフォルダの `.mo` は読まない（S358。同名のクラスが二重にならない）
#[test]
fn read_dirs_skips_hidden_dirs() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("flat_hidden_dirs");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join(".history")).unwrap();
    std::fs::write(dir.join("A.mo"), "model A Real x; end A;").unwrap();
    std::fs::write(dir.join(".history/A.mo"), "model A Real x; end A;").unwrap();
    let files = Project::read_dirs(&[dir.as_path()]).unwrap();
    assert_eq!(files.len(), 1, "{files:?}");
}

#[test]
fn msl_function_bodies_decide_array_sizes() {
    // S370: S367 は `anyTrue`・`isPowerOf2`・`numberOfSymmetricBaseSystems` を名前で決め打っていた。MSL 4.1.0 の本体（注釈と説明を除く）を実行して同じ値になる。
    // `realFFTsamplePoints`（FFT の 4 本の `ns`）は `while`・`break`・`assert`・名前つきの引数・既定値の入力を使う
    let src = |body: &str| {
        format!(
            "type Frequency = Real(unit = \"Hz\");
            function isPowerOf2 input Integer i(min=1); output Boolean result; protected Integer target; Integer powOf2; algorithm
              assert(i>=1, \"Integer input to isPowerOf2 has to be >= 1\");
              if mod(i, 2) == 1 then result := i == 1;
              else target := div(i, 2); powOf2 := 1;
                while powOf2 < target loop powOf2 := powOf2 * 2; end while;
                result := target == powOf2;
              end if;
            end isPowerOf2;
            function numberOfSymmetricBaseSystems input Integer m=3; output Integer n; algorithm
              n := 1;
              if mod(m, 2) == 0 then
                if m == 2 then n := 1; else n := n*2*numberOfSymmetricBaseSystems(integer(m/2)); end if;
              else n := 1; end if;
            end numberOfSymmetricBaseSystems;
            function realFFTsamplePoints
              input Frequency f_max; input Frequency f_resolution; input Integer f_max_factor(min=1)=5; output Integer ns;
            protected Integer ns1;
            algorithm
              assert(f_resolution > 0, \"f_resolution > 0 required\");
              assert(f_max > f_resolution, \"f_max > f_resolution required\");
              ns1 :=2*integer(ceil(f_max*f_max_factor/f_resolution));
              ns :=if mod(ns1, 2) == 0 then ns1 else ns1 + 1;
              while true loop
                ns1 :=ns;
                while mod(ns1,2) == 0 loop ns1 :=div(ns1, 2);end while;
                while mod(ns1,3) == 0 loop ns1 :=div(ns1, 3);end while;
                while mod(ns1,5) == 0 loop ns1 :=div(ns1, 5);end while;
                if ns1 <= 1 then break; end if;
                ns :=ns + 2;
              end while;
            end realFFTsamplePoints;
            model M {body} end M;"
        )
    };
    let dims = |body: &str| {
        let m = Project::from_sources([("t.mo", src(body).as_str())])
            .flatten("M")
            .unwrap();
        m.vars.iter().map(|v| v.dims[0]).collect::<Vec<_>>()
    };
    assert_eq!(
        dims(
            "Real a[if isPowerOf2(16) then 1 else 2]; Real b[if isPowerOf2(12) then 1 else 2]; Real c[if isPowerOf2(1) then 1 else 2];"
        ),
        [1, 2, 1]
    );
    assert_eq!(
        dims(
            "Real a[numberOfSymmetricBaseSystems(3)]; Real b[numberOfSymmetricBaseSystems(6)]; Real c[numberOfSymmetricBaseSystems(12)]; Real d[numberOfSymmetricBaseSystems(16)]; Real e[numberOfSymmetricBaseSystems()];"
        ),
        [1, 2, 4, 8, 1]
    );
    assert_eq!(
        dims(
            "Real a[realFFTsamplePoints(50, 2.5, f_max_factor=5)]; Real b[realFFTsamplePoints(21, 2)]; Real c[realFFTsamplePoints(7, 2, 5)];"
        ),
        [200, 108, 36]
    );
}
