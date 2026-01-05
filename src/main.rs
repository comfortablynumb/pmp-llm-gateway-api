use clap::Parser;
use pmp_llm_gateway::cli::{self, Cli, Command};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Serve => cli::serve::run().await,
        Command::Api => cli::api::run().await,
        Command::Ui(args) => cli::ui::run(args).await,
    }
}
