use std::path::PathBuf;

use freyja_core::planner::{Plan, PlanAction, PlanReason};

use crate::cli::PlanArgs;

pub fn execute(file: &PathBuf, args: PlanArgs) -> Result<(), freyja_core::error::Error> {
    todo!()
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
