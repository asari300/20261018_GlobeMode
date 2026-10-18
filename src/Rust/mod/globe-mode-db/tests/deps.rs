//! `deps_closure`（Rumoca に渡すファイルの絞り込み）と、S7・S8 のクエリのテスト。
//! 入力は src/Modelica/GlobeModeValidation（MSL を使わないモデルだけを検査する。MSL を使う計測は `#[ignore]`）。

use std::path::PathBuf;

use globe_mode_core::{Stage, codes, line_col, normalize};
use globe_mode_db::GlobeDb;
use globe_mode_frontend::Source;
use globe_mode_ir::ClassKind;

fn package_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation")
}

fn msl_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../data/msl")
}

fn load(db: &mut GlobeDb) {
    db.load_dir(&package_dir())
        .expect("GlobeModeValidation を読める");
}

fn loaded() -> GlobeDb {
    let mut db = GlobeDb::new();
    load(&mut db);
    db
}

/// パッケージの中のファイルのパス（`load_dir` が読んだパスを、DB と同じく区切り `/` にそろえたもの）。
fn file(name: &str) -> String {
    globe_mode_db::normalize_path(
        &name
            .split('/')
            .fold(package_dir(), |p, part| p.join(part))
            .display()
            .to_string(),
    )
}

/// DB のパスを、パッケージの中の相対パス（`Connect/RealCircuit.mo`）に直す。
fn relative(paths: &[String]) -> Vec<String> {
    let base = file("").trim_end_matches('/').to_string() + "/";
    paths
        .iter()
        .map(|p| p.strip_prefix(&base).unwrap_or(p).to_string())
        .collect()
}

/// ファイルの本文（Windows の CRLF でも置換が当たるように LF にそろえる）。
fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap().replace("\r\n", "\n")
}

fn count(executed: &[String], query: &str) -> usize {
    executed.iter().filter(|q| q.as_str() == query).count()
}

const CIRCUIT: &str = "GlobeModeValidation.Connect.RealCircuit";
const MEDIUM: &str = "GlobeModeValidation.Connect.MediumFlow";

// --------------------------------------------------------------------------- 閉包の中身

#[test]
fn real_circuit_closure_has_only_the_files_it_uses() {
    let mut db = loaded();
    let closure = relative(&db.deps_closure(CIRCUIT));
    // パスの昇順。`within` の連鎖（GlobeModeValidation → Connect）と、部品（Components）、
    // 部品のコネクタ（Connectors。Components.mo の中から参照）。
    // Components.mo の `replaceable package Medium` は、MediumFlow.mo の入れ子の `Medium` と同じ名前だが、
    // 入れ子のクラスは突き合わせない（defined_classes はトップレベルだけ）ので、MediumFlow.mo は入らない
    assert_eq!(
        closure,
        [
            "Components.mo",
            "Connect/RealCircuit.mo",
            "Connect/package.mo",
            "Connectors.mo",
            "package.mo",
        ]
    );
    for excluded in [
        "Connect/MediumFlow.mo",
        "Connect/RealSignals.mo",
        "Invalid/package.mo",
        "Invalid/StructurallySingular.mo",
        "Invalid/HighIndexDuplicate.mo",
    ] {
        assert!(!closure.iter().any(|p| p == excluded), "{excluded}");
    }
    assert!(closure.len() < db.paths().count());
}

#[test]
fn closures_follow_component_types_and_within() {
    let mut db = loaded();
    // 部品の型が同じパッケージの別のファイル（Subsystem.mo）にある
    let hierarchical = relative(&db.deps_closure("GlobeModeValidation.Connect.Hierarchical"));
    assert!(
        hierarchical.iter().any(|p| p == "Connect/Subsystem.mo"),
        "{hierarchical:?}"
    );
    // 部品を使わないモデルは、within の連鎖だけ
    let singular = relative(&db.deps_closure("GlobeModeValidation.Invalid.StructurallySingular"));
    assert_eq!(
        singular,
        [
            "Invalid/StructurallySingular.mo",
            "Invalid/package.mo",
            "package.mo"
        ]
    );
    // 入れ子のクラスがモデルなら、それを入れ子に持つファイルが起点
    let resistor = relative(&db.deps_closure("GlobeModeValidation.Components.Resistor"));
    assert_eq!(resistor, ["Components.mo", "Connectors.mo", "package.mo"]);
}

