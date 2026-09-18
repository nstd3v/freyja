use std::path::Path;

use freyja_core::{
    builder::Builder,
    planner::{PlanAction, Planner},
};

use freyja_core::error::Error;

use crate::helpers::{load_spec, load_state};

pub async fn execute(
    spec_path: &Path,
    state_path: &Path,
    planner: &Planner<'_>,
    builder: &dyn Builder,
) -> Result<(), Error> {
    let spec = load_spec(spec_path)?;
    let mut state = load_state(state_path)?;

    let plan = planner.plan(&spec, &state).await?;

    for target_plan in plan.targets {
        match target_plan.action {
            PlanAction::Build => {
                let target = &spec.targets[&target_plan.target];

                println!("building {}", target_plan.target);

                builder.build(&target_plan.target, target).await?;

                state.record_build(target_plan.target, target_plan.dependencies);

                state.save(state_path)?;
            }

            PlanAction::Skip => {
                println!("skipping {}", target_plan.target);
            }
        }
    }

    Ok(())
}
