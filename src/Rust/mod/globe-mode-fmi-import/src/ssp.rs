//! `.ssp`（SSP 2.0.1。S253、ADR-0026 の (r)）を [`CosimSpec`] にする。回すのは S249 のマスターそのまま。
//!
//! `.ssp` を一時ディレクトリに展開し（zip slip・サイズの上限は FMU と同じ `unpack`）、
//! `SystemStructure.ssd` の `Component` を `fmus`、`Connection` を `connections` にする。
//! 通信刻みは SSD に無いので呼び手が渡す（S274 の `write_ssp` が書いた `.ssp` なら、独自の `Annotation` から読む）。`Component` の `ParameterBindings` は `start` にする（S263）。

use std::path::{Component, Path};

use std::collections::BTreeMap;
use std::fmt::Write as _;

use globe_mode_fmi::ssp::{COSIM_ANNOTATION, parse_ssd, parse_ssv};
use globe_mode_fmi::{Causality, VarType};

use crate::cosim::valid_fmu_name;
use crate::{Algorithm, CosimSpec, Error, Result, io, read_model_description, unpack};

/// SSP の中の相対パスだけ（`..`・絶対パス・URI スキームは断る）。
fn is_inside_relative(source: &str) -> bool {
    !source.contains(':')
        && Path::new(source)
            .components()
            .all(|p| matches!(p, Component::Normal(_)))
}

/// 展開した `.ssp`。`spec.fmus` は展開先の FMU を指すので、drop（展開先を消す）まで回してよい。
#[derive(Debug)]
pub struct Ssp {
    _dir: tempfile::TempDir,
    pub spec: CosimSpec,
}

impl Ssp {
    /// `stop_time`・`step` が `None` なら、終了時刻は `DefaultExperiment`（無ければ誤り）、刻みは Annotation（S274）か (終了 - 開始) / 100。
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
            step: step.or(ssd.cosim_step).unwrap_or((stop - start) / 100.0),
            algorithm: match ssd.cosim_algorithm.as_deref() {
                None | Some("jacobi") => Algorithm::Jacobi,
                Some("gauss-seidel") => Algorithm::GaussSeidel,
                Some(a) => return Err(Error::Ssp(format!("Annotation の algorithm {a:?} は未知"))),
            },
            ..Default::default()
        };
        for c in &ssd.components {
            if !is_inside_relative(&c.source) {
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
        // ParameterBindings → start（5.2.3: 後ろの binding が勝つ。FMU に無い名前は無視する）
        for c in &ssd.components {
            let Ok(d) = read_model_description(&spec.fmus[&c.name]) else {
                continue; // 読めない FMU は実行のときに誤りになる
            };
            for b in &c.bindings {
                let params = match &b.source {
                    None => b.params.clone(),
                    Some(s) if is_inside_relative(s) => {
                        let xml = std::fs::read_to_string(dir.path().join(s)).map_err(|e| {
                            Error::Ssp(format!(
                                "Component {:?} の ParameterBinding {s:?} を読めない: {e}",
                                c.name
                            ))
                        })?;
                        parse_ssv(&xml).map_err(|m| Error::Ssp(format!("{s}: {m}")))?
                    }
                    Some(s) => {
                        return Err(Error::Ssp(format!(
                            "Component {:?} の ParameterBinding の source {s:?} は SSP の中の相対パスでない",
                            c.name
                        )));
                    }
                };
                for (n, v) in params {
                    let n = format!("{}{n}", b.prefix);
                    if d.variables.iter().any(|x| x.name == n) {
                        spec.start.insert(format!("{}.{n}", c.name), v);
                    }
                }
            }
        }
        Ok(Ssp { _dir: dir, spec })
    }
}

/// XML の属性値に入れる文字列。
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// [`CosimSpec`] を `.ssp` に書く（S274）。[`Ssp::open`] で読み戻すと同じ結線・start・時刻・刻み・algorithm になる。
///
/// SSD は `System` 1 つ。`Component` は `resources/<名前>.fmu`（FMU の zip をそのまま写す）、`Connectors` は結線に出る変数だけ
/// （causality・型は FMU の modelDescription から）、`start` は `ParameterBindings` のインライン、時刻は `DefaultExperiment`、
/// 刻みと algorithm は独自の `Annotation`（[`COSIM_ANNOTATION`]）に入れる。`timeout`・`memory_limit_mib` は実行の上限なので書かない。
/// 結線は Real（Float64）だけ（[`Ssp::open`] が Real 以外のコネクタを断るため）。
/// `"a.x"` を（FMU の名前, 変数名, 変数）にする。
fn var_of<'a>(
    descs: &'a BTreeMap<&String, globe_mode_fmi::ModelDescription>,
    key: &'a str,
) -> Result<(&'a str, &'a str, &'a globe_mode_fmi::ScalarVariable)> {
    let err = |m: String| Error::Ssp(m);
    let (f, v) = key
        .split_once('.')
        .ok_or_else(|| err(format!("{key:?} が `<FMU の名前>.<変数名>` の形でない")))?;
    let d = descs
        .get(&f.to_owned())
        .ok_or_else(|| err(format!("{key:?} の FMU {f:?} が `fmus` に無い")))?;
    let x = d
        .variables
        .iter()
        .find(|x| x.name == v)
        .ok_or_else(|| err(format!("{key:?}: 変数 {v:?} が無い")))?;
    Ok((f, v, x))
}