#[test]
fn unknown_model_has_an_empty_closure_and_stops_at_s1_with_candidates() {
    let mut db = loaded();
    let model = "GlobeModeValidation.Connect.Nope";
    assert!(db.deps_closure(model).is_empty());
    let r = db.check_model(model);
    assert_eq!(r.failed_stage(), Some(Stage::S1), "{:?}", r.diagnostics);
    let d = &r.diagnostics[0];
    assert_eq!(d.code, codes::MODEL_NOT_FOUND);
    // 文書を渡していないので、候補はワークスペースの索引から足す
    assert!(
        d.notes
            .iter()
            .any(|n| n.starts_with("候補") && n.contains(CIRCUIT)),
        "{:?}",
        d.notes
    );
}

#[test]
fn a_file_with_a_syntax_error_is_always_in_the_closure() {
    let mut db = loaded();
    // 何を定義しているか分からないので、使うかどうかに関わらず入れる（S0 で止まる）
    let broken = file("Invalid/StructurallySingular.mo");
    db.set_file(
        &broken,
        "within GlobeModeValidation.Invalid;\nmodel StructurallySingular\n  Real x\nend StructurallySingular;\n",
    );
    assert!(db.deps_closure(CIRCUIT).contains(&broken));
    let r = db.check_model(CIRCUIT);
    assert_eq!(r.failed_stage(), Some(Stage::S0));
    assert_eq!(r.diagnostics[0].file.as_deref(), Some(broken.as_str()));
}

#[test]
fn a_new_file_that_the_model_uses_joins_the_closure() {
    let mut db = loaded();
    let model_path = file("Connect/RealCircuit.mo");
    assert!(db.check_model(CIRCUIT).passed());
    db.take_executed();

    // 新しいファイルの部品を使う
    let extra = file("Extra.mo");
    db.set_file(
        &extra,
        "within GlobeModeValidation;\npackage Extra\n  model Lamp\n    extends Components.Resistor;\n  end Lamp;\nend Extra;\n",
    );
    let text = read(&model_path);
    db.set_file(
        &model_path,
        text.replace(
            "  Components.Ground ground;\n",
            "  Components.Ground ground;\n  Extra.Lamp lamp(R = 50);\n",
        )
        .replace(
            "  connect(source.n, ground.p);\n",
            "  connect(source.n, ground.p);\n  connect(source.p, lamp.p);\n  connect(lamp.n, source.n);\n",
        ),
    );
    assert!(db.deps_closure(CIRCUIT).contains(&extra));
    let r = db.check_model(CIRCUIT);
    assert!(r.passed(), "{:?}", r.diagnostics);
    assert_eq!(count(&db.take_executed(), "check"), 1);
}

#[test]
fn checking_the_closure_gives_the_same_report_as_checking_all_files() {
    // 「足りないのは駄目」の確かめ: 索引のモデルとブロックをすべて、閉包だけと全ファイルとで検査して比べる
    let mut db = loaded();
    let models: Vec<String> = {
        let index = db.class_index();
        index
            .names()
            .filter(|n| {
                index
                    .get(n)
                    .is_some_and(|c| matches!(c.kind, ClassKind::Model | ClassKind::Block))
            })
            .map(str::to_string)
            .collect()
    };
    assert!(models.len() >= 20, "{}", models.len());
    let all: Vec<(String, String)> = db
        .paths()
        .map(|p| {
            let text = read(p);
            (p.to_string(), normalize::source_shape(&text).text)
        })
        .collect();
    let sources: Vec<Source<'_>> = all
        .iter()
        .map(|(name, text)| Source { name, text })
        .collect();
    let checker = db.checker().clone();
    for model in &models {
        let ours = db.check(model);
        let reference = checker.check(&sources, model);
        // MSL を使うモデルは、どちらでも同じ所で止まる（ここではライブラリを読んでいない）
        assert_eq!(ours, reference, "{model}");
    }
}

