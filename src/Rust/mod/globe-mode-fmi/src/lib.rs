//! FMI 2.0 / 3.0 の `modelDescription.xml` を読む（S057、P5 の準備。[ADR-0026]）。
//!
//! 今は XML を変数表にするだけ。共有ライブラリの読み込み（`libloading`）とマスターは P5 で足す。
//! XML だけなので wasm32 でも通る（ブラウザで FMU の中身を見る用途）。
//!
//! [ADR-0026]: https://github.com/asari300/20261018_GlobeMode/blob/main/doc/adr/0026-fmi-import-placement.md

use std::fmt;
use std::fmt::Write as _;

#[cfg(feature = "export")]
pub mod export;

/// 読み取りの失敗。
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// XML として壊れている。
    Xml(String),
    /// ルートが `fmiModelDescription` でない、または必須の属性が無い・読めない。
    Schema(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Xml(m) => write!(f, "XML を読めない: {m}"),
            Error::Schema(m) => write!(f, "modelDescription.xml が不正: {m}"),
        }
    }
}

impl std::error::Error for Error {}

/// `ModelExchange` / `CoSimulation` / `ScheduledExecution`（3.0 のみ）の要素。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface {
    /// 共有ライブラリの名前（拡張子なし）。
    pub model_identifier: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Causality {
    Parameter,
    CalculatedParameter,
    Input,
    Output,
    Local,
    Independent,
    /// 3.0 のみ。
    StructuralParameter,
}

/// 属性 `initial`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Initial {
    Exact,
    Approx,
    Calculated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variability {
    Constant,
    Fixed,
    Tunable,
    Discrete,
    Continuous,
}

/// 変数の型。2.0 の `Real` は `Float64`、`Integer` は `Int32` に写す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarType {
    Float32,
    Float64,
    Int8,
    UInt8,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Int64,
    UInt64,
    Boolean,
    String,
    /// 3.0 のみ。
    Binary,
    Enumeration,
    /// 3.0 のみ。
    Clock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScalarVariable {
    pub name: String,
    pub value_reference: u32,
    pub causality: Causality,
    pub variability: Variability,
    pub ty: VarType,
    /// S117: 省略されていれば `None`（既定の initial は補わない）。
    pub initial: Option<Initial>,
    /// S117: 属性 `start` の文字列そのまま（型によらず書き戻せるように）。
    pub start: Option<String>,
    /// S117: 属性 `derivative`。2.0 は状態の索引（1 始まり）、3.0 は状態の valueReference。
    pub derivative: Option<u32>,
}

/// `DefaultExperiment` の属性（文字列そのまま）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DefaultExperiment {
    pub start_time: Option<String>,
    pub stop_time: Option<String>,
    pub tolerance: Option<String>,
}

/// `ModelStructure`（S117）。2.0 は `Unknown` の `index`（変数の 1 始まりの索引）、3.0 は
/// `Output`・`ContinuousStateDerivative`・`InitialUnknown` の `valueReference`。`dependencies` は読まない。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelStructure {
    pub outputs: Vec<u32>,
    pub derivatives: Vec<u32>,
    pub initial_unknowns: Vec<u32>,
    /// S137: 3.0 の `EventIndicator` の `valueReference`（2.0 は持たず、読み書きでは空のまま）。
    pub event_indicators: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDescription {
    /// 属性 `fmiVersion` の文字列（"2.0"、"3.0" など）。
    pub fmi_version: String,
    pub model_name: String,
    /// 2.0 は `guid`、3.0 は `instantiationToken`。
    pub instantiation_token: String,
    pub model_exchange: Option<Interface>,
    pub co_simulation: Option<Interface>,
    /// 3.0 のみ。
    pub scheduled_execution: Option<Interface>,
    pub variables: Vec<ScalarVariable>,
    /// S117 から下は書く側（[`write_model_description_2`]）のために読む。
    pub generation_tool: Option<String>,
    pub variable_naming_convention: Option<String>,
    /// 2.0 は属性（省略されていれば 0）、3.0 は `ModelStructure` の `EventIndicator` の数（S124）。
    pub number_of_event_indicators: u32,
    pub default_experiment: Option<DefaultExperiment>,
    pub model_structure: ModelStructure,
}

impl ModelDescription {
    /// 主版（2 または 3）。`fmiVersion` が "2." / "3." で始まらなければ `None`。
    pub fn major(&self) -> Option<u8> {
        match self.fmi_version.split('.').next()? {
            "2" => Some(2),
            "3" => Some(3),
            _ => None,
        }
    }
}

fn schema<T>(m: impl Into<String>) -> Result<T, Error> {
    Err(Error::Schema(m.into()))
}

