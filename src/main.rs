use clap::Parser;
use pmp_llm_gateway::cli::{self, Cli, Command};

fn main() -> anyhow::Result<()> {
    // Configure tokio runtime with larger stack size for worker threads
    // to avoid stack overflow from large async futures caused by
    // trait object indirection in AppState services
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(16 * 1024 * 1024) // 16MB stack size (default is 2MB)
        .build()?;

    runtime.block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Serve => cli::serve::run().await,
        Command::Api => cli::api::run().await,
        Command::Ui(args) => cli::ui::run(args).await,
    }
}