// --------------------------------------------------------------------------- 防火壁

#[test]
fn editing_an_unused_file_does_not_rerun_check() {
    let mut db = loaded();
    let before = db.check_model(CIRCUIT);
    db.take_executed();

    // 式を変える（空白やコメントではない）。RealCircuit の閉包には入っていない
    let path = file("Invalid/StructurallySingular.mo");
    let text = read(&path);
    let edited = text.replace("  x = 2;\n", "  x = 3 * y;\n");
    assert_ne!(edited, text);
    db.set_file(&path, edited);

    let after = db.check_model(CIRCUIT);
    let executed = db.take_executed();
    assert_eq!(count(&executed, "source_shape"), 0, "{executed:?}");
    assert_eq!(count(&executed, "deps_closure"), 0, "{executed:?}");
    assert_eq!(count(&executed, "check"), 0, "{executed:?}");
    assert_eq!(count(&executed, "structural"), 0, "{executed:?}");
    assert_eq!(after, before);
}

#[test]
fn editing_a_used_file_reruns_check_once() {
    let mut db = loaded();
    db.check_model(CIRCUIT);
    db.take_executed();

    let path = file("Components.mo");
    let text = read(&path);
    let edited = text.replace("parameter Real R = 1 ", "parameter Real R = 2 ");
    assert_ne!(edited, text);
    db.set_file(&path, edited);

    let r = db.check_model(CIRCUIT);
    assert!(r.passed(), "{:?}", r.diagnostics);
    let executed = db.take_executed();
    assert_eq!(count(&executed, "check"), 1, "{executed:?}");
    // 識別子は変わらないので、閉包は前と等しい（作り直しても下流には伝わらない）
    assert_eq!(count(&executed, "identifiers"), 1, "{executed:?}");
}

#[test]
fn declarations_in_an_unused_file_reach_the_index_but_not_check() {
    let mut db = loaded();
    db.check_model(CIRCUIT);
    db.take_executed();

    // 使わないファイルに部品を足す: アウトラインと索引は変わり、閉包は作り直すが、中身は同じ
    let path = file("Connect/RealSignals.mo");
    let text = read(&path);
    let edited = text.replace(
        "  Components.RealGain gain2(k = 0.5);\n",
        "  Components.RealGain gain2(k = 0.5);\n  Components.RealGain gain3;\n",
    );
    assert_ne!(edited, text);
    db.set_file(&path, edited);

    db.check_model(CIRCUIT);
    let executed = db.take_executed();
    assert_eq!(count(&executed, "class_index"), 1, "{executed:?}");
    assert_eq!(count(&executed, "deps_closure"), 1, "{executed:?}");
    assert_eq!(count(&executed, "check"), 0, "{executed:?}");
}

// --------------------------------------------------------------------------- S7・S8

#[test]
fn high_index_duplicate_has_an_index_report() {
    let mut db = loaded();
    let path = file("Invalid/HighIndexDuplicate.mo");
    let r = db.check_model("GlobeModeValidation.Invalid.HighIndexDuplicate");
    // S7 は Info だけなので、S7 では止まらない。S8 は動的な状態 0 個に対して fixed = true の vC が
    // 1 個あるので過剰（GM-E8001）になる（S8 の要る数は S7 の動的な状態の数で数える）
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.is_error() && d.stage <= Stage::S7),
        "{:?}",
        r.diagnostics
    );
    assert!(r.init.is_some());
    let s6 = r.structural.as_ref().expect("S6");
    assert!(s6.is_regular());
    let s7 = r.index.as_ref().expect("S7");
    assert_eq!(s7.differentiations.len(), s6.n_equations);
    // 電圧則 2 本を 1 回ずつ微分する（Pantelides 法）。状態 vC が状態選択の候補
    assert_eq!((s7.index, s7.n_differentiated), (2, 2), "{s7:?}");
    assert_eq!(s7.dummy_candidates.len(), 1, "{s7:?}");
    // S7 の診断は IndexReport::diagnostics からだけ（S6 の結果からは出さない）
    let s7_codes: Vec<&str> = r
        .diagnostics
        .iter()
        .filter(|d| d.stage == Stage::S7)
        .map(|d| d.code)
        .collect();
    assert_eq!(s7_codes, [codes::HIGH_INDEX, codes::STATE_SELECTION]);
    // 位置は元の本文（先頭のコメントの後ろ）に戻してある
    let text = read(&path);
    for d in r.diagnostics.iter().filter(|d| d.stage == Stage::S7) {
        if let Some(range) = d.range {
            assert_eq!(d.file.as_deref(), Some(path.as_str()));
            assert!(line_col(&text, range.start).line > 9, "{d:?}");
        }
    }
}

