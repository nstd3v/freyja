use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "freyja",
    version,
    about = "Dependency-aware OCI image build orchestrator"
)]
pub struct Cli {
    /// Path to the Freyja configuration file.
    #[arg(
        short = 'f',
        long = "file",
        global = true,
        default_value = "freyja.toml"
    )]
    pub file: PathBuf,

    /// Path to the Freyj state file
    #[arg(
        short = 's',
        long = "state",
        global = true,
        default_value = ".freyja/state.toml"
    )]
    pub state: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Show which targets need to be rebuilt.
    Plan(PlanArgs),

    /// Build targets that need to be rebuilt.
    Build(BuildArgs),

    /// Explain why a target needs to be rebuilt.
    Explain(ExplainArgs),
}

#[derive(Debug, Args)]
pub struct PlanArgs {
    // Plan only the specified target.
    //pub target: Option<String>,
}

#[derive(Debug, Args)]
pub struct BuildArgs {
    // Build only the specified target.
    //pub target: Option<String>,

    // Build even if dependencies have not changed.
    //#[arg(long)]
    //pub force: bool,
}

#[derive(Debug, Args)]
pub struct ExplainArgs {
    /// Target to explain.
    pub target: String,
}
