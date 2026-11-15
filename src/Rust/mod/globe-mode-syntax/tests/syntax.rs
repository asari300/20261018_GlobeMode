use globe_mode_syntax::ast;
use globe_mode_syntax::{SyntaxKind, SyntaxKind::*, SyntaxNode, lex, parse};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

fn mo_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            mo_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "mo") {
            out.push(p);
        }
    }
}

fn kinds(n: &SyntaxNode, k: SyntaxKind) -> usize {
    n.descendants().filter(|d| d.kind() == k).count()
}

fn roundtrip(text: &str) -> globe_mode_syntax::Parse {
    let toks: String = lex(text)
        .iter()
        .scan(0, |at, t| {
            let s = &text[*at..*at + t.len];
            *at += t.len;
            Some(s)
        })
        .collect();
    assert_eq!(toks, text, "lex");
    let p = parse(text);
    assert_eq!(p.syntax().to_string(), text, "tree");
    p
}

#[test]
fn lexer_kinds() {
    let k: Vec<_> = lex("model M // c\n  Real 'a b' = 1.5e-3 + .5; /* x */ \"s\\\"t\" a.*b <> := ")
        .iter()
        .map(|t| t.kind)
        .filter(|k| !k.is_trivia())
        .collect();
    assert_eq!(
        k,
        [
            KwModel, Ident, Ident, Ident, Eq, UReal, Plus, Dot, UInt, Semi, Str, Ident, DotStar,
            Ident, Ne, Assign
        ]
    );
    // 未終了の文字列・コメント・知らない文字・非 ASCII も全バイトが残る
    for s in [
        "\"abc",
        "/* abc",
        "'q",
        "a ¬ b",
        "日本語",
        "\"\\",
        "x\u{feff}y",
    ] {
        roundtrip(s);
    }
}

#[test]
fn structure_of_a_model() {
    let src = r#"within A.B;
encapsulated partial model M "desc"
  extends Base(x = 1) annotation(a = 1);
  import Modelica.Units.SI;
  import C = Modelica.Constants;
  import Modelica.Math.*;
  parameter Real k[2](each unit = "m") = {1, 2} "gain" annotation(Dialog(group = "g")), j = 3;
  Real x(start = 0) if k[1] > 0;
  replaceable model Inner = Foo constrainedby Bar;
  model Nested
    Real y;
  end Nested;
protected
  Real z = if x > 0 then 1 else 2;
equation
  der(x) = -k[1] * x;
  if x > 0 then
    z = 1;
  elseif x < -1 then
    z = 2;
  else
    z = 3;
  end if;
  for i in 1:2 loop
    connect(a[i], b[i]);
  end for;
  when x < 0 then
    reinit(x, 0);
  end when;
algorithm
  y := if x > 0 then 1 else 2;
  while y < 3 loop y := y + 1; end while;
end M;
type T = Real(unit = "m");
"#;
    let p = roundtrip(src);
    assert!(p.errors.is_empty(), "{:?}", p.errors);
    let t = p.syntax();
    assert_eq!(kinds(&t, WithinClause), 1);
    assert_eq!(kinds(&t, ClassDef), 4);
    assert_eq!(kinds(&t, ShortBody), 2);
    assert_eq!(kinds(&t, ExtendsClause), 1);
    assert_eq!(kinds(&t, ImportClause), 3);
    assert_eq!(kinds(&t, ComponentClause), 4);
    assert_eq!(kinds(&t, Declaration), 5);
    assert_eq!(kinds(&t, Condition), 1);
    assert_eq!(kinds(&t, EquationSection), 1);
    assert_eq!(kinds(&t, AlgorithmSection), 1);
    // der / if(3 枝・中に 3 本) / for(中に 1) / when(中に 1)
    assert_eq!(kinds(&t, Equation), 1 + 1 + 3 + 1 + 1 + 1 + 1);
    assert_eq!(kinds(&t, Statement), 3);
}