#[test]
fn real_circuit_reaches_s7_and_s8() {
    let mut db = loaded();
    let r = db.check_model(CIRCUIT);
    assert!(r.passed(), "{:?}", r.diagnostics);
    assert!(r.index.is_some());
    // C の抽出が入るまでは、S8 の入力が無い理由が notes にある
    match (&r.init, &r.check.init) {
        (Some(s8), Some(init)) => {
            assert_eq!(s8.required, init.states.len() + s8.free_parameters.len());
            // 状態 capacitor.v に fixed も initial equation も無い: 不足は Warning で、止めない
            assert_eq!((s8.required, s8.provided), (1, 0), "{s8:?}");
            assert!(
                r.diagnostics
                    .iter()
                    .any(|d| d.code == codes::INIT_UNDERDETERMINED && d.stage == Stage::S8),
                "{:?}",
                r.diagnostics
            );
        }
        (None, None) => assert!(
            r.check.notes.iter().any(|n| n.contains("S8")),
            "{:?}",
            r.check.notes
        ),
        other => panic!("init と S8 の結果が食い違う: {other:?}"),
    }
}

#[test]
fn init_positions_can_be_mapped_back_to_the_original_text() {
    let mut db = loaded();
    let r = db.check_model(CIRCUIT);
    let Some(init) = &r.check.init else {
        // C の抽出が入るまでは、S8 の入力が無い
        return;
    };
    let original = db.init_in_original(init);
    let state = &original.states[0];
    assert_eq!(state.name, "capacitor.v");
    let (Some(path), Some(range)) = (&state.file, state.range) else {
        return; // 位置の無い状態（C の抽出の都合）なら確かめない
    };
    // Components.mo の TwoPin の `Real v` の宣言（字下げがあるので、正規化した本文の中の位置とは違う）
    let text = read(path);
    assert!(
        text[range.start as usize..range.end as usize].contains('v'),
        "{range:?}"
    );
    assert!(line_col(&text, range.start).line > 1);
    assert_ne!(Some(range), init.states[0].range);
}

#[test]
fn s8_counts_the_dynamic_states_from_s7() {
    // インデックス 3 の振り子: 状態 4 個のうち、動的な状態は 2 個（残りは微分した拘束で決まる）
    let mut db = loaded();
    let r = db.check_model("GlobeModeValidation.Index.Pendulum");
    let s7 = r.index.as_ref().expect("S7");
    assert!(s7.index >= 2 && !s7.failed, "{s7:?}");
    let init = r.check.init.as_ref().expect("S8 の入力");
    assert!(s7.dynamic_states < init.states.len(), "{s7:?}");
    let s8 = r.init.as_ref().expect("S8");
    assert_eq!(
        s8.required,
        s7.dynamic_states + s8.free_parameters.len(),
        "{s8:?}"
    );
}

#[test]
fn an_overdetermined_initialization_stops_at_s8() {
    let mut db = loaded();
    let r = db.check_model("GlobeModeValidation.Init.OverDetermined");
    assert!(r.check.passed(), "{:?}", r.check.diagnostics);
    assert!(r.index.is_some());
    let s8 = r.init.as_ref().expect("S8");
    assert!(s8.provided > s8.required, "{s8:?}");
    assert_eq!(r.failed_stage(), Some(Stage::S8), "{:?}", r.diagnostics);
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == codes::INIT_OVERDETERMINED)
    );
}

