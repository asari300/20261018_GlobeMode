//! SSP 2.0.1 の `SystemStructure.ssd` を読む（S253、ADR-0026 の (r)）。XML だけ（zip・FMU には触れない）。
//!
//! 読むのは最小の部分集合: `System`（入れ子も。S295）の `Elements/Component`（FMU）、`Connections/Connection`、`DefaultExperiment`。
//! 入れ子の System の Component は `外側.内側` の名前に平らにし、System 自身のコネクタは（System の階層名, コネクタ）の節点にする。
//! `Component`・`System` の `ParameterBindings`（`.ssv` の参照かインラインの `ParameterSet`。S263・S295）は Real・Integer・Boolean の値を読む。
//! 範囲外のものは**黙って無視せず**明確な誤りにする（最上位のシステム自身のコネクタへの接続・`ParameterMapping`・
//! `SignalDictionary`・`LinearTransformation`・FMU でない `Component`・Real・Integer・Boolean 以外のコネクタ・単位の食い違い・
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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ssd {
    /// 入れ子の System の下のものも平らにして `外側.内側` の名前にする（S295）
    pub components: Vec<SspComponent>,
    /// 端の要素は階層名。入れ子の System 自身のコネクタは要素が System の階層名（ [`Ssd::systems`] にある）
    pub connections: Vec<SspConnection>,
    /// 入れ子の System の階層名（ルートは含まない）
    pub systems: Vec<String>,
    /// System の `ParameterBindings`（System の階層名〔ルートは空〕, binding）。名前は階層名（`comp.var`）
    pub system_bindings: Vec<(String, SspBinding)>,
    pub start_time: Option<f64>,
    pub stop_time: Option<f64>,
    /// S274: GlobeMode 独自の `Annotation`（[`COSIM_ANNOTATION`]）の `step`（通信刻み）
    pub cosim_step: Option<f64>,
    /// S274: 同 `algorithm`（`jacobi` か `gauss-seidel`。解釈は呼び手）
    pub cosim_algorithm: Option<String>,
}

/// S274: `globe fmu ssp-export` が SSD の `Annotations` に入れる、通信刻みと結線の解き方（SSD に置き場が無いもの）。
/// SSP の規則どおり、型は自分たちが持つドメインの逆順。中身は `<globe:cosim step=".." algorithm=".."/>`。
pub const COSIM_ANNOTATION: &str = "io.github.asari300.globe-mode.cosim";

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

/// 入れ子の System の深さと名前 1 つの長さの上限（信頼できない `.ssp` で階層名の連結が深さ² のメモリになる。#106）。
/// 実際の SSP は数段（FMU の入れ子は 10 段に届かない）なので、桁に余裕を持たせた値。
const MAX_DEPTH: usize = 64;
pub const MAX_NAME: usize = 1024;
/// 階層名の全体の長さと、階層名を抱える項目（System・Component・コネクタ・接続・System の束縛）の総数の上限。
/// 深さ × 名前の積と、項目数 × 階層名の積を抑える（#112。64 段 × 1 KB の階層名 × 5000 個で RSS 650 MB だった）。
const MAX_PATH: usize = 4096;
const MAX_ITEMS: usize = 10_000;

/// 階層の名前（`外側.内側`）。ルートの System は空。
fn join(path: &str, name: &str) -> String {
    if path.is_empty() {
        name.to_owned()
    } else {
        format!("{path}.{name}")
    }
}

/// `System` を下りながら、Component を平らな名前で集める（S295）。
#[derive(Default)]
struct Walk {
    ssd: Ssd,
    names: std::collections::BTreeSet<String>,
    items: usize,
    /// (要素の階層名, コネクタ) -> (型, 単位)。System 自身のコネクタは (System の階層名, コネクタ)
    connectors: std::collections::BTreeMap<(String, String), (String, Option<String>)>,
}

impl Walk {
    fn item(&mut self) -> Result<(), String> {
        self.items += 1;
        if self.items > MAX_ITEMS {
            return Err(format!("要素の数が多すぎる（{MAX_ITEMS} 個まで）"));
        }
        Ok(())
    }

    fn connectors(&mut self, node: roxmltree::Node, owner: &str) -> Result<(), String> {
        for c in child(node, "Connectors")
            .into_iter()
            .flat_map(|c| c.children().filter(|c| c.is_element()))
        {
            self.item()?;
            let ty = c.first_element_child().map(|t| t.tag_name().name());
            let unit = c.first_element_child().and_then(|t| t.attribute("unit"));
            self.connectors.insert(
                (
                    owner.to_owned(),
                    c.attribute("name").unwrap_or_default().to_owned(),
                ),
                (ty.unwrap_or_default().to_owned(), unit.map(str::to_owned)),
            );
        }
        Ok(())
    }

