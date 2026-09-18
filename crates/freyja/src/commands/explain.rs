use std::path::Path;

use freyja_core::{
    error::Error,
    planner::{PlanReason, Planner, TargetPlan},
};

use crate::helpers::{load_spec, load_state};

pub async fn execute(
    target_name: &str,
    spec_path: &Path,
    state_path: &Path,
    planner: &Planner<'_>,
) -> Result<(), Error> {
    let spec = load_spec(spec_path)?;
    let state = load_state(state_path)?;

    let plan = planner.plan(&spec, &state).await?;

    let target = plan
        .targets
        .iter()
        .find(|target| target.target == target_name)
        .ok_or_else(|| Error::UnknownTarget {
            target: target_name.to_string(),
        })?;

    print_explanation(target);

    Ok(())
}

fn print_explanation(target: &TargetPlan) {
    println!("target: {}", target.target);
    println!("action: {:?}", target.action);

    if target.reasons.is_empty() {
        println!("reason: dependencies unchanged");
        return;
    }

    println!("reasons:");

    for reason in &target.reasons {
        match reason {
            PlanReason::NeverBuilt => {
                println!("  - target has never been built");
            }

            PlanReason::DependencyChanged {
                name,
                kind,
                reference,
                previous_fingerprint,
                current_fingerprint,
            } => {
                println!("  - dependency `{name}` changed");
                println!("    type: {kind}");
                println!("    reference: {reference}");
                println!("    previous: {previous_fingerprint}");
                println!("    current:  {current_fingerprint}");
            }

            PlanReason::DependencyAdded {
                name,
                kind,
                reference,
            } => {
                println!("  - dependency `{name}` was added");
                println!("    type: {kind}");
                println!("    reference: {reference}");
            }

            PlanReason::DependencyRemoved {
                name,
                kind,
                reference,
            } => {
                println!("  - dependency `{name}` was removed");
                println!("    type: {kind}");
                println!("    reference: {reference}");
            }
        }
    }
}
