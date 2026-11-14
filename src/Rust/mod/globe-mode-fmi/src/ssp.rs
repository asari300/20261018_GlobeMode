//! SSP 2.0.1 の `SystemStructure.ssd` を読む（S253、ADR-0026 の (r)）。XML だけ（zip・FMU には触れない）。
//!
//! 読むのは最小の部分集合: 1 つの `System` の `Elements/Component`（FMU）、`Connections/Connection`、`DefaultExperiment`。
//! `Component` の `ParameterBindings`（`.ssv` の参照かインラインの `ParameterSet`。S263）は Real・Integer・Boolean の値を読む。
//! 範囲外のものは**黙って無視せず**明確な誤りにする（入れ子の `System`・System の階層の `ParameterBindings`・`ParameterMapping`・
//! `SignalDictionary`・`LinearTransformation`・FMU でない `Component`・システム自身のコネクタへの接続・Real 以外のコネクタ・単位の食い違い・
//! 単位つきの値・配列・String / Enumeration の値）。

/// `ssd:Component`（`type` が FMU のもの）。
#[derive(Debug, Clone, PartialEq)]
pub struct SspComponent {
    pub name: String,
    /// `source`（SSP の中の相対 URI。例 `resources/a.fmu`）
    pub source: String,
    /// `ParameterBindings/ParameterBinding`（SSD の順。後ろのものが前のものに勝つ。5.2.3）
    pub bindings: Vec<SspBinding>,
}

/// `ssd:ParameterBinding`。`source`（`.ssv`。SSP の中の相対 URI）か `params`（インラインの値）のどちらか。
#[derive(Debug, Clone, PartialEq)]
pub struct SspBinding {
    pub source: Option<String>,
    /// 名前の前に付ける文字列（無ければ空）
    pub prefix: String,
    /// インラインのとき `(名前, 値)`。Integer・Boolean も f64（Boolean は 0 / 1）
    pub params: Vec<(String, f64)>,
}

/// `ssd:Connection`。start / end はデータの向きではない（SSP 2.0.1 の 5.3.2）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SspConnection {
    pub start_element: String,
    pub start_connector: String,
    pub end_element: String,
    pub end_connector: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ssd {
    pub components: Vec<SspComponent>,
    pub connections: Vec<SspConnection>,
    pub start_time: Option<f64>,
    pub stop_time: Option<f64>,
}

/// 範囲外で、見つけたら断る要素（名前空間は見ない）。
const UNSUPPORTED: [&str; 4] = [
    "ParameterMapping",
    "SignalDictionaries",
    "SignalDictionaryReference",
    "LinearTransformation",
];

fn child<'a, 'i>(n: roxmltree::Node<'a, 'i>, tag: &str) -> Option<roxmltree::Node<'a, 'i>> {
    n.children()
        .find(|c| c.is_element() && c.tag_name().name() == tag)
}

/// `ssv:ParameterSet` の `Parameters/Parameter` を `(名前, 値)` にする（`.ssv` のルートも、インラインの `ParameterValues` の子も同じ）。
fn parameter_set(set: roxmltree::Node) -> Result<Vec<(String, f64)>, String> {
    let mut out = Vec::new();
    for p in child(set, "Parameters")
        .into_iter()
        .flat_map(|p| p.children().filter(|c| c.is_element()))
    {
        let name = p.attribute("name").ok_or("Parameter に name が無い")?;
        let mut kids = p.children().filter(|c| c.is_element());
        let (Some(t), None) = (kids.next(), kids.next()) else {
            return Err(format!(
                "Parameter {name:?} は型の子（Real など）が 1 つでない（配列の Dimension は未対応）"
            ));
        };
        let ty = t.tag_name().name();
        let value = t
            .attribute("value")
            .ok_or_else(|| format!("Parameter {name:?} に value が無い"))?;
        if t.has_attribute("unit") {
            return Err(format!(
                "Parameter {name:?} は単位つき。単位の換算は未対応（範囲外）"
            ));
        }
        let bad = || format!("Parameter {name:?} の値 {value:?} が {ty} として読めない");
        let v = match ty {
            "Real" | "Float64" => value.parse::<f64>().map_err(|_| bad())?,
            "Integer" | "Int32" => f64::from(value.parse::<i32>().map_err(|_| bad())?),
            "Boolean" => match value {
                "true" | "1" => 1.0,
                "false" | "0" => 0.0,
                _ => return Err(bad()),
            },
            _ => {
                return Err(format!(
                    "Parameter {name:?} の型 {ty} は未対応（Real・Integer・Boolean だけ）"
                ));
            }
        };
        out.push((name.to_owned(), v));
    }
    Ok(out)
}

