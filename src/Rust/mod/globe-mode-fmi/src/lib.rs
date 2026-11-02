//! FMI 2.0 / 3.0 の `modelDescription.xml` を読む（S057、P5 の準備。[ADR-0026]）。
//!
//! 今は XML を変数表にするだけ。共有ライブラリの読み込み（`libloading`）とマスターは P5 で足す。
//! XML だけなので wasm32 でも通る（ブラウザで FMU の中身を見る用途）。
//!
//! [ADR-0026]: https://github.com/asari300/20261018_GlobeMode/blob/main/doc/adr/0026-fmi-import-placement.md

use std::fmt;

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
    let vr = required(n, "valueReference")?;
    Ok(ScalarVariable {
        name: required(n, "name")?.to_string(),
        value_reference: vr
            .parse()
            .or_else(|_| schema(format!("valueReference {vr:?} は u32 でない")))?,
        causality: causality(n.attribute("causality"))?,
        variability: variability(n.attribute("variability"), default)?,
        ty,
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
    Ok(ModelDescription {
        model_name: required(root, "modelName")?.to_string(),
        instantiation_token: required(root, token_attr)?.to_string(),
        model_exchange: interface("ModelExchange")?,
        co_simulation: interface("CoSimulation")?,
        scheduled_execution: interface("ScheduledExecution")?,
        fmi_version,
        variables,
    })
}