#[test]
fn broken_inputs_keep_a_tree() {
    for s in [
        "model M Real x equation x = 1; end M;",
        "model M\n  Real x(\nequation\n  x = 1;\nend M;",
        "model M equation if x then y = 1; end M;",
        "model M equation x = (1; y = 2; end M;",
        "model M end;",
        "model",
        "end if; ) ] ; model",
        "model M model N end N; ",
        "x = 1;",
        "model M equation else y = 1; end if; end M;",
        "model M\n  Real x;\n  ???\n  Real y;\nend M;",
        "model M extends ; import ; Real ; end M;",
    ] {
        let p = roundtrip(s);
        assert!(!p.errors.is_empty(), "no error for {s:?}");
    }
    // 誤りの後ろの宣言は読めている
    let p = roundtrip("model M\n  Real x\nequation\n  x = 1;\nend M;");
    assert_eq!(kinds(&p.syntax(), EquationSection), 1);
    // 深い入れ子で溢れない
    let deep = "model M equation ".to_string() + &"if a then ".repeat(5000);
    roundtrip(&deep);
    roundtrip(&"model M ".repeat(5000));
}

#[test]
fn repo_fixtures() {
    let mut files = Vec::new();
    mo_files(&root().join("src/Modelica"), &mut files);
    assert!(files.len() > 50);
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap();
        let p = roundtrip(&text);
        assert!(
            p.errors.is_empty(),
            "{}: {:?}",
            f.display(),
            &p.errors[..p.errors.len().min(3)]
        );
        // 途中で切っても木は作れて往復する（簡易な fuzz）
        let step = (text.len() / 12).max(1);
        for cut in (0..text.len()).step_by(step) {
            if text.is_char_boundary(cut) {
                roundtrip(&text[..cut]);
            }
        }
    }
    // わざと誤ったファイル
    let bad = root().join("src/Rust/mod/globe-mode-frontend/tests/data");
    let mut files = Vec::new();
    mo_files(&bad, &mut files);
    let broken: Vec<_> = files
        .iter()
        .filter(|f| f.ends_with("SyntaxError.mo") || f.ends_with("Broken.mo"))
        .collect();
    assert!(!broken.is_empty());
    for f in broken {
        let p = roundtrip(&std::fs::read_to_string(f).unwrap());
        assert!(!p.errors.is_empty(), "{}", f.display());
        assert!(kinds(&p.syntax(), ClassDef) >= 1);
    }
}

/// 木を S 式にする。`ComponentRef` / `Literal` / `EndExpr` は字句、ほかは `(種類 子…)`。括弧・コンマは省く
fn sexpr(n: &SyntaxNode) -> String {
    if matches!(n.kind(), ComponentRef | Literal | EndExpr) {
        return n.text().to_string().split_whitespace().collect();
    }
    let mut out = format!("({}", format!("{:?}", n.kind()).trim_end_matches("Expr"));
    for c in n.children_with_tokens() {
        match c {
            rowan::NodeOrToken::Node(c) => {
                out.push(' ');
                out += &sexpr(&c);
            }
            rowan::NodeOrToken::Token(t) => {
                if !t.kind().is_trivia()
                    && !matches!(
                        t.kind(),
                        LParen | RParen | Comma | LBracket | RBracket | LBrace | RBrace
                    )
                {
                    out.push(' ');
                    out += t.text();
                }
            }
        }
    }
    out.push(')');
    out
}

/// `model M Real x = <e>; end M;` の束縛の式を S 式で返す（誤りが無いこと）
fn ex(e: &str) -> String {
    let p = roundtrip(&format!("model M Real x = {e}; end M;"));
    assert!(p.errors.is_empty(), "{e}: {:?}", p.errors);
    let m = p
        .syntax()
        .descendants()
        .find(|d| d.kind() == Modification)
        .unwrap();
    sexpr(&m.children().last().unwrap())
}

