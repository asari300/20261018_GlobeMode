//! `.ssp`（SSP 2.0.1。S253、ADR-0026 の (r)）を [`CosimSpec`] にする。回すのは S249 のマスターそのまま。
//!
//! `.ssp` を一時ディレクトリに展開し（zip slip・サイズの上限は FMU と同じ `unpack`）、
//! `SystemStructure.ssd` の `Component`（入れ子の System の下は `外側.内側`。S295）を `fmus`、`Connection`（System のコネクタを通るものは 1 本にたどる）を `connections` にする。
//! 通信刻みは SSD に無いので呼び手が渡す（S274 の `write_ssp` が書いた `.ssp` なら、独自の `Annotation` から読む）。`Component` の `ParameterBindings` は `start` にする（S263）。

use std::path::{Component, Path};

use std::collections::BTreeMap;
use std::fmt::Write as _;

use globe_mode_fmi::ssp::{COSIM_ANNOTATION, MAX_NAME, SspBinding, parse_ssd, parse_ssv};
use globe_mode_fmi::{Causality, VarType};

use crate::cosim::valid_fmu_path;
use crate::{
    Algorithm, CosimSpec, Error, Result, io, read_model_description, read_xml_file, unpack,
};

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
        let ssd = read_xml_file(&dir.path().join("SystemStructure.ssd"))
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
            if spec
                .fmus
                .insert(c.name.clone(), dir.path().join(&c.source))
                .is_some()
            {
                // 後ろが前を黙って消し、ParameterBindings は両方が後ろの FMU に入る（S288）
                return Err(Error::Ssp(format!("Component の名前 {:?} が重複", c.name)));
            }
        }
        // modelDescription は FMU ごとに 1 回だけ読む（結線の本数に比例して読み直さない。S288）。読めない FMU は実行のときに誤りになる
        let descs: BTreeMap<&String, _> = spec
            .fmus
            .iter()
            .map(|(n, p)| (n, read_model_description(p).ok()))
            .collect();
        // start / end はデータの向きではない（SSP 5.3.2）。FMU の causality で「出力 → 入力」に揃える
        let causality = |e: &str, v: &str| -> Option<Causality> {
            descs
                .get(&e.to_string())?
                .as_ref()?
                .variables
                .iter()
                .find(|x| x.name == v)
                .map(|x| x.causality)
        };
        // 入れ子の System のコネクタは素通し（S295）: 結線を節点（要素, コネクタ）の網にまとめ、網ごとに出力 1 本 → 残りの入力へ
        let mut ids: BTreeMap<(&str, &str), usize> = BTreeMap::new();
        let mut parent: Vec<usize> = Vec::new();
        let mut edges = Vec::new();
        for c in &ssd.connections {
            let mut end = [0; 2];
            for (n, (e, k)) in end.iter_mut().zip([
                (c.start_element.as_str(), c.start_connector.as_str()),
                (c.end_element.as_str(), c.end_connector.as_str()),
            ]) {
                if !spec.fmus.contains_key(e) && !ssd.systems.iter().any(|s| s == e) {
                    return Err(Error::Ssp(format!("接続の端 {e}.{k} の要素が無い")));
                }
                *n = *ids.entry((e, k)).or_insert_with(|| {
                    parent.push(parent.len());
                    parent.len() - 1
                });
            }
            edges.push((end[0], end[1]));
        }
        fn find(p: &mut [usize], mut x: usize) -> usize {
            while p[x] != x {
                p[x] = p[p[x]];
                x = p[x];
            }
            x
        }
        for (a, b) in edges {
            let (a, b) = (find(&mut parent, a), find(&mut parent, b));
            parent[a] = b;
        }
        // 網 → FMU の端（節点の番号の順 = SSD に現れた順）
        let mut nets: BTreeMap<usize, Vec<(&str, &str)>> = BTreeMap::new();
        let mut by_id: Vec<_> = ids.iter().collect();
        by_id.sort_by_key(|(_, i)| **i);
        for (&(e, k), &i) in by_id {
            if spec.fmus.contains_key(e) {
                nets.entry(find(&mut parent, i)).or_default().push((e, k));
            }
        }
        let mut pairs: Vec<(usize, String, String)> = Vec::new();
        for ends in nets.values() {
            let outs: Vec<_> = ends
                .iter()
                .filter(|(e, k)| causality(e, k) == Some(Causality::Output))
                .collect();
            let name = |(e, k): &(&str, &str)| format!("{e}.{k}");
            if ends.len() < 2 {
                return Err(Error::Ssp(format!(
                    "{} の接続は FMU の端が 1 つで終わっている",
                    name(&ends[0])
                )));
            }
            let src = outs.first().copied().unwrap_or(&ends[0]);
            if outs.len() > 1 {
                return Err(Error::Ssp(format!(
                    "{} と {} が同じ入力につながる（出力は 1 本だけ）",
                    name(outs[0]),
                    name(outs[1])
                )));
            }
            for end in ends.iter().filter(|e| *e != src) {
                // 先（入力）の並びは SSD に現れた順のまま。Gauss–Seidel の順に効く
                pairs.push((ids[end], name(src), name(end)));
            }
        }
        pairs.sort();
        spec.connections = pairs.into_iter().map(|(_, a, b)| (a, b)).collect();
        // ParameterBindings → start（5.2.3: 同じ階層は後ろの binding が勝ち、階層が違えば上位の System が勝つ。
        // FMU に無い名前は無視する）。深い階層から先に適用して、上位に上書きさせる
        let scoped: Vec<(&str, &SspBinding)> = ssd
            .system_bindings
            .iter()
            .map(|(p, b)| (p.as_str(), b))
            .chain(
                ssd.components
                    .iter()
                    .flat_map(|c| c.bindings.iter().map(|b| (c.name.as_str(), b))),
            )
            .collect();
        let depth = |p: &str| {
            if p.is_empty() {
                0
            } else {
                p.split('.').count()
            }
        };
        let mut order: Vec<usize> = (0..scoped.len()).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(depth(scoped[i].0)));
        let mut ssv: BTreeMap<&str, _> = BTreeMap::new();
        for i in order {
            let (scope, b) = scoped[i];
            let params = match &b.source {
                None => b.params.clone(),
                Some(s) if is_inside_relative(s) => {
                    // 同じ source を指す binding は 1 回だけ読んで解析する（#105）
                    if !ssv.contains_key(s.as_str()) {
                        let xml = read_xml_file(&dir.path().join(s)).map_err(|e| {
                            Error::Ssp(format!(
                                "{scope:?} の ParameterBinding {s:?} を読めない: {e}"
                            ))
                        })?;
                        let p = parse_ssv(&xml).map_err(|m| Error::Ssp(format!("{s}: {m}")))?;
                        ssv.insert(s.as_str(), p);
                    }
                    ssv[s.as_str()].clone()
                }
                Some(s) => {
                    return Err(Error::Ssp(format!(
                        "{scope:?} の ParameterBinding の source {s:?} は SSP の中の相対パスでない"
                    )));
                }
            };
            for (n, v) in params {
                // System の binding の名前は階層名（`comp.var`）、Component のは変数名
                let full = if scope.is_empty() {
                    format!("{}{n}", b.prefix)
                } else {
                    format!("{scope}.{}{n}", b.prefix)
                };
                for (f, d) in &descs {
                    let Some(d) = d else { continue };
                    if let Some(var) = full
                        .strip_prefix(f.as_str())
                        .and_then(|r| r.strip_prefix('.'))
                        && d.variables.iter().any(|x| x.name == var)
                    {
                        spec.start.insert(format!("{f}.{var}"), v);
                    }
                }
            }
        }
        Ok(Ssp { _dir: dir, spec })
    }

    /// 展開した FMU を `dir/<名前>.fmu` に写し、それを指す spec にする（パスは `<名前>.fmu`。spec.json を `dir` に置く前提。S315）。
    /// `Ssp` は drop で展開先を消すので、JSON に残すには FMU を外に出すしかない。既にあるファイルは上書きしない。
    pub fn export_spec(&self, dir: &Path) -> Result<CosimSpec> {
        let mut spec = self.spec.clone();
        for name in spec.fmus.keys() {
            // 名前は信頼できない入力: `..`・`/` でフォルダの外に書かせない
            if !valid_fmu_path(name) {
                return Err(Error::Ssp(format!("Component の名前 {name:?} が不正")));
            }
            let to = dir.join(format!("{name}.fmu"));
            if to.symlink_metadata().is_ok() {
                return Err(Error::Ssp(format!(
                    "{} が既にある（上書きしない）",
                    to.display()
                )));
            }
        }
        for (name, p) in &mut spec.fmus {
            let file = format!("{name}.fmu");
            std::fs::copy(&*p, dir.join(&file)).map_err(io)?;
            *p = file.into();
        }
        Ok(spec)
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
/// 結線は Float64（`ssc:Real`）・Int32（`ssc:Integer`）・Boolean（`ssc:Boolean`）で、両端は同じ型（S295）。
/// `"a.x"` を（FMU の名前, 変数名, 変数）にする。
fn var_of<'a>(
    descs: &'a BTreeMap<&String, globe_mode_fmi::ModelDescription>,
    key: &'a str,
) -> Result<(&'a str, &'a str, &'a globe_mode_fmi::ScalarVariable)> {
    let err = |m: String| Error::Ssp(m);
    // FMU の名前は `.` を含んでよい（S315）ので、一番長く合う名前で割る（`cosim.rs` の `endpoint` と同じ）
    let (f, v) = descs
        .keys()
        .filter_map(|f| Some((f.as_str(), key.strip_prefix(f.as_str())?.strip_prefix('.')?)))
        .max_by_key(|(f, _)| f.len())
        .ok_or_else(|| {
            err(format!(
                "{key:?} が `<FMU の名前>.<変数名>` の形でないか、FMU が `fmus` に無い"
            ))
        })?;
    let d = &descs[&f.to_owned()];
    let x = d
        .variables
        .iter()
        .find(|x| x.name == v)
        .ok_or_else(|| err(format!("{key:?}: 変数 {v:?} が無い")))?;
    Ok((f, v, x))
}