    fn system(&mut self, system: roxmltree::Node, path: &str, depth: usize) -> Result<(), String> {
        if depth > MAX_DEPTH {
            return Err(format!("System の入れ子が深すぎる（{MAX_DEPTH} 段まで）"));
        }
        if !path.is_empty() {
            self.ssd.systems.push(path.to_owned());
            self.connectors(system, path)?;
        }
        for b in bindings(system, if path.is_empty() { "System" } else { path })? {
            self.item()?;
            self.ssd.system_bindings.push((path.to_owned(), b));
        }
        for e in child(system, "Elements")
            .into_iter()
            .flat_map(|e| e.children().filter(|c| c.is_element()))
        {
            let tag = e.tag_name().name();
            let local = e.attribute("name").unwrap_or_default();
            if local.len() > MAX_NAME {
                return Err(format!("要素の名前が長すぎる（{MAX_NAME} バイトまで）"));
            }
            let name = join(path, local);
            if name.len() > MAX_PATH {
                return Err(format!("階層名が長すぎる（{MAX_PATH} バイトまで）"));
            }
            self.item()?;
            if tag != "Component" && tag != "System" {
                return Err(format!("未対応の要素 {tag}"));
            }
            if !self.names.insert(name.clone()) {
                return Err(format!("要素の名前 {name:?} が重複"));
            }
            if tag == "System" {
                self.system(e, &name, depth + 1)?;
                continue;
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
            self.connectors(e, &name)?;
            self.ssd.components.push(SspComponent {
                bindings: bindings(e, &name)?,
                name,
                source: source.to_owned(),
            });
        }
        for c in child(system, "Connections")
            .into_iter()
            .flat_map(|c| c.children().filter(|c| c.is_element()))
        {
            self.item()?;
            // 要素を省くとこの System 自身のコネクタ（入れ子の System では外側から見たのと同じ節点）
            let own = |a: &str| match c.attribute(a) {
                Some(e) => Ok(join(path, e)),
                None if path.is_empty() => {
                    Err("最上位のシステム自身のコネクタへの接続は未対応（範囲外）".to_owned())
                }
                None => Ok(path.to_owned()),
            };
            let (se, ee) = (own("startElement")?, own("endElement")?);
            if se.len().max(ee.len()) > MAX_PATH {
                return Err(format!("階層名が長すぎる（{MAX_PATH} バイトまで）"));
            }
            let (Some(sc), Some(ec)) = (
                c.attribute("startConnector").map(str::to_owned),
                c.attribute("endConnector").map(str::to_owned),
            ) else {
                return Err("Connection に startConnector / endConnector が無い".into());
            };
            let end = |e: &String, k: &String| self.connectors.get(&(e.clone(), k.clone()));
            for (e, k) in [(&se, &sc), (&ee, &ec)] {
                if let Some((ty, _)) = end(e, k)
                    && !["Real", "Float64", "Integer", "Int32", "Boolean"].contains(&ty.as_str())
                {
                    return Err(format!(
                        "{e}.{k} は Real・Integer・Boolean のどれでもない（{ty}）。この 3 つだけ対応"
                    ));
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
            self.ssd.connections.push(SspConnection {
                start_element: se,
                start_connector: sc,
                end_element: ee,
                end_connector: ec,
            });
        }
        Ok(())
    }
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
            && !matches!(
                n.parent_element().map(|p| p.tag_name().name()),
                Some("Component" | "System")
            )
        {
            return Err("ParameterBindings は Component か System の直下だけ（範囲外）".into());
        }
    }
    let system = child(root, "System").ok_or("System が無い")?;
    let mut walk = Walk::default();
    walk.system(system, "", 0)?;
    let Walk { mut ssd, .. } = walk;
    let time = |a: &str| -> Result<Option<f64>, String> {
        child(root, "DefaultExperiment")
            .and_then(|d| d.attribute(a))
            .map(|v| {
                v.parse()
                    .map_err(|_| format!("DefaultExperiment の {a} {v:?} が数でない"))
            })
            .transpose()
    };
    let cosim = child(root, "Annotations")
        .into_iter()
        .flat_map(|a| a.children())
        .filter(|a| a.is_element() && a.attribute("type") == Some(COSIM_ANNOTATION))
        .find_map(|a| child(a, "cosim"));
    let cosim_step = cosim
        .and_then(|c| c.attribute("step"))
        .map(|v| {
            v.parse()
                .map_err(|_| format!("Annotation の step {v:?} が数でない"))
        })
        .transpose()?;
    ssd.cosim_step = cosim_step;
    ssd.cosim_algorithm = cosim
        .and_then(|c| c.attribute("algorithm"))
        .map(str::to_owned);
    ssd.start_time = time("startTime")?;
    ssd.stop_time = time("stopTime")?;
    Ok(ssd)
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
        let string = SSD.replace("ssc:Real unit=\"UNIT\"", "ssc:String");
        assert!(parse_ssd(&string).unwrap_err().contains("どれでもない"));
        let sys = SSD.replace("startElement=\"b\" ", "");
        assert!(parse_ssd(&sys).unwrap_err().contains("システム自身"));
    }

    /// S295: 入れ子の System は平らな名前になり、System 自身のコネクタは外と内で同じ節点になる。
    #[test]
    fn flattens_nested_systems() {
        let ssd = r#"<ssd:SystemStructureDescription xmlns:ssd="x" xmlns:ssc="y" name="s" version="2.0">
  <ssd:System name="top"><ssd:Elements>
    <ssd:Component name="a" source="resources/a.fmu"/>
    <ssd:System name="g">
      <ssd:Connectors><ssd:Connector name="i" kind="input"><ssc:Integer/></ssd:Connector></ssd:Connectors>
      <ssd:Elements><ssd:Component name="b" source="resources/b.fmu"/></ssd:Elements>
      <ssd:Connections><ssd:Connection startConnector="i" endElement="b" endConnector="u"/></ssd:Connections>
    </ssd:System>
  </ssd:Elements>
  <ssd:Connections><ssd:Connection startElement="a" startConnector="y" endElement="g" endConnector="i"/></ssd:Connections>
  </ssd:System></ssd:SystemStructureDescription>"#;
        let got = parse_ssd(ssd).unwrap();
        let names: Vec<_> = got.components.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            (names, got.systems),
            (["a", "g.b"].to_vec(), vec!["g".to_owned()])
        );
        let ends: Vec<_> = got
            .connections
            .iter()
            .map(|c| (c.start_element.as_str(), c.end_element.as_str()))
            .collect();
        assert_eq!(ends, [("g", "g.b"), ("a", "g")]);
        let dup = ssd.replace("name=\"g\"", "name=\"a\"");
        assert!(parse_ssd(&dup).unwrap_err().contains("重複"));
    }