#[test]
fn stages_after_an_error_are_not_run() {
    let mut db = loaded();
    // S6 で止まるモデルは、S7 と S8 に進まない
    let r = db.check_model("GlobeModeValidation.Invalid.StructurallySingular");
    assert_eq!(r.failed_stage(), Some(Stage::S6));
    assert!(r.index.is_none());
    assert!(r.init.is_none());
    // S4 で止まるモデルは、S6 にも進まない
    let r = db.check_model("GlobeModeValidation.Invalid.SignalTypeMismatch");
    assert!(r.structural.is_none() && r.index.is_none() && r.init.is_none());
}

#[test]
fn a_parameter_value_edit_does_not_rerun_s7_or_s8() {
    let mut db = loaded();
    let path = file("Connect/RealCircuit.mo");
    let text = read(&path);
    db.check_model(CIRCUIT);
    db.take_executed();

    // 長さの違う値: check の結果（位置）は変わるが、位置を落とした構造は同じ
    db.set_file(
        &path,
        text.replace("resistor(R = 100)", "resistor(R = 1000)"),
    );
    let r = db.check_model(CIRCUIT);
    assert!(r.passed(), "{:?}", r.diagnostics);
    let executed = db.take_executed();
    assert_eq!(count(&executed, "check"), 1, "{executed:?}");
    assert_eq!(count(&executed, "index_report"), 0, "{executed:?}");
    assert_eq!(count(&executed, "init_report"), 0, "{executed:?}");
}

// --------------------------------------------------------------------------- 計測