pub fn write_ssp(spec: &CosimSpec, out: &Path) -> Result<()> {
    write_ssp_with(spec, out, false)
}

/// [`write_ssp`] の `ssv` 版（S318）。`ssv` が真なら `start` を Component ごとの `resources/<名前>.ssv` に書き、
/// `ParameterBinding source="resources/<名前>.ssv"` で参照する（偽ならインラインの `ParameterValues`）。読み戻した結果は同じ。
pub fn write_ssp_with(spec: &CosimSpec, out: &Path, ssv: bool) -> Result<()> {
    use std::io::Write;
    let err = |m: String| Error::Ssp(m);
    let mut descs = BTreeMap::new();
    for (name, path) in &spec.fmus {
        // `.` を含む名前（入れ子を平らにしたもの）は平らな Component の名前のまま書く（S315）。SSD の `name` は
        // `xs:string`（minLength 1 だけ）で `.` を禁じていない。読み取りは名前を 1 つの文字列として扱う
        if !valid_fmu_path(name) || name.len() > MAX_NAME {
            return Err(err(format!(
                "FMU の名前 {name:?} が不正（`.` でつないだ英数字・`_`・`-` だけ、{MAX_NAME} バイトまで）"
            )));
        }
        descs.insert(name, read_model_description(path)?);
    }
    // コンポーネントごとの (変数名 → ("input" | "output", SSP の型の要素))
    let mut connectors: BTreeMap<&str, BTreeMap<&str, (&str, &str)>> = BTreeMap::new();
    let mut connections = String::new();
    for (a, b) in &spec.connections {
        let mut ends = Vec::new();
        let mut types = Vec::new();
        for (key, want, kind) in [
            (a, Causality::Output, "output"),
            (b, Causality::Input, "input"),
        ] {
            let (f, v, x) = var_of(&descs, key)?;
            let tag = match x.ty {
                VarType::Float64 => "Real",
                VarType::Int32 => "Integer",
                VarType::Boolean => "Boolean",
                _ => "",
            };
            if x.causality != want || tag.is_empty() || x.dimensions {
                return Err(err(format!(
                    "接続 {a} -> {b}: {key} は {kind} の Float64・Int32・Boolean（スカラー）でない"
                )));
            }
            connectors.entry(f).or_default().insert(v, (kind, tag));
            ends.push((esc(f), esc(v)));
            types.push(tag);
        }
        if types[0] != types[1] {
            return Err(err(format!(
                "接続 {a} -> {b}: 型が違う（{} と {}）",
                types[0], types[1]
            )));
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
    let mut ssv_files: Vec<(String, String)> = Vec::new();
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
            for (v, (kind, tag)) in c {
                writeln!(ssd, r#"          <ssd:Connector name="{}" kind="{kind}"><ssc:{tag}/></ssd:Connector>"#, esc(v)).unwrap();
            }
            ssd.push_str("        </ssd:Connectors>\n");
        }
        if let Some(p) = starts.get(name.as_str()) {
            if ssv {
                // 型の要素（Real・Integer・Boolean）は ssv の名前空間（XSD は elementFormDefault="qualified"）
                ssv_files.push((format!("resources/{name}.ssv"), format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<ssv:ParameterSet xmlns:ssv=\"http://ssp-standard.org/SSP1/SystemStructureParameterValues\" version=\"2.0\" name=\"start\">\n  <ssv:Parameters>\n{p}  </ssv:Parameters>\n</ssv:ParameterSet>\n")));
                writeln!(ssd, "        <ssd:ParameterBindings><ssd:ParameterBinding source=\"resources/{n}.ssv\"/></ssd:ParameterBindings>").unwrap();
            } else {
                write!(ssd, "        <ssd:ParameterBindings><ssd:ParameterBinding><ssd:ParameterValues>\n          <ssv:ParameterSet version=\"2.0\" name=\"start\"><ssv:Parameters>\n{p}          </ssv:Parameters></ssv:ParameterSet>\n        </ssd:ParameterValues></ssd:ParameterBinding></ssd:ParameterBindings>\n").unwrap();
            }
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
        for (path, text) in &ssv_files {
            zip.start_file(path, zip::write::SimpleFileOptions::default())
                .map_err(crate::zip_err)?;
            zip.write_all(text.as_bytes()).map_err(io)?;
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
        bad.fmus.insert("x y".into(), bad.fmus["a"].clone());
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

    /// S318: `write_ssp_with(.., true)` は start を `resources/<名前>.ssv` に書く。読み戻すとインライン版と同じ spec で、
    /// `.ssv` は ssv の名前空間・`ssv:Real`・`ssv:Integer`・`ssv:Boolean` の要素になる（型は FMU の変数から）。
    #[test]
    fn ssv_export_reads_back_like_inline() {
        let dir = tempfile::tempdir().unwrap();
        let var = |n: &str, ty: &str, start: &str| {
            format!(
                r#"<ScalarVariable name="{n}" valueReference="0" causality="parameter" variability="fixed"><{ty} start="{start}"/></ScalarVariable>"#
            )
        };
        let a = fake_fmu(
            dir.path(),
            "a",
            &(var("k", "Real", "1") + &var("n", "Integer", "1") + &var("f", "Boolean", "false")),
        );
        let b = fake_fmu(dir.path(), "b", &var("k", "Real", "1"));
        let spec = CosimSpec {
            fmus: [("a".to_owned(), a), ("b".to_owned(), b)].into(),
            stop_time: 1.0,
            step: 0.5,
            start: [
                ("a.k".to_owned(), 0.1),
                ("a.n".to_owned(), 3.0),
                ("a.f".to_owned(), 1.0),
            ]
            .into(),
            ..Default::default()
        };
        let (inline, ssv) = (dir.path().join("i.ssp"), dir.path().join("v.ssp"));
        write_ssp_with(&spec, &inline, false).unwrap();
        write_ssp_with(&spec, &ssv, true).unwrap();
        let got = Ssp::open(&ssv, None, None).unwrap().spec;
        assert_eq!(
            got.start,
            Ssp::open(&inline, None, None).unwrap().spec.start
        );
        assert_eq!(got.start, spec.start);
        let mut z = zip::ZipArchive::new(std::fs::File::open(&ssv).unwrap()).unwrap();
        let mut text = String::new();
        std::io::Read::read_to_string(&mut z.by_name("resources/a.ssv").unwrap(), &mut text)
            .unwrap();
        assert!(
            text.contains(
                r#"xmlns:ssv="http://ssp-standard.org/SSP1/SystemStructureParameterValues""#
            ),
            "{text}"
        );
        for want in [
            r#"<ssv:Real value="0.1"/>"#,
            r#"<ssv:Integer value="3"/>"#,
            r#"<ssv:Boolean value="true"/>"#,
        ] {
            assert!(text.contains(want), "{text}");
        }
        // start の無い Component には .ssv を作らない
        assert!(z.by_name("resources/b.ssv").is_err());
    }

    /// S315: 名前に `.` を含む FMU（入れ子を平らにしたもの）は平らな Component のまま書いて読み戻せ、
    /// `export_spec` で FMU を外に出した spec は（パス以外）元と同じ。
    #[test]
    fn dotted_names_stay_flat_and_export_spec_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let var = |n: &str, c: &str| {
            format!(
                r#"<ScalarVariable name="{n}" valueReference="0" causality="{c}" variability="continuous"><Real/></ScalarVariable>"#
            )
        };
        let a = fake_fmu(dir.path(), "a", &var("y", "output"));
        let b = fake_fmu(dir.path(), "b", &var("u", "input"));
        let spec = CosimSpec {
            fmus: [("s.a".to_owned(), a), ("s.b".to_owned(), b)].into(),
            connections: vec![("s.a.y".into(), "s.b.u".into())],
            stop_time: 1.0,
            step: 0.5,
            ..Default::default()
        };
        let out = dir.path().join("s.ssp");
        write_ssp(&spec, &out).unwrap();
        let ssp = Ssp::open(&out, None, None).unwrap();
        let to = dir.path().join("out");
        std::fs::create_dir(&to).unwrap();
        let got = ssp.export_spec(&to).unwrap();
        assert_eq!(got.connections, spec.connections);
        assert_eq!(
            (got.start_time, got.stop_time, got.step),
            (Some(0.0), 1.0, 0.5)
        );
        assert_eq!(got.fmus.keys().collect::<Vec<_>>(), ["s.a", "s.b"]);
        assert_eq!(got.fmus["s.a"], std::path::Path::new("s.a.fmu"));
        assert_eq!(
            std::fs::read(to.join("s.b.fmu")).unwrap(),
            std::fs::read(&spec.fmus["s.b"]).unwrap()
        );
        // 上書きしない
        assert!(ssp.export_spec(&to).is_err());
    }

    /// S295: Int32・Boolean の結線も書いて読み戻せる。型が違う結線は書く前に断る。
    #[test]
    fn integer_and_boolean_wires_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let var = |n: &str, c: &str, ty: &str| {
            format!(
                r#"<ScalarVariable name="{n}" valueReference="0" causality="{c}" variability="discrete"><{ty}/></ScalarVariable>"#
            )
        };
        let a = fake_fmu(
            dir.path(),
            "a",
            &(var("i", "output", "Integer") + &var("b", "output", "Boolean")),
        );
        let b = fake_fmu(
            dir.path(),
            "b",
            &(var("i", "input", "Integer") + &var("b", "input", "Boolean")),
        );
        let mut spec = CosimSpec {
            fmus: [("a".to_owned(), a), ("b".to_owned(), b)].into(),
            connections: vec![("a.i".into(), "b.i".into()), ("a.b".into(), "b.b".into())],
            stop_time: 1.0,
            step: 0.5,
            ..Default::default()
        };
        let out = dir.path().join("s.ssp");
        write_ssp(&spec, &out).unwrap();
        assert_eq!(
            Ssp::open(&out, None, None).unwrap().spec.connections,
            spec.connections
        );
        spec.connections = vec![("a.i".into(), "b.b".into())];
        assert!(
            write_ssp(&spec, &out)
                .unwrap_err()
                .to_string()
                .contains("型が違う")
        );
    }

    /// S288 (#102): 同名の Component・同じ入力への複数の結線は黙って最後を採らず断る
    #[test]
    fn duplicate_component_names_and_inputs_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let var = |n: &str, c: &str| {
            format!(
                r#"<ScalarVariable name="{n}" valueReference="0" causality="{c}" variability="continuous"><Real/></ScalarVariable>"#
            )
        };
        let a = fake_fmu(
            dir.path(),
            "a",
            &(var("y1", "output") + &var("y2", "output")),
        );
        let b = fake_fmu(dir.path(), "b", &var("u", "input"));
        let spec = CosimSpec {
            fmus: [("a".to_owned(), a.clone()), ("b".to_owned(), b)].into(),
            connections: vec![("a.y1".into(), "b.u".into()), ("a.y2".into(), "b.u".into())],
            stop_time: 1.0,
            step: 0.5,
            ..Default::default()
        };
        let e = crate::cosim::run_cosim_with(&spec, None, &mut |_| {}).unwrap_err();
        assert!(e.to_string().contains("別の結線がもうある"), "{e}");
        // 同名の Component を持つ .ssp
        let out = dir.path().join("dup.ssp");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&out).unwrap());
        z.start_file(
            "SystemStructure.ssd",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        let c = |s: &str| {
            format!(
                r#"<ssd:Component name="a" type="application/x-fmu-sharedlibrary" source="resources/{s}.fmu"/>"#
            )
        };
        write!(z, r#"<ssd:SystemStructureDescription xmlns:ssd="http://ssp-standard.org/SSP1/SystemStructureDescription" name="s" version="2.0"><ssd:System name="s"><ssd:Elements>{}{}</ssd:Elements></ssd:System><ssd:DefaultExperiment startTime="0" stopTime="1"/></ssd:SystemStructureDescription>"#, c("a"), c("b")).unwrap();
        z.finish().unwrap();
        let e = Ssp::open(&out, None, None).unwrap_err();
        assert!(e.to_string().contains("重複"), "{e}");
    }
}