#[test]
fn precedence_and_associativity() {
    for (src, want) in [
        ("a + b * c", "(Binary a + (Binary b * c))"),
        ("a - b - c", "(Binary a - b - c)"),
        ("a * b / c", "(Binary a * b / c)"),
        ("-a * b", "(Unary - (Binary a * b))"),
        ("-a + b", "(Binary (Unary - a) + b)"),
        ("a ^ b * c", "(Binary (Binary a ^ b) * c)"),
        ("a * b ^ c", "(Binary a * (Binary b ^ c))"),
        ("-a ^ b", "(Unary - (Binary a ^ b))"),
        ("2 ^ -1 * 3", "(Binary (Binary 2 ^ (Unary - 1)) * 3)"),
        (
            "a .* b .+ c ./ d",
            "(Binary (Binary a .* b) .+ (Binary c ./ d))",
        ),
        ("not a and b", "(Binary (Unary not a) and b)"),
        ("not a < b", "(Unary not (Binary a < b))"),
        ("a or b and c", "(Binary a or (Binary b and c))"),
        (
            "a and b or c and d",
            "(Binary (Binary a and b) or (Binary c and d))",
        ),
        ("a < b + c", "(Binary a < (Binary b + c))"),
        (
            "a + b < c or d",
            "(Binary (Binary (Binary a + b) < c) or d)",
        ),
        ("a == b", "(Binary a == b)"),
        ("1 : n", "(Range 1 : n)"),
        ("1 : 2 : n", "(Range 1 : 2 : n)"),
        ("a + 1 : b or c", "(Range (Binary a + 1) : (Binary b or c))"),
        ("x[:, 1:end].y[i]", "x[:,1:end].y[i]"),
        ("(a + b) * c", "(Binary (Paren (Binary a + b)) * c)"),
        ("(a, , b)", "(Paren a b)"),
        (
            "if a then 1 elseif b then 2 else 3 + 4",
            "(If if a then 1 elseif b then 2 else (Binary 3 + 4))",
        ),
        (
            "1 + if a then 1 else 2",
            "(Binary 1 + (If if a then 1 else 2))",
        ),
        (
            "f(1, k = 2, g(x))",
            "(Call f (FunctionArgs 1 (NamedArg k = 2) (Call g (FunctionArgs x))))",
        ),
        (
            "der(x) + .P.q()",
            "(Binary (Call der (FunctionArgs x)) + (Call .P.q (FunctionArgs)))",
        ),
        ("{1, {2, 3}}", "(Array 1 (Array 2 3))"),
        ("[1, 2; 3, 4]", "(Matrix 1 2 ; 3 4)"),
        (
            "sum(x[i] for i in 1:3, j in v)",
            "(Call sum (FunctionArgs (Comprehension x[i] for (ForIndices (ForIndex i in (Range 1 : 3)) (ForIndex j in v)))))",
        ),
        (
            "{x for i in 1:n}",
            "(Array (Comprehension x for (ForIndices (ForIndex i in (Range 1 : n)))))",
        ),
        (
            "apply(function f(a = 1), 2)",
            "(Call apply (FunctionArgs (PartialApp function f (FunctionArgs (NamedArg a = 1))) 2))",
        ),
        ("\"a\" + \"b\"", "(Binary \"a\" + \"b\")"),
        ("true and not false", "(Binary true and (Unary not false))"),
    ] {
        assert_eq!(ex(src), want, "{src}");
    }
}

#[test]
fn modifications_and_statements() {
    let src = r#"model M
  Real x[3](start = 0, each final unit = "m" "u", redeclare Real y, a.b(c = 1) = 2) = {1, 2, 3}
    if k > 0 "desc" annotation(Placement(transformation(extent = {{-10, -10}, {10, 10}})));
  extends B(final p = 1, break u, break connect(a.b, c), d(e = 1)) annotation(x = 1);
  type T = Real[2](unit = "m") constrainedby Foo(a = 1) "d" annotation(a = 2);
  type E = enumeration(a "x", b);
algorithm
  (a, b) := f(x, 1);
  y := if c then 1 else 2;
  g(x);
  when a then
    return;
  end when;
  while i < 3 loop
    if a then break; elseif b then x := 1; else x := 2; end if;
  end while;
  for i in 1:2, j loop x[i] := i; end for;
equation
  connect(a.b, c[1].d) annotation(Line(points = {{0, 0}, {1, 1}}));
  reinit(x, 0);
  when x > 0 then
    y = 1 "d";
  elsewhen x < 0 then
    y = 2;
  end when;
  x = 1;
end M;
"#;
    let p = roundtrip(src);
    assert!(p.errors.is_empty(), "{:?}", p.errors);
    let t = p.syntax();
    assert_eq!(kinds(&t, ClassModification), 12 + 1);
    assert_eq!(kinds(&t, ElementModification), 16);
    assert_eq!(kinds(&t, BreakModification), 2);
    assert_eq!(kinds(&t, Redeclaration), 1);
    assert_eq!(kinds(&t, Annotation), 4);
    assert_eq!(kinds(&t, ForIndex), 2);
    assert_eq!(kinds(&t, Condition), 1);
    assert_eq!(kinds(&t, ShortBody), 2);
    // 文: 3 本 + when(return) 2 + while(if(break, x:=1, x:=2)) 5 + for(1) 2
    assert_eq!(kinds(&t, Statement), 3 + 2 + 5 + 2);
    // 方程式: connect, reinit, when(中に 2), x = 1
    assert_eq!(kinds(&t, Equation), 2 + 3 + 1);
    // 式の構造を取り出す
    let calls: Vec<_> = t
        .descendants()
        .filter_map(ast::CallExpr::cast)
        .filter_map(|c| c.callee())
        .collect();
    assert_eq!(calls[..2], ["connect", "f"]);
}