fn required<'a>(n: roxmltree::Node<'a, '_>, attr: &str) -> Result<&'a str, Error> {
    match n.attribute(attr) {
        Some(v) => Ok(v),
        None => schema(format!("<{}> に {attr} が無い", n.tag_name().name())),
    }
}

fn causality(s: Option<&str>) -> Result<Causality, Error> {
    Ok(match s.unwrap_or("local") {
        "parameter" => Causality::Parameter,
        "calculatedParameter" => Causality::CalculatedParameter,
        "input" => Causality::Input,
        "output" => Causality::Output,
        "local" => Causality::Local,
        "independent" => Causality::Independent,
        "structuralParameter" => Causality::StructuralParameter,
        other => return schema(format!("causality {other:?} は未知")),
    })
}

fn initial(s: Option<&str>) -> Result<Option<Initial>, Error> {
    Ok(Some(match s {
        None => return Ok(None),
        Some("exact") => Initial::Exact,
        Some("approx") => Initial::Approx,
        Some("calculated") => Initial::Calculated,
        Some(other) => return schema(format!("initial {other:?} は未知")),
    }))
}

fn u32_attr(n: roxmltree::Node<'_, '_>, attr: &str) -> Result<Option<u32>, Error> {
    n.attribute(attr)
        .map(|v| {
            v.parse()
                .or_else(|_| schema(format!("{attr} {v:?} は u32 でない")))
        })
        .transpose()
}

fn variability(s: Option<&str>, default: Variability) -> Result<Variability, Error> {
    Ok(match s {
        None => default,
        Some("constant") => Variability::Constant,
        Some("fixed") => Variability::Fixed,
        Some("tunable") => Variability::Tunable,
        Some("discrete") => Variability::Discrete,
        Some("continuous") => Variability::Continuous,
        Some(other) => return schema(format!("variability {other:?} は未知")),
    })
}

fn var_type(tag: &str, v3: bool) -> Option<VarType> {
    use VarType::*;
    Some(match (tag, v3) {
        ("Real", false) => Float64,
        ("Integer", false) => Int32,
        ("Float32", true) => Float32,
        ("Float64", true) => Float64,
        ("Int8", true) => Int8,
        ("UInt8", true) => UInt8,
        ("Int16", true) => Int16,
        ("UInt16", true) => UInt16,
        ("Int32", true) => Int32,
        ("UInt32", true) => UInt32,
        ("Int64", true) => Int64,
        ("UInt64", true) => UInt64,
        ("Binary", true) => Binary,
        ("Clock", true) => Clock,
        ("Boolean", _) => Boolean,
        ("String", _) => String,
        ("Enumeration", _) => Enumeration,
        _ => return None,
    })
}

fn variable(
    n: roxmltree::Node<'_, '_>,
    ty_node: roxmltree::Node<'_, '_>,
    v3: bool,
) -> Result<ScalarVariable, Error> {
    let tag = ty_node.tag_name().name();
    let Some(ty) = var_type(tag, v3) else {
        return schema(format!("変数の型 <{tag}> は未知"));
    };
    // 既定の variability: 2.0 は continuous。3.0 の XSD に既定は無く、仕様本文では Float が continuous、
    // それ以外が discrete（本文は未確認。research/fmi-2026-11-01.md）
    let default = if !v3 || matches!(ty, VarType::Float32 | VarType::Float64) {
        Variability::Continuous
    } else {
        Variability::Discrete
    };
    required(n, "valueReference")?;
    Ok(ScalarVariable {
        name: required(n, "name")?.to_string(),
        value_reference: u32_attr(n, "valueReference")?.unwrap_or_default(),
        causality: causality(n.attribute("causality"))?,
        variability: variability(n.attribute("variability"), default)?,
        ty,
        initial: initial(n.attribute("initial"))?,
        start: ty_node.attribute("start").map(str::to_string),
        derivative: u32_attr(ty_node, "derivative")?,
    })
}

