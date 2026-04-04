//! Pipeline end-to-end demo
//!
//! Run with: cargo run --example pipeline_demo

use std::time::Instant;
use zcode::pipeline::{Pipeline, PipelineConfig, PhaseStatus};
use zcode::pipeline::config::VerificationPhaseConfig;

// ANSI colors for pretty output
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";
const LINE: &str = "============================================================";

fn print_header(title: &str) {
    println!();
    println!("{BOLD}{CYAN}{LINE}{RESET}");
    println!("{BOLD}{CYAN}  {}{RESET}", title);
    println!("{BOLD}{CYAN}{LINE}{RESET}");
}

fn print_phase_result(phase: &zcode::pipeline::PhaseResult, idx: usize) {
    let (icon, color) = match &phase.status {
        PhaseStatus::Success => ("PASS", GREEN),
        PhaseStatus::Failed => ("FAIL", RED),
        PhaseStatus::Skipped => ("SKIP", YELLOW),
        PhaseStatus::Retry { .. } => ("RETRY", YELLOW),
    };
    let ms = phase.duration.as_millis();
    println!(
        "  {color}{icon}{RESET} [{idx}] {BOLD}{:<14}{RESET} {ms:>5}ms  {}",
        phase.phase_name,
        truncate(&phase.summary, 50),
    );
    if let Some(err) = &phase.error {
        println!("         {RED}Error: {err}{RESET}");
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}

fn print_metrics(metrics: &zcode::pipeline::PipelineMetrics) {
    let duration = metrics
        .total_duration
        .map(|d| format!("{:.1}s", d.as_secs_f64()))
        .unwrap_or_else(|| "n/a".to_string());
    println!();
    println!("{DIM}--- Metrics ---{RESET}");
    println!("  Total duration : {duration}");
    println!("  Total tokens   : {}", metrics.total_tokens);
    println!("  Est. cost      : ${:.4}", metrics.estimated_cost_usd);
    println!("  Phases run     : {}", metrics.phase_metrics.len());
}

#[tokio::main]
async fn main() {
    print_header("Zcode Pipeline Demo");

    // ── Scenario 1: Full pipeline with all 5 phases ──
    println!();
    println!("{BOLD}Scenario 1: Full pipeline (all phases){RESET}");
    println!("{DIM}Requirement: \"Create a hello world Python script\"{RESET}");

    let tmp = tempfile::TempDir::new().expect("create temp dir");
    let pipeline = Pipeline::default_pipeline();
    let wall_start = Instant::now();

    let result = pipeline
        .run("Create a hello world Python script", tmp.path())
        .await
        .expect("pipeline run failed");

    let wall_elapsed = wall_start.elapsed();

    for (i, phase) in result.phase_results.iter().enumerate() {
        print_phase_result(phase, i + 1);
    }
    print_metrics(&result.metrics);

    let status_icon = if result.success { format!("{GREEN}OK{RESET}") } else { format!("{RED}FAIL{RESET}") };
    println!();
    println!("{BOLD}Result: {status_icon} | Wall time: {:.1}s{RESET}", wall_elapsed.as_secs_f64());
    println!("{DIM}{}", result.summary());

    // ── Scenario 2: Partial pipeline (cognition + delivery disabled) ──
    println!();
    println!("{BOLD}Scenario 2: Partial pipeline (cognition + delivery off){RESET}");
    println!("{DIM}Requirement: \"Fix the off-by-one error in src/lib.rs\"{RESET}");

    let tmp2 = tempfile::TempDir::new().expect("create temp dir 2");
    let mut config = PipelineConfig::default();
    config.cognition.enabled = false;
    config.delivery.enabled = false;
    config.verification = VerificationPhaseConfig {
        enabled: true,
        optional: true,
        min_score: 60.0,
        max_retries: 1,
    };
    let pipeline2 = Pipeline::new(config);
    let wall_start2 = Instant::now();

    let result2 = pipeline2
        .run("Fix the off-by-one error in src/lib.rs", tmp2.path())
        .await
        .expect("pipeline run 2 failed");

    let wall_elapsed2 = wall_start2.elapsed();

    for (i, phase) in result2.phase_results.iter().enumerate() {
        print_phase_result(phase, i + 1);
    }
    print_metrics(&result2.metrics);

    let status_icon2 = if result2.success { format!("{GREEN}OK{RESET}") } else { format!("{RED}FAIL{RESET}") };
    println!();
    println!("{BOLD}Result: {status_icon2} | Wall time: {:.1}s{RESET}", wall_elapsed2.as_secs_f64());
    println!("{DIM}{}", result2.summary());

    // ── Summary ──
    println!();
    println!("{BOLD}{CYAN}{LINE}{RESET}");
    println!("{BOLD}{CYAN}  Demo complete{RESET}");
    println!("{BOLD}{CYAN}{LINE}{RESET}");
    println!();
    println!("  Scenario 1 phases: {}  success: {}", result.phase_results.len(), result.success);
    println!("  Scenario 2 phases: {}  success: {}", result2.phase_results.len(), result2.success);
}