#[test]
fn typed_ast_folds_a_flat_chain() {
    fn eval(e: &ast::Expr) -> f64 {
        match e {
            ast::Expr::Literal(l) => l.token().unwrap().text().parse().unwrap(),
            ast::Expr::Paren(p) => eval(&p.items()[0]),
            ast::Expr::Unary(u) => -eval(&u.operand().unwrap()),
            ast::Expr::Binary(b) => {
                let v: Vec<f64> = b.operands().iter().map(eval).collect();
                let mut acc = v[0];
                for (op, x) in b.ops().iter().zip(&v[1..]) {
                    acc = match op.text() {
                        "+" => acc + x,
                        "-" => acc - x,
                        "*" => acc * x,
                        "/" => acc / x,
                        "^" => acc.powf(*x),
                        o => panic!("{o}"),
                    };
                }
                acc
            }
            e => panic!("{:?}", e.syntax()),
        }
    }
    let p = parse("model M equation 1 + 2 * 3 ^ 2 - 4 / (2 - 0) = 0; end M;");
    assert!(p.errors.is_empty());
    let eq = p
        .syntax()
        .descendants()
        .find(|d| d.kind() == Equation)
        .unwrap();
    let e = ast::Expr::cast(eq.children().next().unwrap()).unwrap();
    assert_eq!(eval(&e), 17.0);
    let p = parse("model M equation f(1, k = 2) = if a then 1 elseif b then 2 else 3; end M;");
    let eq = p
        .syntax()
        .descendants()
        .find(|d| d.kind() == Equation)
        .unwrap();
    let mut it = eq.children().filter_map(ast::Expr::cast);
    let (Some(ast::Expr::Call(c)), Some(ast::Expr::If(i))) = (it.next(), it.next()) else {
        panic!()
    };
    assert_eq!((c.args().len(), c.named_args()[0].0.as_str()), (1, "k"));
    assert_eq!(i.branches().len(), 2);
    assert!(i.else_branch().is_some());
}

#[test]
fn deep_and_long_expressions() {
    let m = |e: &str| format!("model M Real x = {e}; end M;");
    for e in [
        "(".repeat(5000),
        "-".repeat(5000) + "1",
        "f(".repeat(5000),
        "{".repeat(5000),
        "[".repeat(5000),
        "if a then ".repeat(5000),
        "a[".repeat(5000),
        "not ".repeat(5000),
        "1 ^ ".repeat(5000) + "1",
        "x(a = ".repeat(5000),
    ] {
        roundtrip(&m(&e));
        roundtrip(&format!("model M equation {e} = 0; end M;"));
        roundtrip(&format!("model M Real x(a(b(c = {e}))); end M;"));
    }
    // 長い平らな連鎖は木を深くしない
    let long = "1 + ".repeat(200_000) + "1";
    let p = roundtrip(&m(&long));
    assert!(p.errors.is_empty());
    let a = "model M Real x(".to_string() + &"a(".repeat(5000);
    roundtrip(&a);
}

#[test]
fn broken_expressions_keep_a_tree() {
    for s in [
        "model M equation x = ; y = 1; end M;",
        "model M equation x = * 3; end M;",
        "model M equation x = (1 + ; y = 2; end M;",
        "model M equation f(1, ) = 0; end M;",
        "model M equation if a then b = 1; end if end M;",
        "model M equation x[1 2] = 3; end M;",
        "model M Real x(start = ) = ; end M;",
        "model M Real x = if a then 1; end M;",
        "model M algorithm x := ; y := 1; end M;",
        "model M algorithm x = 1; end M;",
        "model M equation der = 1; end M;",
        "model M annotation(; end M;",
        "model M Real x annotation(a = , b); end M;",
    ] {
        let p = roundtrip(s);
        assert!(!p.errors.is_empty(), "no error for {s:?}");
    }
    // 誤りの次の文は読める
    let p = roundtrip("model M equation x = ; y = 1; z = 2; end M;");
    assert_eq!(kinds(&p.syntax(), Equation), 3);
    // どの位置で切っても往復する（式の多い入力）
    let src = "model M Real x(start = if a then {1, 2} else [1; 2], each unit = \"m\") = f(a, k = b[1:end].c) \"d\" annotation(A(b = {{1}}));\nequation\n  (a, , b) = g(x for i in 1:n); when x > 0 then reinit(x, -1); end when;\nend M;";
    for cut in 0..src.len() {
        if src.is_char_boundary(cut) {
            roundtrip(&src[..cut]);
        }
    }
}