/// `.ssv`（ルートは `ssv:ParameterSet`。SSP 2.0.1 の 6 章）の値。
pub fn parse_ssv(xml: &str) -> Result<Vec<(String, f64)>, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| format!(".ssv を読めない: {e}"))?;
    let root = doc.root_element();
    if root.tag_name().name() != "ParameterSet" {
        return Err(".ssv のルートが ParameterSet でない".into());
    }
    parameter_set(root)
}

fn bindings(component: roxmltree::Node, cname: &str) -> Result<Vec<SspBinding>, String> {
    let mut out = Vec::new();
    for b in child(component, "ParameterBindings")
        .into_iter()
        .flat_map(|b| b.children().filter(|c| c.is_element()))
    {
        let ty = b
            .attribute("type")
            .unwrap_or("application/x-ssp-parameter-set");
        if b.tag_name().name() != "ParameterBinding" || ty != "application/x-ssp-parameter-set" {
            return Err(format!(
                "{cname} の ParameterBindings に未対応のもの（ParameterBinding の ssp-parameter-set だけ）"
            ));
        }
        if b.attribute("sourceBase").is_some_and(|s| s != "SSD") {
            return Err(format!(
                "{cname} の ParameterBinding の sourceBase は SSD だけ対応"
            ));
        }
        let source = b.attribute("source").filter(|s| !s.is_empty());
        let values = child(b, "ParameterValues");
        let params = match (source, values) {
            (None, Some(v)) => parameter_set(
                child(v, "ParameterSet").ok_or("ParameterValues の中に ParameterSet が無い")?,
            )?,
            (Some(_), None) => Vec::new(),
            _ => {
                return Err(format!(
                    "{cname} の ParameterBinding は source か ParameterValues のどちらか 1 つ"
                ));
            }
        };
        out.push(SspBinding {
            source: source.map(str::to_owned),
            prefix: b.attribute("prefix").unwrap_or_default().to_owned(),
            params,
        });
    }
    Ok(out)
}

