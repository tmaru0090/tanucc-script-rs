use log::debug;
use std::env;
use std::path::Path;
use std::process::{Command, Stdio};
use tanucc_script::decoder::interpreter::Decoder;
use tanucc_script::lexer::tokenizer::Lexer;
use tanucc_script::parser::syntax::Node;
use tanucc_script::parser::syntax::Parser;
fn with_env_var<F>(key: &str, value: &str, mut f: F)
where
    F: FnMut(),
{
    env::set_var(key, value);
    f();
    env::remove_var(key);
}

#[test]
fn parse() -> Result<(), String> {
    with_env_var("RUST_LOG", "debug", || {
        env_logger::init();
        let contents = String::from("let a =100;");
        let tokens = Lexer::from_tokenize("", contents.clone()).unwrap();
        let nodes = Parser::from_parse(&tokens, "", contents).unwrap();
        debug!("{:?}", tokens);
        debug!("{:?}", nodes);
    });
    Ok(())
}
// 時期に直接関数呼び出しに変更しろ
#[test]
fn all_default_run() {
    env_logger::init();
    with_env_var("RUST_LOG", "debug", || {
        let default_script_dir = Path::new("./script");
        env::set_current_dir(&default_script_dir)
            .expect("カレントディレクトリの設定に失敗しました");

        #[cfg(any(feature = "full", feature = "decoder"))]
        let contents = std::fs::read_to_string("example/main.sc").unwrap();
        let tokens = Lexer::from_tokenize("example/main.sc", contents.clone()).unwrap();
        debug!("tokens: {:?}", tokens.clone());

        let nodes = if let Err(e) = Parser::from_parse(&tokens, "example/main.sc", contents.clone())
        {
            eprintln!("{}", e);
            return;
        } else if let Ok(v) = Parser::from_parse(&tokens, "example/main.sc", contents.clone()) {
            v
        } else {
            Box::new(Node::default())
        };
        /*
        let mut parser = Parser::new(&tokens, "example/main.sc", contents.clone());
        let nodes = parser.parse_single_statement().unwrap();
        */

        debug!("nodes: {:?}", nodes.clone());
        let mut decoder = Decoder::load_scriptd("example/main.sc", &nodes)
            .unwrap_or_else(|e| {
                eprintln!("{}", e);
                Decoder::new()
            })
            .generate_doc(true)
            .generate_ast_file(true)
            .generate_error_log_file(true)
            .measured_decode_time(true);
        #[cfg(any(feature = "full", feature = "decoder"))]
        match decoder.decode() {
            Ok(v) => {
                debug!("ret: {:?}", v);
                //           debug!("ast_maps: {:?}", decoder.ast_map());
                debug!("decode total-time: {:?}", decoder.decode_time())
            }
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        }
    });
}