/// MSL の全 `.mo`（`GLOBE_MSL` か `data/msl`。無ければ何もしない）。
/// `cargo test --release -p globe-mode-syntax -- --ignored --nocapture`
#[test]
#[ignore]
fn msl_all() {
    let dir = std::env::var_os("GLOBE_MSL").map_or_else(|| root().join("data/msl"), PathBuf::from);
    let mut files = Vec::new();
    mo_files(&dir, &mut files);
    if files.is_empty() {
        eprintln!("MSL が無いので省く: {}", dir.display());
        return;
    }
    let (mut ok, mut with_errors, mut bad_roundtrip, mut panics) = (0, 0, 0, 0);
    for f in &files {
        let Ok(text) = std::fs::read_to_string(f) else {
            continue;
        };
        let r = std::panic::catch_unwind(|| {
            let p = parse(&text);
            (
                p.syntax().to_string() == text,
                p.errors
                    .first()
                    .map(|e| format!("{:?} {}", e.range, e.message)),
            )
        });
        match r {
            Err(_) => panics += 1,
            Ok((false, _)) => bad_roundtrip += 1,
            Ok((true, None)) => ok += 1,
            Ok((true, Some(e))) => {
                with_errors += 1;
                eprintln!("errors: {} {e}", f.display());
            }
        }
    }
    eprintln!(
        "files={} ok={ok} with_errors={with_errors} bad_roundtrip={bad_roundtrip} panics={panics}",
        files.len()
    );
    assert_eq!((bad_roundtrip, panics), (0, 0));
}

#[test]
fn field_after_call_stays_in_the_call() {
    // Rumoca の flat は `R.T = f(x).T` と印字する
    let p = parse("model M equation R.T = f(x).T + g(1, 2).a.b; end M;");
    assert!(p.errors.is_empty(), "{:?}", p.errors);
    let t = p.syntax();
    let calls: Vec<_> = t
        .descendants()
        .filter_map(ast::CallExpr::cast)
        .filter_map(|c| c.callee())
        .collect();
    assert_eq!(calls, ["f", "g"]);
}

#[test]
fn subscript_after_paren_stays_in_the_paren() {
    // Rumoca の flat は配列の部品の修飾を `({a for i in 1 : n})[1]` と印字する
    let p =
        parse("model M equation 0 = (({a + f(b) for i in 1 : n})[1]); 0 = (x)[2, 3] + y; end M;");
    assert!(p.errors.is_empty(), "{:?}", p.errors);
    assert_eq!(
        p.syntax()
            .descendants()
            .filter_map(ast::ParenExpr::cast)
            .count(),
        3
    );
}

/// S341: `extends B(redeclare …)` の部品の再宣言（接頭辞・`final`・添字・`constrainedby`）。クラスの再宣言は含まない
#[test]
fn redeclared_components_in_extends() {
    let p = parse(
        "model A extends B(redeclare final X.Y z(k = 1), redeclare replaceable parameter .T[2] w constrainedby C.D, redeclare package M = N, redeclare model extends E); end A;",
    );
    assert!(p.errors.is_empty(), "{:?}", p.errors);
    let ext = p
        .syntax()
        .descendants()
        .find_map(ast::ExtendsClause::cast)
        .unwrap();
    let got: Vec<_> = ext
        .redeclared_components()
        .into_iter()
        .map(|r| {
            (
                r.name,
                r.ty.0.join("."),
                r.ty.1,
                r.constraint.map(|c| c.0.join(".")),
            )
        })
        .collect();
    assert_eq!(
        got,
        vec![
            ("z".into(), "X.Y".into(), false, None),
            ("w".into(), "T".into(), true, Some("C.D".into())),
        ]
    );
    assert_eq!(ext.redeclared_classes().len(), 1);
}
