//! CLI module for PMP LLM Gateway
//!
//! Provides subcommands for running the gateway in different modes:
//! - `serve`: API + UI combined (default)
//! - `api`: API server only
//! - `ui`: UI server with optional API proxy

pub mod api;
pub mod serve;
pub mod ui;

use clap::{Parser, Subcommand};

/// PMP LLM Gateway - Unified interface for multiple LLM providers
#[derive(Parser)]
#[command(name = "pmp-llm-gateway")]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Run API + UI server combined (default mode)
    Serve,

    /// Run API server only
    Api,

    /// Run UI server with optional API proxy
    Ui(ui::UiArgs),
}
