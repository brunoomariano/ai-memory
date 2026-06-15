//! `ai-memory auto-improve` — dry-run durable wiki edit proposals.

use ai_memory_consolidate::AutoImproveReport;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::cli::AutoImproveArgs;
use crate::config::Config;
use crate::http_client::{ServerEndpoint, post_json};

/// Request sent to `POST /admin/auto-improve`.
#[derive(Serialize)]
struct AutoImproveRequest {
    workspace: String,
    project: String,
    session_id: String,
    dry_run: bool,
    stage: bool,
    min_observations: usize,
    min_session_duration_secs: u64,
    min_confidence: f32,
    max_input_tokens: usize,
    max_proposals_per_run: usize,
    include_raw_fallback: bool,
    proposal_actor: String,
    pending_path: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct StageResponse {
    run_id: String,
    proposal_ids: Vec<String>,
    sidecar_paths: Vec<String>,
    report: AutoImproveReport,
}

/// Run the `auto-improve` subcommand.
///
/// # Errors
/// Returns an error if the command is not explicitly dry-run, if the server is
/// unreachable, or if the server rejects the review request.
pub async fn run(config: &Config, args: AutoImproveArgs) -> Result<()> {
    if args.dry_run == args.stage {
        bail!("choose exactly one of --dry-run or --stage");
    }

    let endpoint = ServerEndpoint::from_config(config);
    let project = super::resolve_project_name(config, args.project.as_deref())?;
    let settings = &config.auto_improve;
    let request = AutoImproveRequest {
        workspace: args.workspace,
        project: project.clone(),
        session_id: args.session_id,
        dry_run: args.dry_run,
        stage: args.stage,
        min_observations: args.min_observations.unwrap_or(settings.min_observations),
        min_session_duration_secs: args
            .min_session_duration_secs
            .unwrap_or(settings.min_session_duration_secs),
        min_confidence: args.min_confidence.unwrap_or(settings.min_confidence),
        max_input_tokens: args.max_input_tokens.unwrap_or(settings.max_input_tokens),
        max_proposals_per_run: args.max_proposals.unwrap_or(settings.max_proposals_per_run),
        include_raw_fallback: args.include_raw_fallback || settings.include_raw_fallback,
        proposal_actor: settings.proposal_actor.clone(),
        pending_path: settings.pending_path.clone(),
    };

    if args.stage {
        let response: StageResponse = post_json(&endpoint, "/admin/auto-improve", &request).await?;
        if args.json {
            println!("{}", serde_json::to_string_pretty(&response)?);
        } else {
            println!("Staged auto-improve run {}", response.run_id);
            for (id, path) in response
                .proposal_ids
                .iter()
                .zip(response.sidecar_paths.iter())
            {
                println!("  - {id}: {path}");
            }
        }
    } else {
        let report: AutoImproveReport =
            post_json(&endpoint, "/admin/auto-improve", &request).await?;
        if args.json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            print_human_report(&report, &project);
            println!("\n--- machine-readable ---");
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }
    Ok(())
}

fn print_human_report(report: &AutoImproveReport, project: &str) {
    println!(
        "\nAuto-improve dry-run for {project} session {}\n",
        report.session_id
    );
    println!("Summary: {}", report.summary);
    println!(
        "Input: {} observations, {}s span, ~{} tokens",
        report.observations_considered, report.session_duration_secs, report.estimated_input_tokens
    );
    println!(
        "Reviewer: {}/{}; min confidence {}",
        report.provider, report.model, report.min_confidence
    );
    println!(
        "Future staging: actor `{}`, path `{}`",
        report.proposal_actor, report.pending_path
    );

    if report.proposals.is_empty() {
        println!("\nValidated proposals: none");
    } else {
        println!("\nValidated proposals:");
        for proposal in &report.proposals {
            println!(
                "  - {} [{}] confidence {:.2}: {}",
                proposal.path, proposal.kind, proposal.confidence, proposal.title
            );
        }
    }

    if !report.rejected_candidates.is_empty() {
        println!("\nRejected candidates:");
        for rejected in &report.rejected_candidates {
            println!("  - {}: {}", rejected.reason, rejected.evidence);
        }
    }

    if !report.warnings.is_empty() {
        println!("\nWarnings:");
        for warning in &report.warnings {
            println!("  - {warning}");
        }
    }
}
