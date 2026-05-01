use mathdocs_markdown::RenderEngine;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

#[test]
fn renders_linear_model_example() {
    let rendered = RenderEngine::default()
        .render_path(&fixture("linear_model/linear_model.py"))
        .unwrap();
    assert!(rendered.markdown.contains("# Linear model"));
    assert!(rendered.markdown.contains(r"\operatorname{loss}"));
    assert!(rendered.markdown.contains(r"A_{ij}x_{j}"));
    assert!(rendered.markdown.contains(r"\sigma"));
}

#[test]
fn renders_electrodynamics_example() {
    let rendered = RenderEngine::default()
        .render_path(&fixture("electrodynamics/electrodynamics.py"))
        .unwrap();
    assert!(rendered
        .markdown
        .contains("# Electrodynamics as a U(1) Gauge Theory"));
    assert!(rendered.markdown.contains("F = dA"));
    assert!(rendered.markdown.contains(r"F_{\mu\nu}"));
    assert!(rendered.markdown.contains("A' = A + d\\alpha"));
    assert!(rendered.markdown.contains("dF = 0"));
}

#[test]
fn renders_feature_showcase_example() {
    let rendered = RenderEngine::default()
        .render_path(&fixture("feature_showcase/feature_showcase.py"))
        .unwrap();
    assert!(rendered.markdown.contains("# MathDocs Feature Showcase"));
    assert!(rendered.markdown.contains(r"e^{rt}"));
    assert!(rendered.markdown.contains(r"x^{\beta}"));
    assert!(rendered.markdown.contains(r"\int_{0}^{1} x^{2}\,dx"));
    assert!(rendered.markdown.contains(r"C_{ik} = A_{ij}B_{jk}"));
}

#[test]
fn renders_generated_plot_example() {
    let rendered = RenderEngine::default()
        .render_path(&fixture("generated_plot/generated_plot.py"))
        .unwrap();
    assert!(rendered.markdown.contains("# Generated Plot"));
    assert!(rendered.markdown.contains("![Training loss curve]("));
    assert!(rendered.markdown.contains("training_loss.svg)"));
    assert!(rendered
        .markdown
        .contains("_Loss decreases over eight training epochs._"));
    assert!(rendered
        .markdown
        .contains(r"\operatorname{final}_{loss} = 0.18"));
}

#[test]
fn merges_sidecar_metadata() {
    let rendered = RenderEngine::default()
        .render_path(&fixture("sidecar_demo/sidecar_demo.py"))
        .unwrap();
    assert!(rendered.markdown.contains(r"\theta"));
    assert!(rendered.markdown.contains(r"\sigma"));
}

#[test]
fn applies_comment_auto_name_config() {
    let source = r#"
# mathdocs: disable=auto_name/subscript
final_loss = 1
alpha = beta # mathdocs: disable=auto_name/symbol
"#;
    let rendered = RenderEngine::default().render_source(None, "test.py", source);
    assert!(rendered.markdown.contains(r"final\_loss = 1"));
    assert!(rendered
        .markdown
        .contains(r"\operatorname{alpha} = \operatorname{beta}"));
}

#[test]
fn applies_root_mathdocs_toml_auto_name_config() {
    let root = std::env::temp_dir().join(format!(
        "mathdocs-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("mathdocs.toml"),
        "[auto_name]\nsubscript = false\nsymbol = false\n",
    )
    .unwrap();
    let source_path = root.join("example.py");
    std::fs::write(&source_path, "final_loss = alpha\n").unwrap();

    let rendered = RenderEngine::default().render_path(&source_path).unwrap();
    assert!(rendered
        .markdown
        .contains(r"final\_loss = \operatorname{alpha}"));

    std::fs::remove_dir_all(root).unwrap();
}