pub fn write_ssp(spec: &CosimSpec, out: &Path) -> Result<()> {
    use std::io::Write;
    let err = |m: String| Error::Ssp(m);
    let mut descs = BTreeMap::new();
    for (name, path) in &spec.fmus {
        if !valid_fmu_name(name) {
            return Err(err(format!(
                "FMU の名前 {name:?} が不正（英数字・`_`・`-` だけ）"
            )));
        }
        descs.insert(name, read_model_description(path)?);
    }
    // コンポーネントごとの (変数名 → "input" | "output")
    let mut connectors: BTreeMap<&str, BTreeMap<&str, &str>> = BTreeMap::new();
    let mut connections = String::new();
    for (a, b) in &spec.connections {
        let mut ends = Vec::new();
        for (key, want, kind) in [
            (a, Causality::Output, "output"),
            (b, Causality::Input, "input"),
        ] {
            let (f, v, x) = var_of(&descs, key)?;
            if x.causality != want || x.ty != VarType::Float64 || x.dimensions {
                return Err(err(format!(
                    "接続 {a} -> {b}: {key} は {kind} の Float64（スカラー）でない（SSP の結線は Real だけ）"
                )));
            }
            connectors.entry(f).or_default().insert(v, kind);
            ends.push((esc(f), esc(v)));
        }
        writeln!(
            connections,
            r#"      <ssd:Connection startElement="{}" startConnector="{}" endElement="{}" endConnector="{}"/>"#,
            ends[0].0, ends[0].1, ends[1].0, ends[1].1
        )
        .unwrap();
    }
    let mut starts: BTreeMap<&str, String> = BTreeMap::new();
    for (key, &value) in &spec.start {
        let (f, v, x) = var_of(&descs, key)?;
        let typed = match x.ty {
            VarType::Float64 if value.is_finite() => format!("<ssv:Real value=\"{value:?}\"/>"),
            VarType::Int32 if value.fract() == 0.0 && value.abs() <= f64::from(i32::MAX) => {
                format!("<ssv:Integer value=\"{}\"/>", value as i32)
            }
            VarType::Boolean if value == 0.0 || value == 1.0 => {
                format!("<ssv:Boolean value=\"{}\"/>", value == 1.0)
            }
            _ => {
                return Err(err(format!(
                    "start {key}: 値 {value} を {:?} として書けない",
                    x.ty
                )));
            }
        };
        writeln!(
            starts.entry(f).or_default(),
            "            <ssv:Parameter name=\"{}\">{typed}</ssv:Parameter>",
            esc(v)
        )
        .unwrap();
    }
    let mut ssd = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ssd:SystemStructureDescription xmlns:ssd="http://ssp-standard.org/SSP1/SystemStructureDescription"
  xmlns:ssc="http://ssp-standard.org/SSP1/SystemStructureCommon"
  xmlns:ssv="http://ssp-standard.org/SSP1/SystemStructureParameterValues"
  xmlns:globe="urn:globe-mode:cosim" name="system" version="2.0">
  <ssd:System name="system">
    <ssd:Elements>
"#,
    );
    for name in spec.fmus.keys() {
        let n = esc(name);
        writeln!(ssd, r#"      <ssd:Component name="{n}" type="application/x-fmu-sharedlibrary" source="resources/{n}.fmu">"#).unwrap();
        if let Some(c) = connectors.get(name.as_str()) {
            ssd.push_str("        <ssd:Connectors>\n");
            for (v, kind) in c {
                writeln!(ssd, r#"          <ssd:Connector name="{}" kind="{kind}"><ssc:Real/></ssd:Connector>"#, esc(v)).unwrap();
            }
            ssd.push_str("        </ssd:Connectors>\n");
        }
        if let Some(p) = starts.get(name.as_str()) {
            write!(ssd, "        <ssd:ParameterBindings><ssd:ParameterBinding><ssd:ParameterValues>\n          <ssv:ParameterSet version=\"2.0\" name=\"start\"><ssv:Parameters>\n{p}          </ssv:Parameters></ssv:ParameterSet>\n        </ssd:ParameterValues></ssd:ParameterBinding></ssd:ParameterBindings>\n").unwrap();
        }
        ssd.push_str("      </ssd:Component>\n");
    }
    let algorithm = match spec.algorithm {
        Algorithm::Jacobi => "jacobi",
        Algorithm::GaussSeidel => "gauss-seidel",
    };
    write!(
        ssd,
        r#"    </ssd:Elements>
    <ssd:Connections>
{connections}    </ssd:Connections>
  </ssd:System>
  <ssd:DefaultExperiment startTime="{:?}" stopTime="{:?}"/>
  <ssc:Annotations><ssc:Annotation type="{COSIM_ANNOTATION}"><globe:cosim step="{:?}" algorithm="{algorithm}"/></ssc:Annotation></ssc:Annotations>
</ssd:SystemStructureDescription>
"#,
        spec.start_time.unwrap_or(0.0),
        spec.stop_time,
        spec.step
    )
    .unwrap();
    // 書き終えてから置く（途中で失敗した壊れた .ssp を残さない）。FMU は zip のまま写すので Stored
    let tmp = out.with_extension("ssp.tmp");
    let result = (|| -> Result<()> {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&tmp).map_err(io)?);
        zip.start_file(
            "SystemStructure.ssd",
            zip::write::SimpleFileOptions::default(),
        )
        .map_err(crate::zip_err)?;
        zip.write_all(ssd.as_bytes()).map_err(io)?;
        let stored = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .large_file(true);
        for (name, path) in &spec.fmus {
            zip.start_file(format!("resources/{name}.fmu"), stored)
                .map_err(crate::zip_err)?;
            std::io::copy(&mut std::fs::File::open(path).map_err(io)?, &mut zip).map_err(io)?;
        }
        zip.finish().map_err(crate::zip_err)?;
        std::fs::rename(&tmp, out).map_err(io)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// `modelDescription.xml` だけの FMU（実行はしない）。`vars` は FMI 2.0 の `ScalarVariable` の列。
    fn fake_fmu(dir: &Path, name: &str, vars: &str) -> std::path::PathBuf {
        let path = dir.join(format!("{name}.fmu"));
        let mut z = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        z.start_file(
            "modelDescription.xml",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        write!(
            z,
            r#"<fmiModelDescription fmiVersion="2.0" modelName="M" guid="{{1}}"><CoSimulation modelIdentifier="M"/><ModelVariables>{vars}</ModelVariables></fmiModelDescription>"#
        )
        .unwrap();
        z.finish().unwrap();
        path
    }

    /// 書いて読み戻すと同じ spec になる。変数名の `<`・`&`・`"` は XML で壊れない。
    #[test]
    fn write_then_open_round_trips_with_escaped_names() {
        let dir = tempfile::tempdir().unwrap();
        let var = |n: &str, c: &str, v: &str, ty: &str, start: &str| {
            format!(
                r#"<ScalarVariable name="{}" valueReference="0" causality="{c}" variability="{v}"><{ty}{start}/></ScalarVariable>"#,
                esc(n)
            )
        };
        let (y, u, k, n) = ("y<&\"", "u<&\"", "k&<\"", "n<1");
        let a = fake_fmu(
            dir.path(),
            "a",
            &(var(y, "output", "continuous", "Real", "")
                + &var(k, "parameter", "fixed", "Real", r#" start="1""#)
                + &var(n, "parameter", "fixed", "Integer", r#" start="1""#)),
        );
        let b = fake_fmu(
            dir.path(),
            "b-2",
            &var(u, "input", "continuous", "Real", ""),
        );
        let spec = CosimSpec {
            fmus: [("a".to_owned(), a), ("b-2".to_owned(), b)].into(),
            connections: vec![(format!("a.{y}"), format!("b-2.{u}"))],
            start_time: Some(0.5),
            stop_time: 2.0,
            step: 0.25,
            algorithm: Algorithm::GaussSeidel,
            start: [(format!("a.{k}"), 0.1), (format!("a.{n}"), 3.0)].into(),
            ..Default::default()
        };
        let out = dir.path().join("s.ssp");
        write_ssp(&spec, &out).unwrap();
        let ssp = Ssp::open(&out, None, None).unwrap();
        let got = &ssp.spec;
        assert_eq!(got.connections, spec.connections);
        assert_eq!(got.start, spec.start);
        assert_eq!(
            (got.start_time, got.stop_time, got.step, got.algorithm),
            (Some(0.5), 2.0, 0.25, Algorithm::GaussSeidel)
        );
        assert_eq!(got.fmus.keys().collect::<Vec<_>>(), ["a", "b-2"]);
        // 引数の --step は Annotation に勝つ
        assert_eq!(Ssp::open(&out, None, Some(0.5)).unwrap().spec.step, 0.5);
        // 不正な名前・Real でない結線は書く前に断る
        let mut bad = spec.clone();
        bad.fmus.insert("x.y".into(), bad.fmus["a"].clone());
        assert!(write_ssp(&bad, &out).is_err());
        let mut bad = spec.clone();
        bad.connections = vec![(format!("a.{k}"), format!("b-2.{u}"))];
        assert!(
            write_ssp(&bad, &out)
                .unwrap_err()
                .to_string()
                .contains("output")
        );
    }
}
