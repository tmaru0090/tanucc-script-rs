use log::debug;
use std::env;
use std::net::TcpStream;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tanucc_script::decoder::interpreter::Decoder;
use tanucc_script::lexer::tokenizer::Lexer;
use tanucc_script::parser::syntax::Node;
use tanucc_script::parser::syntax::Parser;
use tanucc_script::types::SystemType;
use tanucc_script::types::SystemValue;
use tanucc_script::types::SystemValueTuple;

use std::fs::File;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[macro_export]
macro_rules! to_sysval {
    ($struct_name:ident { $($field_name:ident : $field_type:ty),* }) => {
        impl $struct_name {
            pub fn to_system_value(&self) -> SystemValue {
                SystemValue::Struct(vec![
                    $(
                        map_field_to_system_value(&self.$field_name)
                    ),*
                ])
            }
        }

        fn map_field_to_system_value(field: &dyn std::any::Any) -> SystemValue {
            if let Some(val) = field.downcast_ref::<usize>() {
                SystemValue::Usize(*val)
            } else if let Some(val) = field.downcast_ref::<u8>() {
                SystemValue::U8(*val)
            } else if let Some(val) = field.downcast_ref::<u16>() {
                SystemValue::U16(*val)
            } else if let Some(val) = field.downcast_ref::<u32>() {
                SystemValue::U32(*val)
            } else if let Some(val) = field.downcast_ref::<u64>() {
                SystemValue::U64(*val)
            } else if let Some(val) = field.downcast_ref::<i8>() {
                SystemValue::I8(*val)
            } else if let Some(val) = field.downcast_ref::<i16>() {
                SystemValue::I16(*val)
            } else if let Some(val) = field.downcast_ref::<i32>() {
                SystemValue::I32(*val)
            } else if let Some(val) = field.downcast_ref::<i64>() {
                SystemValue::I64(*val)
            } else if let Some(val) = field.downcast_ref::<f32>() {
                SystemValue::F32(*val)
            } else if let Some(val) = field.downcast_ref::<f64>() {
                SystemValue::F64(*val)
            } else if let Some(val) = field.downcast_ref::<String>() {
                SystemValue::String(val.clone())
            } else if let Some(val) = field.downcast_ref::<bool>() {
                SystemValue::Bool(*val)
            } else if let Some(val) = field.downcast_ref::<Vec<SystemValue>>() {
                SystemValue::Array(val.clone())
            } else if let Some(val) = field.downcast_ref::<*mut i32>() {
                unsafe { SystemValue::Pointer(Box::new(SystemValue::I32(**val))) }
            } else if let Some(val) = field.downcast_ref::<*const i8>() {
                unsafe {
                    let c_str = std::ffi::CStr::from_ptr(*val);
                    SystemValue::String(c_str.to_string_lossy().into_owned())
                }
            } else if let Some(val) = field.downcast_ref::<Box<SystemValueTuple>>() {
                SystemValue::Tuple(val.clone())
            } else if let Some(val) = field.downcast_ref::<Vec<SystemValue>>() {
                SystemValue::Struct(val.clone())
            } else if let Some(val) = field.downcast_ref::<File>() {
                SystemValue::System(SystemType::File(Arc::new(Mutex::new(val.try_clone().unwrap()))))
            } else if let Some(val) = field.downcast_ref::<PathBuf>() {
                SystemValue::System(SystemType::PathBuf(val.clone()))
            } else if let Some(val) = field.downcast_ref::<TcpStream>() {
                SystemValue::System(SystemType::TcpStream(Arc::new(Mutex::new(val.try_clone().unwrap()))))
            } else if let Some(val) = field.downcast_ref::<SystemTime>() {
                SystemValue::System(SystemType::SystemTime(*val))
            } else {
                unimplemented!()
            }
        }
    };
}

// 使用例
struct Test {
    n: i32,
    f: f32,
    str: String,
    b: bool,
    c_str: *const i8,
    file: File,
    path: PathBuf,
    time: SystemTime,
}

to_sysval!(Test { n: i32, f: f32, str: String, b: bool, c_str: *const i8, file: File, path: PathBuf, time: SystemTime });

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
        let main_script = "example/main.txt";
        let contents = std::fs::read_to_string(main_script).unwrap();
        let tokens = Lexer::from_tokenize(main_script, contents.clone()).unwrap();
        debug!("tokens: {:?}", tokens.clone());

        let nodes = if let Err(e) = Parser::from_parse(&tokens, main_script, contents.clone()) {
            eprintln!("{}", e);
            return;
        } else if let Ok(v) = Parser::from_parse(&tokens, main_script, contents.clone()) {
            v
        } else {
            Box::new(Node::default())
        };
        /*
        let mut parser = Parser::new(&tokens, "example/main.sc", contents.clone());
        let nodes = parser.parse_single_statement().unwrap();
        */

        debug!("nodes: {:?}", nodes.clone());
        let mut decoder = Decoder::load_scriptd(main_script, &nodes)
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
        { /*
             let c_string = std::ffi::CString::new("Hello, C!").unwrap();
             let file = File::open("../script/std.txt").unwrap();
             let path = PathBuf::from("../script/std.txt");
             let time = SystemTime::now();
             let test = Test {
                 n: 42,
                 f: 3.14,
                 str: "Hello".to_string(),
                 b: true,
                 c_str: c_string.as_ptr(),
                 file,
                 path,
                 time,
             };
             let system_value = test.to_system_value();
             debug!("{:?}", system_value);

             if let SystemValue::Struct(fields) = system_value {
                 for field in fields {
                     match field {
                         SystemValue::System(system_type) => match system_type {
                             SystemType::File(file) => println!("File handle: {:?}", file),
                             SystemType::PathBuf(path) => println!("Path: {:?}", path),
                             SystemType::TcpStream(stream) => println!("Stream: {:?}", stream),
                             SystemType::SystemTime(time) => println!("Time: {:?}", time),
                         },
                         _ => (),
                     }
                 }
             }
             */
        }
    });
}
