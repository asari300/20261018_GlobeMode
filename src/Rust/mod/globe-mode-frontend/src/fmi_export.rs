//! S094（P5a）: Rumoca の組み込みターゲット `fmi2` のテンプレートで、ソースの FMU のファイルを作る。
//!
//! Rumoca の CLI（umbrella crate `rumoca` の `target_manifest.rs`）と同じ順に、公開 API だけを呼ぶ:
//! DAE と Solve の能力の検査 → `lower_to_fmi_component` → `try_c` → `SolveTemplateRenderer` で
//! マニフェストの `[[files]]` を描く。テンプレートは改変しない（ADR-0031）。

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use rumoca_compile::codegen::SolveTemplateRenderer;
use rumoca_compile::codegen::targets::{
    TargetBundle, TargetTemplateSource, validate_dae_target_capabilities,
    validate_solve_target_capabilities,
};

use crate::CompiledModel;

/// 描いたファイル（FMU の中の相対パスと中身）。`modelDescription.xml` と `sources/*.c|h`。
pub type RenderedFiles = Vec<(String, String)>;

/// `target` は Rumoca の組み込みの FMI ターゲットの名前（`fmi2` か `fmi3`）。
pub fn render_rumoca_fmi(model: &CompiledModel, target: &str) -> Result<RenderedFiles, String> {
    let bundle =
        TargetBundle::builtin(target).ok_or(format!("unknown Rumoca target `{target}`"))?;
    let manifest = bundle.parse_manifest().map_err(|e| format!("{e:#}"))?;
    let caps = manifest
        .capabilities
        .as_ref()
        .ok_or(format!("target `{target}` declares no capabilities"))?;
    let dae = model.rumoca_dae();
    validate_dae_target_capabilities(dae, &manifest, caps).map_err(|e| format!("{e:#}"))?;
    // shortcut: 入力の start を種にしない（rumoca-sim の host_driven_input_seeds は crate の外から呼べない）。入力を持つモデルを出すときに直す
    let component = rumoca_phase_solve::fmi::lower_to_fmi_component(dae, &HashMap::new())
        .map_err(|e| e.to_string())?;
    validate_solve_target_capabilities(component.problem(), &manifest, caps)
        .map_err(|e| format!("{e:#}"))?;
    let c = component
        .into_codegen_view()
        .try_c()
        .map_err(|e| e.to_string())?;
    let renderer = SolveTemplateRenderer::new_owned_with_fmi(c).map_err(|e| e.to_string())?;
    // Rumoca の CLI は guid を毎回 UUID v4 にする。ここでは同じモデルから同じ FMU になるよう名前から作る
    let artifact = serde_json::json!({ "identities": { "fmu": guid(&model.model) } });
    let stem = model.model.replace('.', "_");
    let render = |template: &str| {
        renderer
            .render_with_name_and_artifact(template, &stem, &artifact)
            .map_err(|e| e.to_string())
    };
    manifest
        .files
        .iter()
        .map(|file| {
            let source = bundle
                .template_source(&file.template)
                .map_err(|e| format!("{e:#}"))?;
            Ok((render(&file.path)?.trim().to_owned(), render(&source)?))
        })
        .collect()
}

fn guid(name: &str) -> String {
    let half = |salt: u8| {
        let mut h = DefaultHasher::new();
        (salt, name).hash(&mut h);
        h.finish()
    };
    let (a, b) = (half(0), half(1));
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        a >> 32,
        (a >> 16) & 0xffff,
        a & 0xffff,
        b >> 48,
        b & 0xffff_ffff_ffff
    )
}
