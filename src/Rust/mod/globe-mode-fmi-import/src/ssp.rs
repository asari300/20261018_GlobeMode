//! `.ssp`（SSP 2.0.1。S253、ADR-0026 の (r)）を [`CosimSpec`] にする。回すのは S249 のマスターそのまま。
//!
//! `.ssp` を一時ディレクトリに展開し（zip slip・サイズの上限は FMU と同じ `unpack`）、
//! `SystemStructure.ssd` の `Component` を `fmus`、`Connection` を `connections` にする。
//! 通信刻みは SSD に無いので呼び手が渡す。

use std::path::{Component, Path};

use globe_mode_fmi::Causality;
use globe_mode_fmi::ssp::parse_ssd;

use crate::{CosimSpec, Error, Result, io, read_model_description, unpack};

/// 展開した `.ssp`。`spec.fmus` は展開先の FMU を指すので、drop（展開先を消す）まで回してよい。
#[derive(Debug)]
pub struct Ssp {
    _dir: tempfile::TempDir,
    pub spec: CosimSpec,
}

impl Ssp {
    /// `stop_time`・`step` が `None` なら、終了時刻は `DefaultExperiment`（無ければ誤り）、刻みは (終了 - 開始) / 100。
    pub fn open(path: &Path, stop_time: Option<f64>, step: Option<f64>) -> Result<Ssp> {
        let dir = tempfile::tempdir().map_err(io)?;
        unpack(std::fs::File::open(path).map_err(io)?, dir.path())?;
        let ssd = std::fs::read_to_string(dir.path().join("SystemStructure.ssd"))
            .map_err(|e| Error::Ssp(format!("SystemStructure.ssd を読めない: {e}")))?;
        let ssd = parse_ssd(&ssd).map_err(Error::Ssp)?;
        let start = ssd.start_time.unwrap_or(0.0);
        let stop = stop_time.or(ssd.stop_time).ok_or_else(|| {
            Error::Ssp("終了時刻が無い（DefaultExperiment の stopTime か引数）".into())
        })?;
        let mut spec = CosimSpec {
            start_time: Some(start),
            stop_time: stop,
            step: step.unwrap_or((stop - start) / 100.0),
            ..Default::default()
        };
        for c in &ssd.components {
            // source は SSP の中の相対パスだけ（`..`・絶対パス・URI スキームは断る）
            if c.source.contains(':')
                || !Path::new(&c.source)
                    .components()
                    .all(|p| matches!(p, Component::Normal(_)))
            {
                return Err(Error::Ssp(format!(
                    "Component {:?} の source {:?} は SSP の中の相対パスでない",
                    c.name, c.source
                )));
            }
            spec.fmus.insert(c.name.clone(), dir.path().join(&c.source));
        }
        // start / end はデータの向きではない（SSP 5.3.2）。FMU の causality で「出力 → 入力」に揃える
        let causality = |e: &str, v: &str| -> Option<Causality> {
            let d = read_model_description(spec.fmus.get(e)?).ok()?;
            d.variables
                .iter()
                .find(|x| x.name == v)
                .map(|x| x.causality)
        };
        for c in &ssd.connections {
            let (a, b) = (
                format!("{}.{}", c.start_element, c.start_connector),
                format!("{}.{}", c.end_element, c.end_connector),
            );
            let flipped = causality(&c.start_element, &c.start_connector) == Some(Causality::Input)
                && causality(&c.end_element, &c.end_connector) == Some(Causality::Output);
            spec.connections.push(if flipped { (b, a) } else { (a, b) });
        }
        Ok(Ssp { _dir: dir, spec })
    }
}