/// `modelDescription.xml` の本文を読む。
pub fn parse_model_description(xml: &str) -> Result<ModelDescription, Error> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| Error::Xml(e.to_string()))?;
    let root = doc.root_element();
    if root.tag_name().name() != "fmiModelDescription" {
        return schema(format!("ルートが <{}>", root.tag_name().name()));
    }
    let fmi_version = required(root, "fmiVersion")?.to_string();
    let v3 = fmi_version.starts_with("3.");
    if !v3 && !fmi_version.starts_with("2.") {
        return schema(format!("fmiVersion {fmi_version:?} は未対応"));
    }
    let token_attr = if v3 { "instantiationToken" } else { "guid" };
    let child = |name: &str| root.children().find(|c| c.has_tag_name(name));
    let interface = |name: &str| -> Result<Option<Interface>, Error> {
        child(name)
            .map(|n| {
                Ok(Interface {
                    model_identifier: required(n, "modelIdentifier")?.to_string(),
                })
            })
            .transpose()
    };
    let mut variables = Vec::new();
    for n in child("ModelVariables")
        .into_iter()
        .flat_map(|mv| mv.children())
        .filter(|c| c.is_element())
    {
        if v3 {
            variables.push(variable(n, n, true)?);
        } else {
            let Some(ty) = n.children().find(|c| c.is_element()) else {
                return schema("ScalarVariable に型の子要素が無い");
            };
            variables.push(variable(n, ty, false)?);
        }
    }
    let mut model_structure = ModelStructure::default();
    let structure = child("ModelStructure");
    // 3.0 には numberOfEventIndicators が無く、ModelStructure の <EventIndicator> の数がそれに当たる（S124）
    let event_indicators = structure.map_or(0, |s| {
        s.children()
            .filter(|c| c.has_tag_name("EventIndicator"))
            .count() as u32
    });
    let elements = structure.iter().flat_map(|s| s.children());
    if v3 {
        for n in elements {
            let list = match n.tag_name().name() {
                "Output" => &mut model_structure.outputs,
                "ContinuousStateDerivative" => &mut model_structure.derivatives,
                "InitialUnknown" => &mut model_structure.initial_unknowns,
                "EventIndicator" => &mut model_structure.event_indicators,
                _ => continue,
            };
            list.push(u32_attr(n, "valueReference")?.unwrap_or_default());
        }
    } else {
        for n in elements {
            let list = match n.tag_name().name() {
                "Outputs" => &mut model_structure.outputs,
                "Derivatives" => &mut model_structure.derivatives,
                "InitialUnknowns" => &mut model_structure.initial_unknowns,
                _ => continue,
            };
            for u in n.children().filter(|c| c.has_tag_name("Unknown")) {
                required(u, "index")?;
                list.push(u32_attr(u, "index")?.unwrap_or_default());
            }
        }
    }
    let text = |n: roxmltree::Node<'_, '_>, a: &str| n.attribute(a).map(str::to_string);
    Ok(ModelDescription {
        generation_tool: text(root, "generationTool"),
        variable_naming_convention: text(root, "variableNamingConvention"),
        number_of_event_indicators: if v3 {
            event_indicators
        } else {
            u32_attr(root, "numberOfEventIndicators")?.unwrap_or(0)
        },
        default_experiment: child("DefaultExperiment").map(|n| DefaultExperiment {
            start_time: text(n, "startTime"),
            stop_time: text(n, "stopTime"),
            tolerance: text(n, "tolerance"),
        }),
        model_structure,
        model_name: required(root, "modelName")?.to_string(),
        instantiation_token: required(root, token_attr)?.to_string(),
        model_exchange: interface("ModelExchange")?,
        co_simulation: interface("CoSimulation")?,
        scheduled_execution: interface("ScheduledExecution")?,
        fmi_version,
        variables,
    })
}

/// FMI 2.0 の `modelDescription.xml` を書く（S117。P5b の `globe-mode-fmu` が使う。P5a は Rumoca のテンプレートの XML を
/// そのまま使い、これは読み→書き→読みの試験にだけ通す）。
///
/// [`ModelDescription`] に持つものだけを書く（`description`・`unit`・`SourceFiles`・`dependencies` などは落ちる）。
/// 書ける型は Real・Integer・Boolean だけで、それ以外と 2.0 にない causality は `Err`。
// shortcut: String・Enumeration（declaredType が要る）は無い。P5b が出すときに足す
pub fn write_model_description_2(md: &ModelDescription) -> Result<String, Error> {
    write_model_description(md, 2)
}

/// FMI 3.0 の `modelDescription.xml` を書く（S124。[`write_model_description_2`] と同じ範囲）。
///
/// 型の要素は `Float64`・`Int32`・`Boolean`、`ModelStructure` は `valueReference` で書く。
/// `EventIndicator` は `model_structure.event_indicators` から書く（S137。数が `number_of_event_indicators` と違えば Err）。
pub fn write_model_description_3(md: &ModelDescription) -> Result<String, Error> {
    if md.model_structure.event_indicators.len() != md.number_of_event_indicators as usize {
        return schema("EventIndicator の valueReference の数が number_of_event_indicators と違う");
    }
    write_model_description(md, 3)
}

