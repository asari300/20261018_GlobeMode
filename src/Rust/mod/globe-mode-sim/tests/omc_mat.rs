//! `read_mat4` を omc 1.27.2 が実際に書いた MAT（#16）で確かめる。`tests/data/omc_decay_res.mat` は
//! 次のモデルを `simulate(Decay, startTime=0, stopTime=1, numberOfIntervals=4, outputFormat="mat")` で回した結果
//! （`parameter Real k(unit = "1/s") = 1; Real x(start = 1, fixed = true, unit = "m"); equation der(x) = -k * x;`）。

use globe_mode_sim::mat::read_mat4;

#[test]
fn read_mat4_reads_a_file_omc_wrote() {
    let bytes = include_bytes!("data/omc_decay_res.mat");
    let m = read_mat4(&mut bytes.as_slice()).unwrap();
    // omc は最後の時刻をもう 1 行書く（0、0.25、0.5、0.75、1、1）
    assert_eq!(m.trajectory.time, [0.0, 0.25, 0.5, 0.75, 1.0, 1.0]);
    // omc は time の後に x・der(x) を時系列に、k を data_1 のパラメータに置く
    let x = m.trajectory.variable("x").expect("x");
    for (t, v) in m.trajectory.time.iter().zip(&x.values) {
        assert!((v - (-t).exp()).abs() < 1e-3, "x({t}) = {v}");
    }
    assert!(m.trajectory.variable("der(x)").is_some());
    let k = m.parameters.iter().find(|p| p.name == "k").expect("k");
    assert_eq!(k.value, 1.0);
}
