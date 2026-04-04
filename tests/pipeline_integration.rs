//! Integration test for the full pipeline
//!
//! Run with: cargo test --test pipeline_integration

use zcode::pipeline::{Pipeline, PipelineConfig, PhaseStatus};

#[tokio::test]
async fn pipeline_runs_all_phases_successfully() {
    let tmp = tempfile::TempDir::new().unwrap();
    let pipeline = Pipeline::default_pipeline();

    let result = pipeline
        .run("Create a hello world Python script", tmp.path())
        .await
        .expect("pipeline should not error");

    assert!(result.success, "pipeline should succeed");
    assert_eq!(result.phase_results.len(), 5, "should run all 5 phases");

    let phase_names: Vec<&str> = result.phase_results.iter().map(|p| p.phase_name.as_str()).collect();
    assert_eq!(phase_names, vec!["cognition", "planning", "execution", "verification", "delivery"]);

    for phase in &result.phase_results {
        assert!(
            matches!(phase.status, PhaseStatus::Success),
            "phase '{}' should succeed, got: {:?}",
            phase.phase_name,
            phase.status,
        );
    }
}

#[tokio::test]
async fn pipeline_respects_disabled_phases() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut config = PipelineConfig::default();
    config.cognition.enabled = false;
    config.delivery.enabled = false;

    let pipeline = Pipeline::new(config);
    let result = pipeline
        .run("Fix a bug", tmp.path())
        .await
        .expect("pipeline should not error");

    assert!(result.success, "partial pipeline should succeed");
    assert_eq!(result.phase_results.len(), 3, "should run 3 phases");

    let phase_names: Vec<&str> = result.phase_results.iter().map(|p| p.phase_name.as_str()).collect();
    assert_eq!(phase_names, vec!["planning", "execution", "verification"]);
}

#[tokio::test]
async fn pipeline_metrics_are_recorded() {
    let tmp = tempfile::TempDir::new().unwrap();
    let pipeline = Pipeline::default_pipeline();

    let result = pipeline
        .run("Add tests for the math module", tmp.path())
        .await
        .expect("pipeline should not error");

    assert!(result.metrics.total_duration.is_some(), "total duration should be set");
    assert_eq!(result.metrics.phase_metrics.len(), 5, "should have 5 phase metric entries");
}
