use std::path::Path;

use freyja_core::{
    error::Error,
    planner::{Plan, PlanAction, PlanReason, Planner},
};

use crate::helpers::{load_state, validate_state_path};

pub async fn execute(
    spec: &freyja_core::spec::Spec,
    state_path: &Path,
    planner: &Planner<'_>,
) -> Result<(), Error> {
    validate_state_path(spec, state_path)?;
    let state = load_state(state_path)?;

    let plan = planner.plan(spec, &state).await?;

    print_plan(&plan);

    Ok(())
}

fn print_plan(plan: &Plan) {
    for target in &plan.targets {
        match target.action {
            PlanAction::Build => {
                println!("BUILD {}", target.target);

                for reason in &target.reasons {
                    println!("  {}", format_reason(reason));
                }
            }

            PlanAction::Skip => {
                println!("SKIP  {}", target.target);
            }
        }
    }
}

fn format_reason(reason: &PlanReason) -> String {
    match reason {
        PlanReason::NeverBuilt => "target has never been built".to_owned(),
        PlanReason::BuildInputChanged => {
            "build inputs changed (or old state has no fingerprint)".to_owned()
        }

        PlanReason::DependencyChanged {
            kind,
            reference,
            previous_fingerprint,
            current_fingerprint,
            ..
        } => {
            format!(
                "{kind} dependency {reference} changed: \
                 {previous_fingerprint} -> {current_fingerprint}"
            )
        }

        PlanReason::DependencyAdded {
            kind, reference, ..
        } => {
            format!("{kind} dependency {reference} was added")
        }

        PlanReason::DependencyRemoved {
            kind, reference, ..
        } => {
            format!("{kind} dependency {reference} was removed")
        }
    }
}