pub fn parse_ssd(xml: &str) -> Result<Ssd, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| format!("XML を読めない: {e}"))?;
    let root = doc.root_element();
    if root.tag_name().name() != "SystemStructureDescription" {
        return Err("ルートが SystemStructureDescription でない".into());
    }
    for n in doc.descendants().filter(|n| n.is_element()) {
        let tag = n.tag_name().name();
        if UNSUPPORTED.contains(&tag) {
            return Err(format!("未対応の要素 {tag}（範囲外）"));
        }
        if tag == "ParameterBindings"
            && n.parent_element().map(|p| p.tag_name().name()) != Some("Component")
        {
            return Err(
                "System の階層の ParameterBindings は未対応（Component の直下だけ。範囲外）".into(),
            );
        }
    }
    let system = child(root, "System").ok_or("System が無い")?;
    let mut components = Vec::new();
    let mut connectors = std::collections::BTreeMap::new(); // (要素, コネクタ) -> (型, 単位)
    for e in child(system, "Elements")
        .into_iter()
        .flat_map(|e| e.children().filter(|c| c.is_element()))
    {
        let tag = e.tag_name().name();
        let name = e.attribute("name").unwrap_or_default();
        if tag == "System" {
            return Err(format!("入れ子の System {name:?} は未対応（範囲外）"));
        }
        if tag != "Component" {
            return Err(format!("未対応の要素 {tag}"));
        }
        let ty = e
            .attribute("type")
            .unwrap_or("application/x-fmu-sharedlibrary");
        if ty != "application/x-fmu-sharedlibrary" {
            return Err(format!("Component {name:?} の type {ty:?} は FMU でない"));
        }
        let source = e
            .attribute("source")
            .ok_or_else(|| format!("Component {name:?} に source が無い"))?;
        for c in child(e, "Connectors")
            .into_iter()
            .flat_map(|c| c.children().filter(|c| c.is_element()))
        {
            let ty = c.first_element_child().map(|t| t.tag_name().name());
            let unit = c.first_element_child().and_then(|t| t.attribute("unit"));
            connectors.insert(
                (
                    name.to_owned(),
                    c.attribute("name").unwrap_or_default().to_owned(),
                ),
                (ty.unwrap_or_default().to_owned(), unit.map(str::to_owned)),
            );
        }
        components.push(SspComponent {
            name: name.to_owned(),
            source: source.to_owned(),
            bindings: bindings(e, name)?,
        });
    }
    let mut connections = Vec::new();
    for c in child(system, "Connections")
        .into_iter()
        .flat_map(|c| c.children().filter(|c| c.is_element()))
    {
        let get = |a: &str| c.attribute(a).map(str::to_owned);
        let (Some(se), Some(ee)) = (get("startElement"), get("endElement")) else {
            return Err("システム自身のコネクタへの接続は未対応（範囲外）".into());
        };
        let (Some(sc), Some(ec)) = (get("startConnector"), get("endConnector")) else {
            return Err("Connection に startConnector / endConnector が無い".into());
        };
        let end = |e: &String, k: &String| connectors.get(&(e.clone(), k.clone()));
        for (e, k) in [(&se, &sc), (&ee, &ec)] {
            if let Some((ty, _)) = end(e, k)
                && !["Real", "Float64"].contains(&ty.as_str())
            {
                return Err(format!("{e}.{k} は Real でない（{ty}）。Real だけ対応"));
            }
        }
        if let (Some((_, Some(u1))), Some((_, Some(u2)))) = (end(&se, &sc), end(&ee, &ec))
            && u1 != u2
            && c.attribute("suppressUnitConversion") != Some("true")
        {
            return Err(format!(
                "接続 {se}.{sc} -> {ee}.{ec} の単位が違う（{u1} と {u2}）。単位の変換は未対応（範囲外）"
            ));
        }
        connections.push(SspConnection {
            start_element: se,
            start_connector: sc,
            end_element: ee,
            end_connector: ec,
        });
    }
    let time = |a: &str| -> Result<Option<f64>, String> {
        child(root, "DefaultExperiment")
            .and_then(|d| d.attribute(a))
            .map(|v| {
                v.parse()
                    .map_err(|_| format!("DefaultExperiment の {a} {v:?} が数でない"))
            })
            .transpose()
    };
    Ok(Ssd {
        components,
        connections,
        start_time: time("startTime")?,
        stop_time: time("stopTime")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SSD: &str = r#"<?xml version="1.0"?>
<ssd:SystemStructureDescription xmlns:ssd="http://ssp-standard.org/SSP1/SystemStructureDescription"
  xmlns:ssc="http://ssp-standard.org/SSP1/SystemStructureCommon" name="s" version="2.0">
  <ssd:System name="top">
    <ssd:Elements>
      <ssd:Component name="a" source="resources/a.fmu">
        <ssd:Connectors><ssd:Connector name="x" kind="output"><ssc:Real unit="m"/></ssd:Connector></ssd:Connectors>
      </ssd:Component>
      <ssd:Component name="b" source="resources/b.fmu">
        <ssd:Connectors><ssd:Connector name="u" kind="input"><ssc:Real unit="UNIT"/></ssd:Connector></ssd:Connectors>
      </ssd:Component>
    </ssd:Elements>
    <ssd:Connections>
      <ssd:Connection startElement="b" startConnector="u" endElement="a" endConnector="x"/>
    </ssd:Connections>
  </ssd:System>
  <ssd:DefaultExperiment startTime="0" stopTime="2.5"/>
</ssd:SystemStructureDescription>"#;

    #[test]
    fn reads_components_connections_and_experiment() {
        let ssd = parse_ssd(&SSD.replace("UNIT", "m")).unwrap();
        assert_eq!(ssd.components.len(), 2);
        assert_eq!(ssd.components[1].source, "resources/b.fmu");
        assert_eq!(ssd.connections[0].start_element, "b");
        assert_eq!((ssd.start_time, ssd.stop_time), (Some(0.0), Some(2.5)));
    }

    #[test]
    fn reads_ssv_and_binding_attributes() {
        let ssv = |v: &str| {
            parse_ssv(&format!(
                r#"<ssv:ParameterSet xmlns:ssv="x" version="2.0" name="p"><ssv:Parameters>{v}</ssv:Parameters></ssv:ParameterSet>"#
            ))
        };
        let got = ssv(
            r#"<ssv:Parameter name="a"><ssv:Real value="1.5"/></ssv:Parameter>
            <ssv:Parameter name="b"><ssv:Integer value="-3"/></ssv:Parameter>
            <ssv:Parameter name="c"><ssv:Boolean value="true"/></ssv:Parameter>"#,
        );
        assert_eq!(
            got.unwrap(),
            [("a".into(), 1.5), ("b".into(), -3.0), ("c".into(), 1.0)]
        );
        assert!(ssv(r#"<ssv:Parameter name="s"><ssv:String value="x"/></ssv:Parameter>"#).is_err());
        assert!(
            ssv(r#"<ssv:Parameter name="u"><ssv:Real value="1" unit="m"/></ssv:Parameter>"#)
                .unwrap_err()
                .contains("単位")
        );
        let ssd = SSD.replace("UNIT", "m").replace(
            "source=\"resources/a.fmu\">",
            "source=\"resources/a.fmu\"><ssd:ParameterBindings><ssd:ParameterBinding source=\"resources/p.ssv\" prefix=\"q.\"/></ssd:ParameterBindings>",
        );
        let b = &parse_ssd(&ssd).unwrap().components[0].bindings;
        assert_eq!(
            (b[0].source.as_deref(), b[0].prefix.as_str()),
            (Some("resources/p.ssv"), "q.")
        );
    }

    #[test]
    fn refuses_out_of_scope() {
        let nested = SSD.replace("<ssd:Elements>", "<ssd:Elements><ssd:System name=\"in\"/>");
        assert!(parse_ssd(&nested).unwrap_err().contains("入れ子"));
        let bindings = SSD.replace("</ssd:Elements>", "<ssd:ParameterBindings/></ssd:Elements>");
        let mapping = SSD.replace(
            "source=\"resources/a.fmu\">",
            "source=\"resources/a.fmu\"><ssd:ParameterBindings><ssd:ParameterBinding source=\"p.ssv\"><ssd:ParameterMapping/></ssd:ParameterBinding></ssd:ParameterBindings>",
        );
        assert!(
            parse_ssd(&mapping)
                .unwrap_err()
                .contains("ParameterMapping")
        );
        assert!(
            parse_ssd(&bindings)
                .unwrap_err()
                .contains("ParameterBindings")
        );
        let units = SSD.replace("UNIT", "km");
        assert!(parse_ssd(&units).unwrap_err().contains("単位"));
        let int = SSD.replace("ssc:Real unit=\"UNIT\"", "ssc:Integer");
        assert!(parse_ssd(&int).unwrap_err().contains("Real でない"));
        let sys = SSD.replace("startElement=\"b\" ", "");
        assert!(parse_ssd(&sys).unwrap_err().contains("システム自身"));
    }
}
