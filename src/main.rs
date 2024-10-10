/*
mod context;
mod decoder;
mod error;
mod lexer;
mod memory_mgr;
mod parser;
mod traits;
mod types;
mod cli;

*/
use clap::Parser;

use anyhow::{anyhow, Context, Result as R};

fn main() -> R<(), String> {
    env_logger::init();
    let cli = tanucc_script::cli::Cli::parse();
    tanucc_script::cli::run(cli)?;
    Ok(())
}