/// MSL を使うモデルの検査の時間。`data/msl` が無ければ何もしない。
/// `cargo test --release -p globe-mode-db --test deps -- --ignored --nocapture` で見る。
#[test]
#[ignore]
fn measure_checks_with_msl() {
    let msl = msl_dir();
    if !msl.is_dir() {
        eprintln!("{} が無いので計測しない", msl.display());
        return;
    }
    let t = std::time::Instant::now();
    let mut db = GlobeDb::with_libraries(&[msl]).expect("MSL を読める");
    let msl_load = ms(t);
    load(&mut db);
    let unused = file("Invalid/StructurallySingular.mo");
    let used = file("Components.mo");
    let unused_text = read(&unused);
    let used_text = read(&used);
    eprintln!(
        "MediumFlow の閉包: {:?}（全 {} ファイル）",
        relative(&db.deps_closure(MEDIUM)),
        db.paths().count()
    );

    // (a) 最初の検査
    let t = std::time::Instant::now();
    let r = db.check_model(MEDIUM);
    let first = ms(t);
    eprintln!(
        "MediumFlow: passed = {}, failed_stage = {:?}, notes = {:?}",
        r.passed(),
        r.failed_stage(),
        r.check.notes
    );
    db.take_executed();

    // (b) 使わないファイルの式を変えた後の再検査（check は 0 回）
    let mut unused_times = Vec::new();
    for i in 0..5 {
        db.set_file(
            &unused,
            unused_text.replace("  x = 2;\n", &format!("  x = {};\n", i + 3)),
        );
        let t = std::time::Instant::now();
        db.check_model(MEDIUM);
        unused_times.push(ms(t));
    }
    let executed = db.take_executed();
    assert_eq!(count(&executed, "check"), 0, "{executed:?}");

    // (c) 使うファイルの式を変えた後の再検査（文書を差し替えるので、Rumoca が解決し直す）
    let mut used_times = Vec::new();
    for i in 0..5 {
        db.set_file(
            &used,
            used_text.replace(
                "parameter Real k = 1e-3 ",
                &format!("parameter Real k = {}e-3 ", i + 2),
            ),
        );
        let t = std::time::Instant::now();
        db.check_model(MEDIUM);
        used_times.push(ms(t));
    }
    let executed = db.take_executed();
    assert_eq!(count(&executed, "check"), 5, "{executed:?}");

    // (d) 閉包の違うモデルに切り替える（MediumFlow → RealCircuit → MediumFlow）。
    // salsa が結果を覚えているので、両方に効く Components.mo を毎回変えてから検査する
    let (mut to_circuit, mut back_to_medium) = (Vec::new(), Vec::new());
    for i in 0..5 {
        db.set_file(
            &used,
            used_text.replace(
                "parameter Real R = 1 ",
                &format!("parameter Real R = {} ", i + 2),
            ),
        );
        let t = std::time::Instant::now();
        assert!(db.check_model(CIRCUIT).passed());
        to_circuit.push(ms(t));
        let t = std::time::Instant::now();
        db.check_model(MEDIUM);
        back_to_medium.push(ms(t));
    }

    // 参考: S005 の方式（全ファイルを文書として渡す）で、(c) と同じ編集の後の再検査
    let checker = db.checker().clone();
    let mut all_files_times = Vec::new();
    for i in 0..5 {
        let texts: Vec<(String, String)> = db
            .paths()
            .map(|p| {
                let text = if p == used {
                    used_text.replace(
                        "parameter Real k = 1e-3 ",
                        &format!("parameter Real k = {}e-3 ", i + 7),
                    )
                } else {
                    read(p)
                };
                (p.to_string(), normalize::source_shape(&text).text)
            })
            .collect();
        let sources: Vec<Source<'_>> = texts
            .iter()
            .map(|(name, text)| Source { name, text })
            .collect();
        let t = std::time::Instant::now();
        checker.check(&sources, MEDIUM);
        all_files_times.push(ms(t));
    }

    // (e) 編集なしで、閉包の違うモデルを初めて検査する（文書の組が変わるので Rumoca が解決し直す）。
    // 参考として、全ファイルを渡したまま同じモデルを検査する（文書の組は変わらない。1 回目で揃えてから測る）
    let others = [
        "Connect.RealSignals",
        "Connect.BooleanSignals",
        "Connect.Hierarchical",
        "Init.Balanced",
        "Index.Pendulum",
    ];
    let mut switch_times = Vec::new();
    for m in others {
        let m = format!("GlobeModeValidation.{m}");
        let t = std::time::Instant::now();
        db.check_model(&m);
        switch_times.push(ms(t));
    }
    let texts: Vec<(String, String)> = db
        .paths()
        .map(|p| (p.to_string(), normalize::source_shape(&read(p)).text))
        .collect();
    let sources: Vec<Source<'_>> = texts
        .iter()
        .map(|(name, text)| Source { name, text })
        .collect();
    checker.check(&sources, MEDIUM);
    let mut same_docs_times = Vec::new();
    for m in others {
        let m = format!("GlobeModeValidation.{m}");
        let t = std::time::Instant::now();
        checker.check(&sources, &m);
        same_docs_times.push(ms(t));
    }

    eprintln!("MSL の読み込み: {msl_load:.0} ms");
    eprintln!("(a) MediumFlow の最初の検査: {first:.0} ms");
    for (label, times) in [
        ("(b) 使わないファイルの式を変えた後", &mut unused_times),
        (
            "(c) 使うファイル（Components.mo）を変えた後",
            &mut used_times,
        ),
        ("(d) MediumFlow → RealCircuit", &mut to_circuit),
        ("(d) RealCircuit → MediumFlow", &mut back_to_medium),
        (
            "参考: 全ファイルを渡して (c) と同じ編集",
            &mut all_files_times,
        ),
        (
            "(e) 編集なしで閉包の違うモデルを初めて検査",
            &mut switch_times,
        ),
        (
            "参考: 全ファイルを渡したまま (e) と同じモデル",
            &mut same_docs_times,
        ),
    ] {
        times.sort_by(f64::total_cmp);
        eprintln!(
            "{label}: 中央値 {:.2} ms / 最大 {:.2} ms（5 回）",
            times[2], times[4]
        );
    }
}

fn ms(t: std::time::Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}
