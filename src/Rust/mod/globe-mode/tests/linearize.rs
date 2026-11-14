//! `Workspace::linearize`（S254）。解析的に分かる線形系・振り子の A・B・C・D と、後の時刻での線形化。

use std::collections::BTreeMap;
use std::path::PathBuf;

use globe_mode::Workspace;

fn workspace() -> Workspace {
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../Modelica/GlobeModeValidation");
    let mut paths = Vec::new();
    globe_mode::collect_mo_files(&dir, &mut paths).unwrap();
    let mut ws = Workspace::new();
    for p in paths {
        ws.set_file(
            &p.display().to_string(),
            std::fs::read_to_string(&p).unwrap(),
        );
    }
    ws
}

fn close(got: &[Vec<f64>], want: &[&[f64]]) {
    assert_eq!(got.len(), want.len(), "{got:?}");
    for (g, w) in got.iter().zip(want) {
        assert_eq!(g.len(), w.len(), "{got:?}");
        for (g, w) in g.iter().zip(*w) {
            assert!((g - w).abs() < 1e-6, "{got:?} != {want:?}");
        }
    }
}

#[test]
fn linear_system_matches_the_analytic_matrices() {
    let l = workspace()
        .linearize(
            "GlobeModeValidation.Simulation.LinearSystem",
            &BTreeMap::new(),
            None,
        )
        .unwrap();
    assert_eq!(l.inputs, ["u"]);
    assert_eq!(l.outputs, ["y1", "y2"]);
    assert_eq!(l.states.len(), 2);
    // 状態の並びは名前で引く（Rumoca の並びに依らない）
    let (i1, i2) = (
        l.states.iter().position(|s| s == "x1").unwrap(),
        l.states.iter().position(|s| s == "x2").unwrap(),
    );
    let a = [[-2.0, 1.0], [1.0, -2.0]];
    let b = [1.0, 2.0];
    let c = [[1.0, 2.0], [1.0, -1.0]];
    let (ix, jx) = ([i1, i2], [0usize, 1]);
    for (r, &ri) in ix.iter().enumerate() {
        assert!((l.b[ri][0] - b[r]).abs() < 1e-6, "{:?}", l.b);
        for (q, &qi) in ix.iter().enumerate() {
            assert!((l.a[ri][qi] - a[r][q]).abs() < 1e-6, "{:?}", l.a);
        }
        for (k, row) in c.iter().enumerate() {
            assert!((l.c[k][ri] - row[jx[r]]).abs() < 1e-6, "{:?}", l.c);
        }
    }
    close(&l.d, &[&[3.0], &[0.0]]);
    assert!((l.u0[0] - 0.5).abs() < 1e-12);
    assert!((l.y0[0] - (1.0 - 2.0 + 1.5)).abs() < 1e-6, "{:?}", l.y0);
}

#[test]
fn pendulum_jacobian_follows_the_state() {
    let mut ws = workspace();
    let m = "GlobeModeValidation.Simulation.Pendulum";
    let at = |th0: f64, ws: &mut Workspace, time| {
        let p = BTreeMap::from([("th0".to_string(), th0)]);
        ws.linearize(m, &p, time).unwrap()
    };
    let l = at(0.5, &mut ws, None);
    assert!(l.inputs.is_empty() && l.outputs.is_empty());
    let (ith, iom) = (
        l.states.iter().position(|s| s == "th").unwrap(),
        l.states.iter().position(|s| s == "om").unwrap(),
    );
    assert!((l.a[iom][ith] + 0.5f64.cos()).abs() < 1e-6, "{:?}", l.a);
    assert!((l.a[iom][iom] + 0.5).abs() < 1e-6 && (l.a[ith][iom] - 1.0).abs() < 1e-6);
    // 平衡点（th = 0）
    let l = at(0.0, &mut ws, None);
    assert!((l.a[iom][ith] + 1.0).abs() < 1e-6, "{:?}", l.a);
    // 後の時刻: 実行した点（th(t) は開始点から動いている）の cos に合う
    let l = at(0.5, &mut ws, Some(2.0));
    assert_eq!(l.time, 2.0);
    assert!((l.x0[ith] - 0.5).abs() > 0.1, "{:?}", l.x0);
    assert!((l.a[iom][ith] + l.x0[ith].cos()).abs() < 1e-6, "{:?}", l.a);
}
