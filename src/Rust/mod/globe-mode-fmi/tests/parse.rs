//! Reference-FMUs v0.0.42 の modelDescription.xml（tests/data、BSD-2-Clause）を読む。

use globe_mode_fmi::{
    Causality, Error, ModelDescription, VarType, Variability, parse_model_description,
};

fn load(name: &str) -> ModelDescription {
    let path = format!("{}/tests/data/{name}", env!("CARGO_MANIFEST_DIR"));
    parse_model_description(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn var<'a>(m: &'a ModelDescription, name: &str) -> &'a globe_mode_fmi::ScalarVariable {
    m.variables.iter().find(|v| v.name == name).unwrap()
}

#[test]
fn dahlquist_fmi2_and_fmi3_give_the_same_table() {
    let (a, b) = (load("Dahlquist-fmi2.xml"), load("Dahlquist-fmi3.xml"));
    assert_eq!((a.major(), b.major()), (Some(2), Some(3)));
    assert_eq!(
        (a.fmi_version.as_str(), b.fmi_version.as_str()),
        ("2.0", "3.0")
    );
    for m in [&a, &b] {
        assert_eq!(m.model_name, "Dahlquist");
        assert_eq!(
            m.instantiation_token,
            "{221063D2-EF4A-45FE-B954-B5BFEEA9A59B}"
        );
        assert_eq!(
            m.model_exchange.as_ref().unwrap().model_identifier,
            "Dahlquist"
        );
        assert_eq!(
            m.co_simulation.as_ref().unwrap().model_identifier,
            "Dahlquist"
        );
        assert!(m.scheduled_execution.is_none());
    }
    // 2.0 と 3.0 で変数表（名前・値参照・causality・variability・型）が一致する
    assert_eq!(a.variables, b.variables);
    assert_eq!(a.variables.len(), 4);
    let x = var(&a, "x");
    assert_eq!(
        (x.value_reference, x.causality, x.variability, x.ty),
        (
            1,
            Causality::Output,
            Variability::Continuous,
            VarType::Float64
        )
    );
    assert_eq!(var(&a, "time").causality, Causality::Independent);
    assert_eq!(var(&a, "k").variability, Variability::Fixed);
}

#[test]
fn feedthrough_covers_all_types() {
    let (a, b) = (load("Feedthrough-fmi2.xml"), load("Feedthrough-fmi3.xml"));
    assert_eq!(a.variables.len(), 15);
    assert_eq!(b.variables.len(), 35);
    // 2.0: variability の既定は continuous。3.0: Float 以外の既定は discrete
    let i = var(&a, "Int32_input");
    assert_eq!((i.ty, i.causality), (VarType::Int32, Causality::Input));
    assert_eq!(var(&b, "Int8_input").ty, VarType::Int8);
    assert_eq!(var(&b, "Int8_input").variability, Variability::Discrete);
    assert_eq!(var(&b, "Binary_input").ty, VarType::Binary);
    assert_eq!(var(&b, "Enumeration_output").ty, VarType::Enumeration);
    assert_eq!(var(&b, "Float32_continuous_input").ty, VarType::Float32);
    assert_eq!(
        var(&b, "Float32_continuous_input").variability,
        Variability::Continuous
    );
}

#[test]
fn scheduled_execution_and_clock() {
    let xml = r#"<fmiModelDescription fmiVersion="3.0" modelName="Clocks" instantiationToken="{C5}">
        <ScheduledExecution modelIdentifier="Clocks"/>
        <ModelVariables>
          <Clock name="inClock1" valueReference="1001" causality="input" intervalVariability="constant"/>
        </ModelVariables></fmiModelDescription>"#;
    let m = parse_model_description(xml).unwrap();
    assert!(m.model_exchange.is_none() && m.co_simulation.is_none());
    assert_eq!(m.scheduled_execution.unwrap().model_identifier, "Clocks");
    assert_eq!(m.variables[0].ty, VarType::Clock);
}

#[test]
fn bad_inputs_are_errors_not_panics() {
    assert!(matches!(parse_model_description("<a"), Err(Error::Xml(_))));
    assert!(matches!(
        parse_model_description("<x/>"),
        Err(Error::Schema(_))
    ));
    let head = |v: &str, body: &str| {
        format!(
            r#"<fmiModelDescription fmiVersion="{v}" modelName="M" guid="g" instantiationToken="t">{body}</fmiModelDescription>"#
        )
    };
    // 未対応の版、未知の causality、u32 でない値参照、型の子要素が無い 2.0 の変数
    for xml in [
        head("1.0", ""),
        head(
            "2.0",
            r#"<ModelVariables><ScalarVariable name="a" valueReference="0" causality="bogus"><Real/></ScalarVariable></ModelVariables>"#,
        ),
        head(
            "2.0",
            r#"<ModelVariables><ScalarVariable name="a" valueReference="-1"><Real/></ScalarVariable></ModelVariables>"#,
        ),
        head(
            "2.0",
            r#"<ModelVariables><ScalarVariable name="a" valueReference="0"/></ModelVariables>"#,
        ),
        head(
            "3.0",
            r#"<ModelVariables><Real name="a" valueReference="0"/></ModelVariables>"#,
        ),
    ] {
        assert!(
            matches!(parse_model_description(&xml), Err(Error::Schema(_))),
            "{xml}"
        );
    }
}