    /// #106: 入れ子の深さと名前の長さに上限（階層名の連結が深さ² のメモリにならない）。
    #[test]
    fn nested_system_depth_and_name_are_capped() {
        let nest = |depth: usize, name: &str| {
            let mut s = String::new();
            for _ in 0..depth {
                s += &format!("<ssd:System name=\"{name}\"><ssd:Elements>");
            }
            for _ in 0..depth {
                s += "</ssd:Elements></ssd:System>";
            }
            SSD.replace("UNIT", "m")
                .replacen("</ssd:Elements>", &format!("{s}</ssd:Elements>"), 1)
        };
        parse_ssd(&nest(MAX_DEPTH, "s")).unwrap();
        assert!(
            parse_ssd(&nest(MAX_DEPTH + 1, "s"))
                .unwrap_err()
                .contains("深すぎる")
        );
        assert!(
            parse_ssd(&nest(1, &"x".repeat(MAX_NAME + 1)))
                .unwrap_err()
                .contains("長すぎる")
        );
    }

    /// #112: 深さ 64 × 名前 1 KB の最深部に Component を並べる積と、項目の総数に上限。
    #[test]
    fn hierarchy_name_length_and_item_count_are_capped() {
        let wrap = |inner: String| {
            SSD.replace("UNIT", "m").replacen(
                "</ssd:Elements>",
                &format!("{inner}</ssd:Elements>"),
                1,
            )
        };
        let comp = |i: usize| {
            format!(
                "<ssd:Component name=\"c{i}\" type=\"application/x-fmu-sharedlibrary\" source=\"resources/a.fmu\"/>"
            )
        };
        let deep = "x".repeat(MAX_NAME);
        let mut s = String::new();
        for _ in 0..MAX_DEPTH {
            s += &format!("<ssd:System name=\"{deep}\"><ssd:Elements>");
        }
        s += &(0..5).map(comp).collect::<String>();
        for _ in 0..MAX_DEPTH {
            s += "</ssd:Elements></ssd:System>";
        }
        assert!(
            parse_ssd(&wrap(s))
                .unwrap_err()
                .contains("階層名が長すぎる")
        );
        let many = (0..=MAX_ITEMS).map(comp).collect::<String>();
        assert!(parse_ssd(&wrap(many)).unwrap_err().contains("多すぎる"));
    }
}
