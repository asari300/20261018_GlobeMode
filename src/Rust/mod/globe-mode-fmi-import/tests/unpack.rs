//! 展開の安全（zip slip・シンボリックリンク）。ネイティブコードは実行しない。

use std::io::Write;

use globe_mode_fmi_import::{Error, Fmu, read_model_description};

const XML: &str = r#"<fmiModelDescription fmiVersion="2.0" modelName="M" guid="{1}">
<CoSimulation modelIdentifier="M"/><ModelVariables/></fmiModelDescription>"#;

fn fmu(entries: &[(&str, &[u8], Option<u32>)]) -> tempfile::NamedTempFile {
    let file = tempfile::Builder::new().suffix(".fmu").tempfile().unwrap();
    let mut w = zip::ZipWriter::new(file.reopen().unwrap());
    for (name, data, mode) in entries {
        let mut o = zip::write::SimpleFileOptions::default();
        if let Some(m) = mode {
            o = o.unix_permissions(*m);
        }
        w.start_file(*name, o).unwrap();
        w.write_all(data).unwrap();
    }
    w.finish().unwrap();
    file
}

#[test]
fn reads_the_description_without_unpacking() {
    let f = fmu(&[("modelDescription.xml", XML.as_bytes(), None)]);
    assert_eq!(read_model_description(f.path()).unwrap().model_name, "M");
}

#[test]
fn an_oversized_description_is_cut_for_run_as_for_info() {
    // 64 MiB を超える XML は `info` も `run`（Fmu::open）も同じ上限で切る。切られた XML は読めない（#96）
    let big = format!("{XML}<!--{}-->", "a".repeat(65 << 20));
    let f = fmu(&[("modelDescription.xml", big.as_bytes(), None)]);
    assert!(read_model_description(f.path()).is_err());
    assert!(matches!(
        Fmu::open(f.path()).unwrap_err(),
        Error::Description(_)
    ));
}

#[test]
fn zip_slip_is_rejected_and_nothing_is_written_outside() {
    let parent = tempfile::tempdir().unwrap();
    let evil = format!(
        "../{}-evil.txt",
        parent.path().file_name().unwrap().to_string_lossy()
    );
    let f = fmu(&[
        ("modelDescription.xml", XML.as_bytes(), None),
        (&evil, b"x", None),
    ]);
    let e = Fmu::open(f.path()).unwrap_err();
    assert!(matches!(e, Error::Zip(_)), "{e}");
    assert!(
        !std::env::temp_dir()
            .join(evil.trim_start_matches("../"))
            .exists()
    );
}

#[test]
fn symlink_entries_are_rejected() {
    let file = tempfile::Builder::new().suffix(".fmu").tempfile().unwrap();
    let mut w = zip::ZipWriter::new(file.reopen().unwrap());
    let o = zip::write::SimpleFileOptions::default();
    w.start_file("modelDescription.xml", o).unwrap();
    w.write_all(XML.as_bytes()).unwrap();
    w.add_symlink("link", "/etc/passwd", o).unwrap();
    w.finish().unwrap();
    assert!(
        matches!(Fmu::open(file.path()).unwrap_err(), Error::Zip(m) if m.contains("シンボリックリンク"))
    );
}

#[test]
fn a_missing_library_for_this_os_is_an_error() {
    let f = fmu(&[("modelDescription.xml", XML.as_bytes(), None)]);
    let fmu = Fmu::open(f.path()).unwrap();
    assert!(matches!(
        fmu.instantiate_cs("x").err().unwrap(),
        Error::Library(_)
    ));
}

#[test]
fn a_model_identifier_with_path_separators_is_rejected_before_loading() {
    let xml = XML.replace(r#"modelIdentifier="M""#, r#"modelIdentifier="../../evil""#);
    let f = fmu(&[("modelDescription.xml", xml.as_bytes(), None)]);
    let fmu = Fmu::open(f.path()).unwrap();
    assert!(
        matches!(fmu.instantiate_cs("x").err().unwrap(), Error::Library(m) if m.contains("不正"))
    );
}