fn write_model_description(md: &ModelDescription, major: u8) -> Result<String, Error> {
    let v3 = major == 3;
    if md.major() != Some(major) {
        return schema(format!(
            "fmiVersion {:?} は書けない（{major}.0 のみ）",
            md.fmi_version
        ));
    }
    let esc = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('"', "&quot;")
    };
    let mut x = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<fmiModelDescription\n");
    let root = [
        ("fmiVersion", Some(&md.fmi_version)),
        ("modelName", Some(&md.model_name)),
        (
            if v3 { "instantiationToken" } else { "guid" },
            Some(&md.instantiation_token),
        ),
        ("generationTool", md.generation_tool.as_ref()),
        (
            "variableNamingConvention",
            md.variable_naming_convention.as_ref(),
        ),
    ];
    for (k, v) in root {
        if let Some(v) = v {
            let _ = writeln!(x, "  {k}=\"{}\"", esc(v));
        }
    }
    if v3 {
        x.push_str(">\n");
    } else {
        let _ = writeln!(
            x,
            "  numberOfEventIndicators=\"{}\">",
            md.number_of_event_indicators
        );
    }
    for (tag, i) in [
        ("ModelExchange", &md.model_exchange),
        ("CoSimulation", &md.co_simulation),
        ("ScheduledExecution", &md.scheduled_execution),
    ] {
        if let Some(i) = i {
            let _ = writeln!(
                x,
                "  <{tag} modelIdentifier=\"{}\"/>",
                esc(&i.model_identifier)
            );
        }
    }
    if let Some(e) = &md.default_experiment {
        x.push_str("  <DefaultExperiment");
        for (k, v) in [
            ("startTime", &e.start_time),
            ("stopTime", &e.stop_time),
            ("tolerance", &e.tolerance),
        ] {
            if let Some(v) = v {
                let _ = write!(x, " {k}=\"{}\"", esc(v));
            }
        }
        x.push_str("/>\n");
    }
    x.push_str("  <ModelVariables>\n");
    for v in &md.variables {
        let causality = match v.causality {
            Causality::Parameter => "parameter",
            Causality::CalculatedParameter => "calculatedParameter",
            Causality::Input => "input",
            Causality::Output => "output",
            Causality::Local => "local",
            Causality::Independent => "independent",
            Causality::StructuralParameter if v3 => "structuralParameter",
            Causality::StructuralParameter => return schema("structuralParameter は 2.0 に無い"),
        };
        let variability = match v.variability {
            Variability::Constant => "constant",
            Variability::Fixed => "fixed",
            Variability::Tunable => "tunable",
            Variability::Discrete => "discrete",
            Variability::Continuous => "continuous",
        };
        let ty = match (v.ty, v3) {
            (VarType::Float64, false) => "Real",
            (VarType::Int32, false) => "Integer",
            (VarType::Float64, true) => "Float64",
            (VarType::Int32, true) => "Int32",
            (VarType::Boolean, _) => "Boolean",
            (other, _) => return schema(format!("型 {other:?} は書けない")),
        };
        // 3.0 は型の要素に属性をまとめて書く（ScalarVariable の入れ子が無い）
        let _ = write!(
            x,
            "    <{}name=\"{}\" valueReference=\"{}\" causality=\"{causality}\" variability=\"{variability}\"",
            if v3 {
                format!("{ty} ")
            } else {
                "ScalarVariable ".into()
            },
            esc(&v.name),
            v.value_reference
        );
        if let Some(i) = v.initial {
            let i = match i {
                Initial::Exact => "exact",
                Initial::Approx => "approx",
                Initial::Calculated => "calculated",
            };
            let _ = write!(x, " initial=\"{i}\"");
        }
        if !v3 {
            let _ = write!(x, ">\n      <{ty}");
        }
        if let Some(s) = &v.start {
            let _ = write!(x, " start=\"{}\"", esc(s));
        }
        if let Some(d) = v.derivative {
            let _ = write!(x, " derivative=\"{d}\"");
        }
        x.push_str(if v3 {
            "/>\n"
        } else {
            "/>\n    </ScalarVariable>\n"
        });
    }
    x.push_str("  </ModelVariables>\n  <ModelStructure>\n");
    let s = &md.model_structure;
    if v3 {
        for (tag, list) in [
            ("Output", &s.outputs),
            ("ContinuousStateDerivative", &s.derivatives),
            ("InitialUnknown", &s.initial_unknowns),
            ("EventIndicator", &s.event_indicators),
        ] {
            for r in list {
                let _ = writeln!(x, "    <{tag} valueReference=\"{r}\"/>");
            }
        }
        x.push_str("  </ModelStructure>\n</fmiModelDescription>\n");
        return Ok(x);
    }
    for (tag, list) in [
        ("Outputs", &s.outputs),
        ("Derivatives", &s.derivatives),
        ("InitialUnknowns", &s.initial_unknowns),
    ] {
        // 2.0 の XSD では Unknown が 1 つ以上要るので、空の要素は書かない
        if list.is_empty() {
            continue;
        }
        let _ = writeln!(x, "    <{tag}>");
        for i in list {
            let _ = writeln!(x, "      <Unknown index=\"{i}\"/>");
        }
        let _ = writeln!(x, "    </{tag}>");
    }
    x.push_str("  </ModelStructure>\n</fmiModelDescription>\n");
    Ok(x)
}
