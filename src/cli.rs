use anyhow::{anyhow, Context, Result as R};

#[cfg(any(feature = "full", feature = "decoder"))]
use crate::decoder::interpreter::*;
use crate::error::CompilerError;
#[cfg(any(feature = "full", feature = "lexer"))]
use crate::lexer::tokenizer::{Lexer, Token};
#[cfg(any(feature = "full", feature = "parser"))]
use crate::parser::syntax::Node;
#[cfg(any(feature = "full", feature = "parser"))]
use crate::parser::syntax::Parser;
use env_logger;
use log::{debug, info};

use crate::types::*;
use clap::Arg;
use clap::Command;
use clap::CommandFactory;
use clap::Parser as cParser;
use clap::Subcommand as cSubcommand;
use serde_json::to_string_pretty;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::io::{self, BufRead};
use std::path::Path;
use std::vec::Vec;
#[derive(cParser)]
#[command(author="",about = "",version="0.2",long_about = None)]
pub struct Cli {
    #[arg(value_name = "INTERACTIVE MODE", short, long, global = true)]
    interactive_mode: Option<bool>,
    #[arg(value_name = "DEFAULT DIR", short, long, global = true)]
    default_dir: Option<String>,
    #[arg(value_name = "DOC", long, global = true)]
    doc: bool,
    #[arg(value_name = "AST FILE", long, global = true)]
    ast_file: bool,
    #[arg(value_name = "ERROR LOG FILE", long, global = true)]
    error_log_file: bool,
    #[arg(value_name = "MEASURED DECODE TIME", long, global = true)]
    decode_time: bool,
    #[command(subcommand)]
    command: Commands,
}
#[derive(cSubcommand)]
pub enum Commands {
    Run {
        #[arg(value_name = "FILE")]
        file: Option<String>,
    },
}
pub fn run(cli: Cli) -> R<(), String> {
    let mut default_dir = cli.default_dir;
    if default_dir.is_none() {
        let default_script_dir = Path::new("./script");
        env::set_current_dir(&default_script_dir)
            .expect("カレントディレクトリの設定に失敗しました");
    } else {
        let _default_dir = default_dir.unwrap();
        let default_script_dir = Path::new(&_default_dir);
        env::set_current_dir(&default_script_dir)
            .expect("カレントディレクトリの設定に失敗しました");
    }
    match &cli.command {
        Commands::Run { file } => {
            /*デコード*/
            #[cfg(any(feature = "full", feature = "decoder"))]
            let file_name = &file.clone();
            if file_name.is_none() {
                Cli::command().print_help().unwrap();
                std::process::exit(1);
            }
            let mut decoder = Decoder::load_script(&file_name.clone().unwrap())
                .unwrap_or_else(|e| {
                    eprintln!("{}", e);
                    Decoder::new()
                })
                .generate_doc(cli.doc)
                .generate_ast_file(cli.ast_file)
                .generate_error_log_file(cli.error_log_file)
                .measured_decode_time(cli.decode_time);
            #[cfg(any(feature = "full", feature = "decoder"))]
            match decoder.decode() {
                Ok(v) => {
                    debug!("ret: {:?}", v);
                    //           debug!("ast_maps: {:?}", decoder.ast_map());
                    debug!("decode total-time: {:?}", decoder.decode_time())
                }
                Err(e) => eprintln!("{}", e),
            }
        }
        _ => {}
    }
    /*
    /*テスト用*/
    #[cfg(any(feature = "full", feature = "lexer"))]
    {
        let content = std::fs::read_to_string(file_name).map_err(|e| e.to_string())?;
        let tokens = Lexer::from_tokenize(file_name, content.clone())?;
        debug!("tokens: {:?}", tokens.clone());
        #[cfg(any(feature = "full", feature = "parser"))]
        {
            let nodes = Parser::from_parse(&tokens, file_name, content.clone())?;

            debug!("nodes: {:?}", nodes.clone());
        }
    }*/

    Ok(())
}
