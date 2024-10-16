use crate::compile_error;
use crate::compile_group_error;
use crate::context::*;
use crate::error::CompilerError;
use crate::lexer::tokenizer::{Lexer, Token};
use crate::memory_mgr::*;
use crate::parser::syntax::Node;
use crate::parser::syntax::Parser;
use crate::traits::Size;
use crate::types::*;
use c_encode::ToEncode;
use chrono::{DateTime, Local, Utc};
use hostname::get;
use indexmap::IndexMap;
use libloading::{Library, Symbol};
use log::{debug, info};
use property_rs::Property;
use rodio::{source::Source, OutputStream};
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use serde_json::Number;
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Debug;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::io::{Read, Seek, SeekFrom, Write};
use std::ops::{Add, Div, Mul, Sub};
use std::path::PathBuf;
use std::process::{Command, Output};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;
use std::time::UNIX_EPOCH;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::get_codecs;
use symphonia::default::get_probe;
use uuid::Uuid;
use whoami;
#[cfg(target_os = "windows")]
use win_msgbox::{
    AbortRetryIgnore, CancelTryAgainContinue, Icon, MessageBox, Okay, OkayCancel, RetryCancel,
    YesNo, YesNoCancel,
};

fn get_duration(file_path: &str) -> Option<Duration> {
    // ファイルを開く
    let file = File::open(file_path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // ファイル形式を推定
    let hint = Hint::new();
    let format_opts = FormatOptions::default(); // FormatOptions に修正
    let metadata_opts = MetadataOptions::default(); // MetadataOptionsも使用可能

    let probed = get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .ok()?; // FormatOptionsとMetadataOptionsを使用

    // デコーダを作成
    let format = probed.format;

    // トラックを取得
    let track = format.tracks().get(0)?;

    // サンプル数を計算
    let sample_rate = track.codec_params.sample_rate?;
    let samples = track.codec_params.n_frames?;

    // 長さを計算
    let duration = Duration::from_secs_f64(samples as f64 / sample_rate as f64);

    Some(duration)
}
#[cfg(target_os = "windows")]
fn show_messagebox(
    message_type: &str,
    title: &str,
    message: &str,
    icon: Option<&str>,
) -> Option<String> {
    let msg_icon = match icon {
        Some("information") => Icon::Information,
        Some("warning") => Icon::Warning,
        Some("error") => Icon::Error,
        _ => return None, // デフォルトはアイコンなし
    };

    let response = match message_type {
        "okay" => {
            MessageBox::<Okay>::new(message)
                .title(title)
                .icon(msg_icon)
                .show()
                .unwrap();
            Some("okay".to_string())
        }
        "yesno" => {
            let result = MessageBox::<YesNo>::new(message)
                .title(title)
                .icon(msg_icon)
                .show()
                .unwrap();
            match result {
                YesNo::Yes => Some("yes".to_string()),
                YesNo::No => Some("no".to_string()),
            }
        }
        "okaycancel" => {
            let result = MessageBox::<OkayCancel>::new(message)
                .title(title)
                .icon(msg_icon)
                .show()
                .unwrap();
            match result {
                OkayCancel::Okay => Some("okay".to_string()),
                OkayCancel::Cancel => Some("cancel".to_string()),
            }
        }
        "canceltryagaincontinue" => {
            let result = MessageBox::<CancelTryAgainContinue>::new(message)
                .title(title)
                .icon(msg_icon)
                .show()
                .unwrap();
            match result {
                CancelTryAgainContinue::Cancel => Some("cancel".to_string()),
                CancelTryAgainContinue::TryAgain => Some("tryagain".to_string()),
                CancelTryAgainContinue::Continue => Some("continue".to_string()),
            }
        }
        "retrycancel" => {
            let result = MessageBox::<RetryCancel>::new(message)
                .title(title)
                .icon(msg_icon)
                .show()
                .unwrap();
            match result {
                RetryCancel::Retry => Some("retry".to_string()),
                RetryCancel::Cancel => Some("cancel".to_string()),
            }
        }

        _ => {
            MessageBox::<Okay>::new(message)
                .title(title)
                .icon(msg_icon)
                .show()
                .unwrap();
            Some("okay".to_string())
        }
    };

    response
}

// メイン実行環境
#[derive(Debug, Clone, Property)]
pub struct Decoder {
    #[property(get)]
    ast_mod: IndexMap<String, Option<Box<Node>>>, // モジュールごとのAST(モジュール名,アクセス可能なNode)
    #[property(get)]
    ast_map: IndexMap<String, Box<Node>>, // ASTのリスト(ファイル名,Node)
    #[property(get)]
    memory_mgr: MemoryManager, // メモリーマネージャー
    #[property(get)]
    context: Context, // コンテキスト
    #[property(get)]
    file_contents: IndexMap<String, String>, // ファイルの内容(ファイル名,ファイルの内容)
    #[property(get)]
    current_node: Option<(String, Box<Node>)>, // 現在のノード(ファイル名,現在のNode)
    #[property(get)]
    generated_ast_file: bool, // ASTの生成をするかどうか
    system_functions: IndexMap<String, fn(&mut Decoder, &Vec<Node>, &Node) -> Value>,
    #[property(get)]
    generated_error_log_file: bool, // エラーログを生成するかどうか

    #[property(get)]
    measure_decode_time: bool, // 実行時間を計測するかどうか

    #[property(get)]
    decode_time: f32, // 実行時間

    #[property(get)]
    generated_doc: bool,
    #[property(get)]
    entry_func: (bool, String), // main関数の有無(フラグ,見つかった関数名(main|Main))
}
impl Decoder {
    fn check_reserved_words(&self, input: &str, reserved_words: &[&str]) -> Value {
        if reserved_words.contains(&input) {
            return Err(compile_error!(
                "error",
                self.current_node.clone().unwrap().1.line(),
                self.current_node.clone().unwrap().1.column(),
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "'{}' is a reserved word",
                input
            ));
        } else {
            Ok(SystemValue::Null)
        }
    }

    pub fn generate_doc(self, flag: bool) -> Self {
        Decoder {
            generated_doc: flag,
            ..self
        }
    }

    pub fn measured_decode_time(self, flag: bool) -> Self {
        Decoder {
            measure_decode_time: flag,
            ..self
        }
    }
    pub fn generate_ast_file(self, flag: bool) -> Self {
        Decoder {
            generated_ast_file: flag,
            ..self
        }
    }
    pub fn generate_error_log_file(self, flag: bool) -> Self {
        Decoder {
            generated_error_log_file: flag,
            ..self
        }
    }

    fn generate_html_from_comments(&mut self) -> String {
        let mut html = String::from(
        "<!DOCTYPE html>\n<html>\n<head>\n<title>Comments</title>\n<style>\n\
        body { font-family: Arial, sans-serif; }\n\
        .comment-container { margin: 10px; padding: 10px; border: 1px solid #ccc; border-radius: 4px; }\n\
        .comment { margin-bottom: 5px; padding: 5px; background-color: #f9f9f9; border: 1px solid #ddd; border-radius: 4px; }\n\
        .comment-link { color: #007bff; text-decoration: none; }\n\
        .comment-link:hover { text-decoration: underline; }\n\
        .hidden { display: none; }\n\
        .slide { overflow: hidden; transition: max-height 0.3s ease-out; }\n\
        .slide.show { max-height: 500px; }\n\
        .slide.hide { max-height: 0; }\n\
        #search-box { margin: 20px; padding: 10px; border: 1px solid #ddd; border-radius: 4px; width: 300px; }\n\
        .highlight { background-color: #e3f2fd; border: 1px solid #039be5; border-radius: 2px; }\n\
        .dark-mode { background-color: #333; color: #ddd; }\n\
        .dark-mode .comment { background-color: #444; border: 1px solid #555; }\n\
        .dark-mode .comment-link { color: #82b1ff; }\n\
        .pagination { margin: 20px; }\n\
        .pagination a { margin: 0 5px; text-decoration: none; color: #007bff; }\n\
        .pagination a.active { font-weight: bold; }\n\
        </style>\n</head>\n<body>\n",
    );

        // ダークモード切替ボタンの追加
        html.push_str(
            "<button onclick=\"toggleDarkMode()\">Toggle Dark Mode</button>\n\
        <div id=\"search-container\">\n\
        <input type=\"text\" id=\"search-box\" placeholder=\"Search comments...\"></input>\n\
        </div>\n",
        );

        html.push_str("<div id=\"comments\">\n");

        // コメントを行ごとに処理
        for ((line, column), comments) in &self.context.comment_lists {
            html.push_str(&format!(
            "<div class=\"comment-container\">\n\
            <a href=\"#\" class=\"comment-link\" onclick=\"toggleComments({},{})\">Line {} Column {}</a>\n",
            line, column, line, column
        ));

            html.push_str("<div id=\"comments-");
            html.push_str(&format!(
                "{:02}-{:02}\" class=\"slide hide\">",
                line, column
            ));
            for comment in comments {
                html.push_str(&format!(
                    "<div class=\"comment\" data-comment=\"{}\">{}</div>\n",
                    comment, comment
                ));
            }
            html.push_str("</div>\n</div>\n");
        }

        // ページネーション
        html.push_str(
            "<div class=\"pagination\">\n\
        <a href=\"#\" class=\"active\">1</a>\n\
        <a href=\"#\">2</a>\n\
        <a href=\"#\">3</a>\n\
        <!-- Add more pages as needed -->\n\
        </div>\n",
        );

        // JavaScriptでコメント表示を制御、検索機能、ダークモード切り替え、ページネーション
        html.push_str(
        "<script>\n\
        function toggleComments(line, column) {\n\
            var commentsDiv = document.getElementById('comments-' + String(line).padStart(2, '0') + '-' + String(column).padStart(2, '0'));\n\
            if (commentsDiv.classList.contains('hide')) {\n\
                commentsDiv.classList.remove('hide');\n\
                commentsDiv.classList.add('show');\n\
            } else {\n\
                commentsDiv.classList.remove('show');\n\
                commentsDiv.classList.add('hide');\n\
            }\n\
        }\n\
        document.getElementById('search-box').addEventListener('input', function() {\n\
            var searchValue = this.value.toLowerCase();\n\
            var comments = document.querySelectorAll('.comment');\n\
            comments.forEach(function(comment) {\n\
                var commentText = comment.textContent.toLowerCase();\n\
                var highlightedText = commentText;\n\
                if (searchValue && commentText.includes(searchValue)) {\n\
                    var regex = new RegExp('(' + searchValue + ')', 'gi');\n\
                    highlightedText = commentText.replace(regex, '<span class=\"highlight\">$1</span>');\n\
                    comment.innerHTML = highlightedText;\n\
                } else {\n\
                    comment.innerHTML = commentText;\n\
                }\n\
                comment.style.display = commentText.includes(searchValue) ? '' : 'none';\n\
            });\n\
        });\n\
        function toggleDarkMode() {\n\
            document.body.classList.toggle('dark-mode');\n\
        }\n\
        </script>\n"
    );

        html.push_str("</body>\n</html>");

        html
    }

    // 現在のASTのマップの先頭に指定スクリプトのASTを追加
    pub fn add_first_ast_from_file(&mut self, file_name: &str) -> R<&mut Self, String> {
        let content = std::fs::read_to_string(file_name).map_err(|e| e.to_string())?;
        let tokens = Lexer::from_tokenize(file_name, content.clone())?;
        let nodes = Parser::from_parse(&tokens, file_name, content.clone())?;
        // 最初に要素を挿入するために新しい IndexMap を作る
        let mut new_ast_map = IndexMap::new();
        new_ast_map.insert(file_name.to_string(), nodes.clone());

        // 既存の ast_map の要素を新しいものに追加
        new_ast_map.extend(self.ast_map.drain(..));

        // ast_map を新しいものに置き換える
        self.ast_map = new_ast_map;

        Ok(self)
    }

    // 現在のASTのマップに指定スクリプトのASTを追加
    pub fn add_ast_from_file(&mut self, file_name: &str) -> R<&mut Self, String> {
        let content = std::fs::read_to_string(file_name).map_err(|e| e.to_string())?;
        let tokens = Lexer::from_tokenize(file_name, content.clone())?;
        let nodes = Parser::from_parse(&tokens, file_name, content.clone())?;
        self.ast_map.insert(file_name.to_string(), nodes.clone());
        Ok(self)
    }

    // 現在のASTのマップに文字列でスクリプトのASTを追加
    pub fn add_ast_from_text(&mut self, file_name: &str, content: &str) -> R<&mut Self, String> {
        // トークン化処理
        let tokens = Lexer::from_tokenize(file_name, content.to_string())?;

        // パース処理
        let nodes = Parser::from_parse(&tokens, file_name, content.to_string())?;

        // ASTをマップに追加
        self.ast_map.insert(file_name.to_string(), nodes.clone());
        //panic!("ast_map: {:?}",self.ast_map.clone());

        // 成功時にselfを返す
        Ok(self)
    }
    // スクリプトを読み込む
    pub fn load_script(file_name: &str) -> R<Self, String> {
        let mut ast_map: IndexMap<String, Box<Node>> = IndexMap::new();
        let file_content = std::fs::read_to_string(file_name)
            .map_err(|e| e.to_string())
            .expect("Failed to script file");

        let tokens = Lexer::from_tokenize(file_name, file_content.clone())?;
        let nodes = Parser::from_parse(&tokens, file_name, file_content.clone())?;
        ast_map.insert(file_name.to_string(), nodes.clone());
        Ok(Decoder {
            generated_doc: false,
            ast_mod: IndexMap::new(),
            ast_map,
            memory_mgr: MemoryManager::new(1024 * 1024),
            file_contents: IndexMap::new(),
            current_node: None,
            context: Context::new(),
            generated_ast_file: false,
            generated_error_log_file: false,
            measure_decode_time: false,
            decode_time: 0.0,
            entry_func: (false, String::new()),
            system_functions: IndexMap::new(),
        })
    }
    // スクリプトを読み込む(デバッグ用)
    pub fn load_scriptd(file_name: &str, nodes: &Box<Node>) -> R<Self, String> {
        let mut ast_map: IndexMap<String, Box<Node>> = IndexMap::new();
        let file_content = std::fs::read_to_string(file_name)
            .map_err(|e| e.to_string())
            .expect("Failed to script file");
        ast_map.insert(file_name.to_string(), nodes.clone());
        Ok(Decoder {
            generated_doc: false,
            ast_mod: IndexMap::new(),
            ast_map,
            memory_mgr: MemoryManager::new(1024 * 1024),
            file_contents: IndexMap::new(),
            current_node: None,
            context: Context::new(),
            generated_ast_file: false,
            generated_error_log_file: false,
            measure_decode_time: false,
            decode_time: 0.0,
            entry_func: (false, String::new()),
            system_functions: IndexMap::new(),
        })
    }

    pub fn new() -> Self {
        Self {
            generated_doc: false,
            ast_mod: IndexMap::new(),
            ast_map: IndexMap::new(),
            memory_mgr: MemoryManager::new(1024 * 1024),
            file_contents: IndexMap::new(),
            current_node: None,
            context: Context::new(),
            generated_ast_file: false,
            generated_error_log_file: false,
            measure_decode_time: false,
            decode_time: 0.0,
            entry_func: (false, String::new()),
            system_functions: IndexMap::new(),
        }
    }

    pub fn decode(&mut self) -> Result<SystemValue, String> {
        // 実行にかかった時間を計測
        let start_time = if self.measure_decode_time {
            Some(Instant::now())
        } else {
            None
        };
        let mut value = SystemValue::Null;
        let original_node = self.current_node.clone();

        // ASTを評価して実行
        let mut evaluated_files = std::collections::HashSet::new();
        let ast_map_clone = self.ast_map.clone(); // クローンを作成
        #[cfg(feature = "wip-system")]
        {
            // システム関数の登録
            {
                self.register_system_all_functions();
            }
        }
        // Nodeのイテレータを使用してノードを処理
        for (file_name, node) in ast_map_clone.iter() {
            if evaluated_files.contains(file_name) {
                continue; // 既に評価済みのファイルはスキップ
            }
            evaluated_files.insert(file_name.clone());

            self.current_node = Some((file_name.clone(), Box::new(Node::default())));
            let content = std::fs::read_to_string(file_name.clone()).map_err(|e| e.to_string())?;
            self.file_contents.insert(file_name.clone(), content);

            // Nodeのイテレータを使用してノードを処理
            for current_node in node.iter() {
                let current_node = current_node.borrow(); // 借用して中身にアクセス
                self.current_node = Some((file_name.clone(), Box::new(current_node.clone())));
                if let NodeValue::EndStatement | NodeValue::Null = current_node.value {
                    continue;
                }
                value = self.execute_node(&*current_node)?; // 修正: 借用を渡す
            }
        }

        // メインエントリーが定義されていたら実行
        if self.entry_func.0 {
            self.add_ast_from_text("main-entry", &format!("{}();", self.entry_func.1))?;
            if let Some((_, value_node)) = self.ast_map.clone().iter().last() {
                for current_node in value_node.iter() {
                    let current_node = current_node.borrow(); // 借用して中身にアクセス
                    if let NodeValue::EndStatement | NodeValue::Null = current_node.value {
                        continue;
                    }
                    value = self.execute_node(&*current_node)?; // 修正: 借用を渡す
                }
            }
        }

        self.current_node = original_node;

        if self.generated_ast_file {
            /*   // ディレクトリが存在しない場合は作成
                std::fs::create_dir_all("./script-analysis").map_err(|e| e.to_string())?;
                // IndexMapをHashMapに変換
                let ast_map: std::collections::HashMap<_, _> =
                    self.ast_map.clone().into_iter().collect();
                let ast_json = serde_json::to_string_pretty(&ast_map).map_err(|e| e.to_string())?;
                std::fs::write("./script-analysis/ast.json", ast_json).map_err(|e| e.to_string())?;
            */
        }
        if self.generated_doc {
            let html_doc = self.generate_html_from_comments();
            std::fs::create_dir_all("./script-doc").map_err(|e| e.to_string())?;
            std::fs::write("./script-doc/doc.html", html_doc).map_err(|e| e.to_string())?;
        }
        if let Some(start) = start_time {
            let duration = start.elapsed();
            // 秒とナノ秒を取得
            let secs = duration.as_secs() as f32;
            let nanos = duration.subsec_nanos() as f32;
            self.decode_time = secs + (nanos / 1_000_000_000.0);
        }
        Ok(value)
    }

    fn eval_block(&mut self, block: &Vec<Box<Node>>) -> Result<SystemValue, String> {
        let mut result = SystemValue::Null;
        let initial_local_context = self.context.local_context.clone(); // 現在のローカルコンテキストを保存

        for _b in block {
            for b in _b.iter() {
                let b = b.borrow(); // Rc<RefCell<Node>>の借用を取得

                // ノードを評価
                result = self.execute_node(&*b)?; // 借用を渡す
                                                  // return 文が評価された場合、ブロックの評価を終了
                if let NodeValue::ControlFlow(ControlFlow::Return(_)) = b.value() {
                    break;
                }
                // EndStatementがある場合に処理を中断していないか確認
                if let NodeValue::EndStatement = b.value() {
                    continue; // EndStatementは単なる区切りなので、次のノードの評価を続行
                }
                if let NodeValue::MultiComment(_, _) = b.value() {
                    continue;
                }
                if let NodeValue::SingleComment(_, _) = b.value() {
                    continue;
                }
            }
        }

        // ブロックの処理が終わったらローカルコンテキストを元に戻す
        self.context.local_context = initial_local_context;
        Ok(result)
    }

    fn add_extension_if_missing(&self, path: &str, extension: &str) -> PathBuf {
        let path_buf = PathBuf::from(path);
        let absolute_path = std::fs::canonicalize(&path_buf).unwrap_or(path_buf);

        let mut path_buf = absolute_path.clone();
        if path_buf.extension().is_none() {
            path_buf.set_extension(extension);
        }
        path_buf
    }
    fn eval_include(&mut self, file_name: &String) -> Result<SystemValue, String> {
        let path = self.add_extension_if_missing(file_name, "txt");
        self.add_first_ast_from_file(&path.to_string_lossy().into_owned())?;

        let ast_map = self.ast_map.clone();
        let _node = ast_map.get(&path.to_string_lossy().into_owned()).unwrap();
        let mut result = SystemValue::Null;
        let mut current_node = _node.clone();
        self.current_node = Some((path.to_string_lossy().into_owned().clone(), _node.clone()));

        for node in current_node.iter() {
            let node = node.borrow(); // Rc<RefCell<Node>>の借用を取得
            result = self.execute_node(&*node)?; // 借用を渡す
        }

        Ok(result)
    }

    fn eval_use(&mut self, scope_resolution: &Box<Node>) -> Result<SystemValue, String> {
        let mut imports = match scope_resolution.value {
            NodeValue::ScopeResolution(ref v) => v.clone(),
            _ => todo!(),
        };
        let mut first_mod_name = String::new();
        debug!("Use imports {:?}", imports);
        for (i, import) in imports.iter().enumerate() {
            match import.value {
                NodeValue::ScopeResolution(ref v) => {
                    if i == 0 {
                        let mods = v.clone();
                        for _mod in &mods {
                            first_mod_name = match &_mod.value {
                                NodeValue::Variable(_, ref name, _, _, _) => name.clone(),
                                _ => todo!(),
                            };
                            //panic!("{:?}",first_mod_name);
                        }
                    }
                    debug!("Use import name: {:?}", v.clone());
                }
                NodeValue::DataType(DataType::String(ref v)) => {
                    if !first_mod_name.is_empty() {
                        first_mod_name.push_str(".txt");
                        let file_name = first_mod_name.clone();
                        let content =
                            std::fs::read_to_string(&file_name).map_err(|e| e.to_string())?;
                        let tokens = Lexer::from_tokenize(&file_name, content.clone())?;
                        let _nodes = Parser::from_parse(&tokens, &file_name, content.clone())?;
                        //        panic!("{:?}", _nodes);
                        // 公開された変数のみをフィルタリング
                        let mut public_nodes: Vec<Rc<RefCell<Node>>> = _nodes
                            .iter()
                            .filter(|node| match &node.borrow().value {
                                NodeValue::Declaration(Declaration::Variable(
                                    _,
                                    _,
                                    _,
                                    _,
                                    _,
                                    ref _is_public,
                                )) => {
                                    let is_public = *_is_public;
                                    //debug!("{:?}", is_public);
                                    is_public
                                }
                                NodeValue::Declaration(Declaration::Type(_, _, ref _is_public)) => {
                                    let is_public = *_is_public;
                                    //debug!("{:?}", is_public);
                                    is_public
                                }

                                //  Function(String,Vec<(Box<Node>, String)>,Box<Node>,Box<Node>,bool ,bool,), // 関数定義(関数名,引数,戻り値の型,ボディ,システム関数フラグ,パブリックフラグ)
                                NodeValue::Declaration(Declaration::Function(
                                    _,
                                    _,
                                    _,
                                    _,
                                    _,
                                    ref _is_public,
                                )) => {
                                    let is_public = *_is_public;
                                    //debug!("{:?}", is_public);
                                    is_public
                                }
                                NodeValue::Declaration(Declaration::Struct(
                                    _,
                                    _,
                                    ref _is_public,
                                )) => {
                                    let is_public = *_is_public;
                                    //debug!("{:?}", is_public);
                                    is_public
                                }
                                NodeValue::Declaration(Declaration::Impl(_, ref members)) => {
                                    let mut is_public = false;
                                    for member in members {
                                        match member.value {
                                            NodeValue::Declaration(Declaration::Function(
                                                _,
                                                _,
                                                _,
                                                _,
                                                _,
                                                ref _is_public,
                                            )) => {
                                                is_public = *_is_public;
                                                //debug!("{:?}", is_public);
                                            }
                                            _ => (),
                                        }
                                    }

                                    is_public
                                }

                                _ => false,
                            })
                            .map(|node| Rc::clone(&node))
                            .collect();
                        //public_nodes.reverse();
                        //debug!("{:?}",public_nodes);
                        if !public_nodes.is_empty() {
                            let boxed_node: Box<Node> = Box::from(
                                public_nodes
                                    .iter()
                                    .map(|v| v.borrow().clone())
                                    .collect::<Vec<Node>>(),
                            );
                            self.ast_map.insert(file_name.to_string(), boxed_node);

                            let _node = self.ast_map.get(&file_name).unwrap();
                            self.current_node = Some((file_name.clone(), _node.clone()));

                            let mut result = SystemValue::Null;
                            for node in _node.iter() {
                                let node = node.borrow();
                                result = self.execute_node(&*node)?;
                            }
                            return Ok(result);
                        }
                    }
                }
                _ => todo!(),
            }
        }
        return Ok(SystemValue::Null);
    }

    fn eval_single_comment(&mut self, content: &String, lines: &(usize, usize)) -> Value {
        self.context
            .comment_lists
            .insert((lines.0, lines.1), vec![content.clone()]);
        debug!("MultiComment added at line {}, column {}", lines.0, lines.1);
        Ok(SystemValue::Null)
    }
    fn eval_multi_comment(&mut self, content: &Vec<String>, lines: &(usize, usize)) -> Value {
        let mut result = SystemValue::Null;
        self.context
            .comment_lists
            .insert((lines.0, lines.1), content.clone().to_vec());
        debug!("MultiComment added at line {}, column {}", lines.0, lines.1);
        Ok(result)
    }

    fn eval_array(
        &mut self,
        data_type: &Box<Node>,
        values: &Vec<Box<Node>>,
    ) -> Result<SystemValue, String> {
        // 型を評価
        let v_type = match data_type.value {
            NodeValue::DataType(ref d) => self.execute_node(&Node {
                value: NodeValue::DataType(d.clone()),
                ..Node::default()
            })?,
            _ => SystemValue::Null,
        };

        // 各値を評価し、型チェックを行う
        let mut array = Vec::new();
        for _value in values {
            // _valueはBox<Node>なので、borrow()は不要
            for value in _value.iter() {
                let value_borrowed = value.borrow(); // Rc<RefCell<Node>>の借用を取得
                let v_value = self.execute_node(&*value_borrowed)?; // 借用を渡す
                                                                    //        if v_value == SystemValue::Null {
                                                                    //          continue;
                                                                    //     }
                array.push(SystemValue::Pointer(Box::new(v_value)));
            }
        }

        // 配列全体をヒープにコピー
        self.memory_mgr.allocate(array.clone());
        // 結果を返す
        Ok(SystemValue::Array(array))
    }

    fn eval_assign(
        &mut self,
        node: &Node,
        var_name: &Box<Node>,
        value: &Box<Node>,
        index: &Box<Node>,
    ) -> Result<SystemValue, String> {
        let mut result = SystemValue::Null;

        // ステートメントフラグのチェック
        if !node.is_statement {
            return Err(compile_error!(
                "error",
                self.current_node.clone().unwrap().1.line,
                self.current_node.clone().unwrap().1.column,
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "Variable Assign must be a statement"
            ));
        }

        let name = match var_name.value {
            NodeValue::Variable(_, ref v, _, _, _) => v.clone(),
            _ => String::new(),
        };

        let variable_data = self
            .context
            .local_context
            .get(&name)
            .cloned()
            .or_else(|| self.context.global_context.get(&name).cloned());

        if let Some(mut variable) = variable_data {
            if variable.is_mutable {
                let new_value = self.execute_node(&value)?;
                match &mut variable.value {
                    SystemValue::Array(ref mut array) => {
                        let index_value = self.execute_node(&index)?;
                        if let SystemValue::I32(n) = index_value {
                            let index_usize = n as usize;
                            if index_usize < array.len() {
                                array[index_usize] =
                                    SystemValue::Pointer(Box::new(new_value.clone()));
                                result = new_value.clone();
                            } else {
                                return Err(compile_error!(
                                    "error",
                                    self.current_node.clone().unwrap().1.line,
                                    self.current_node.clone().unwrap().1.column,
                                    &self.current_node.clone().unwrap().0,
                                    &self
                                        .file_contents
                                        .get(&self.current_node.clone().unwrap().0)
                                        .unwrap(),
                                    "Index out of bounds"
                                ));
                            }
                        } else {
                            return Err(compile_error!(
                                "error",
                                self.current_node.clone().unwrap().1.line,
                                self.current_node.clone().unwrap().1.column,
                                &self.current_node.clone().unwrap().0,
                                &self
                                    .file_contents
                                    .get(&self.current_node.clone().unwrap().0)
                                    .unwrap(),
                                "Index is not a number"
                            ));
                        }
                    }
                    _ => {
                        variable.value = new_value.clone();
                        result = new_value.clone();
                    }
                }

                self.memory_mgr
                    .update_value(variable.address.clone(), variable.value.clone());

                if self.context.local_context.contains_key(&name) {
                    self.context.local_context.insert(name.clone(), variable);
                } else {
                    self.context.global_context.insert(name.clone(), variable);
                }

                debug!("Assign: name = {:?}, new_value = {:?}", name, result);
                Ok(result)
            } else {
                Err(compile_error!(
                    "error",
                    self.current_node.clone().unwrap().1.line,
                    self.current_node.clone().unwrap().1.column,
                    &self.current_node.clone().unwrap().0,
                    &self
                        .file_contents
                        .get(&self.current_node.clone().unwrap().0)
                        .unwrap(),
                    "Variable '{}' is not mutable",
                    name
                ))
            }
        } else {
            Err(compile_error!(
                "error",
                self.current_node.clone().unwrap().1.line,
                self.current_node.clone().unwrap().1.column,
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "Variable '{}' is not defined",
                name
            ))
        }
    }
    fn eval_function(
        &mut self,
        name: &String,
        args: &Vec<(Box<Node>, String)>,
        body: &Box<Node>,
        return_type: &Box<Node>,
        is_system: &bool,
        is_public: &bool,
    ) -> Value {
        debug!(
            "eval_function: name: {:?} args: {:?} body: {:?} return_type: {:?} is_system: {:?}",
            name.clone(),
            args.clone(),
            body.clone(),
            return_type.clone(),
            is_system
        );
        let func_name = name; // すでに String 型なのでそのまま使う
                              //   debug!("{:?}", func_name.clone());
        if func_name == "main" || func_name == "Main" {
            self.entry_func.0 = true;
            self.entry_func.1 = func_name.clone();
        }
        self.check_reserved_words(&func_name, RESERVED_WORDS)?;

        // 関数がすでに定義されているかチェック
        if self.context.global_context.contains_key(func_name.as_str()) {
            return Err(compile_error!(
                "error",
                self.current_node.clone().unwrap().1.line(),
                self.current_node.clone().unwrap().1.column(),
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "Function '{}' is already defined",
                func_name
            ));
        }

        let mut arg_addresses = SystemValue::Array(vec![]);
        let func_index = self.memory_mgr.allocate(func_name.clone());

        for (data_type, arg_name) in args.iter() {
            let _data_type = SystemValue::__NodeBlock(vec![data_type.clone()]);
            let _arg_name = SystemValue::String(arg_name.clone());
            arg_addresses.push(SystemValue::Array(vec![_data_type, _arg_name]));
        }

        let _body = SystemValue::__NodeBlock(vec![body.clone()]);

        let _return_type = SystemValue::__NodeBlock(vec![return_type.clone()]);

        // 関数の情報をまとめて保存
        let func_info = SystemValue::Struct(vec![
            SystemValue::Array(vec![arg_addresses.clone()]),
            _body,
            _return_type,
        ]);
        let func_info_index = self.memory_mgr.allocate(func_info.clone());

        if *is_system {
            // 関数の情報をグローバルコンテキストに保存
            self.context.global_context.insert(
                format!("@{}", func_name.clone()),
                Variable {
                    value: func_info.clone(),
                    data_name: String::from(""),
                    address: func_info_index,
                    is_mutable: false,
                    size: func_info.size(),
                },
            );
        }
        // 関数の情報をグローバルコンテキストに保存
        self.context.global_context.insert(
            func_name.clone(),
            Variable {
                value: func_info.clone(),
                data_name: String::from(""),
                address: func_info_index,
                is_mutable: false,
                size: 0,
            },
        );

        debug!(
            "FunctionDeclaration: is_public = {:?}, name = {:?}, args = {:?}, body = {:?}, return_type = {:?}",
            is_public,func_name, arg_addresses, body, return_type
        );
        Ok(SystemValue::Null)
    }

    fn __system_fn_syntax<'a>(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        if args.len() != 2 {
            return Err("syntax expects 2 arguments".into());
        }
        let syntax_name = match args[0].value {
            NodeValue::DataType(DataType::String(ref v)) => v.clone(),
            _ => String::new(),
        };
        let format = match args[1].value {
            NodeValue::DataType(DataType::String(ref v)) => v.clone(),
            _ => String::new(),
        };
        let mut content = String::new();
        let line = node.line();
        let column = node.column();
        let syntax_node = Box::new(Node::new(
            NodeValue::DataType(DataType::String(format.clone())),
            None,
            line,
            column,
        ));
        let user_syntax =
            Parser::<'a>::new_user_syntax(syntax_name.clone(), syntax_node.clone(), line, column);

        if let Some((last_key, mut last_value)) = self.ast_map.pop() {
            if let Some(mut _node) = last_value.iter_mut().next() {
                _node.borrow_mut().next = Rc::new(RefCell::new(Some(user_syntax.clone())));

                // 借用してBoxに変換
                let node_box = Box::new(_node.borrow().clone()); // _nodeを借用し、clone()で新しいBoxを作成
                self.ast_map.insert(last_key, node_box); // Boxを挿入
            }
        }

        debug!("Syntax registered: {:?}", user_syntax.clone());
        // debug!("current_node: {:?}", self.current_node());
        //debug!("new ast_maps: {:?}", self.ast_map.clone());
        return Ok(SystemValue::Null);
    }

    fn __system_fn_as_ptr(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        if args.len() != 1 {
            return Err("asptr expects no arguments".into());
        }
        let mut evaluated_args = Vec::new();
        for arg in args.clone() {
            let evaluated_arg = self.execute_node(&arg)?;
            //debug!("args: {:?}", evaluated_arg);
            evaluated_args.push(evaluated_arg);
        }
        let value = &evaluated_args[0];
        return Ok(SystemValue::Pointer(Box::new(value.clone())));
    }

    fn __system_fn_as(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        if args.len() != 2 {
            return Err("as expects no arguments".into());
        }
        let mut evaluated_args = Vec::new();
        for arg in args.clone() {
            let evaluated_arg = self.execute_node(&arg)?;
            //debug!("args: {:?}", evaluated_arg);
            evaluated_args.push(evaluated_arg);
        }
        let mut data_name = if let SystemValue::String(ref v) = &evaluated_args[1] {
            v.clone()
        } else {
            String::new()
        };
        return Ok(SystemValue::Null);
    }
    fn __system_fn_line(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        if !args.is_empty() {
            return Err("line expects no arguments".into());
        }
        let line = node.line().into();
        return Ok(line);
    }
    fn __system_fn_column(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        if !args.is_empty() {
            return Err("column expects no arguments".into());
        }
        let column = node.column().into();
        return Ok(column);
    }

    fn __system_fn_to_cstring(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        if args.len() != 1 {
            return Err("as expects no one arguments".into());
        }
        let mut evaluated_args = Vec::new();
        for arg in args.clone() {
            let evaluated_arg = self.execute_node(&arg)?;
            //debug!("args: {:?}", evaluated_arg);
            evaluated_args.push(evaluated_arg);
        }
        let mut string = if let SystemValue::String(ref v) = &evaluated_args[0] {
            v.clone()
        } else {
            String::new()
        };
        let c_string = string.to_cstring();
        return Ok(SystemValue::System(SystemType::CString(c_string.clone())));
    }
    fn __system_fn_printf(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        if args.len() != 2 {
            return Err("printf expects no one arguments".into());
        }
        //panic!("{:?}",self.execute_node(&args[0])?);
        let fmt = match self.execute_node(&args[0])? {
            SystemValue::System(SystemType::CString(ref v)) => v.clone(),
            _ => todo!(),
        };
        let string = match self.execute_node(&args[1])? {
            SystemValue::System(SystemType::CString(ref v)) => v.clone(),
            _ => todo!(),
        };
        unsafe {
            libc::printf(fmt.as_ptr(), string.as_ptr());
        }
        return Ok(SystemValue::Null);
    }

    fn __system_fn_file(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        if !args.is_empty() {
            return Err("file expects no arguments".into());
        }
        let file = self.current_node().unwrap().0.into();
        return Ok(file);
    }
    fn __system_fn_list_files(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        if args.len() != 1 {
            return Err("list_files expects exactly one argument".into());
        }

        let dir = match self.execute_node(&args[0])? {
            SystemValue::String(v) => v,
            _ => return Err("list_files expects a string as the file name".into()),
        };

        let mut paths = Vec::new(); // ファイルパスを格納するベクタ
                                    // ディレクトリの内容を読み込み
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        // ファイルのみを対象とする場合
                        paths.push(path.to_string_lossy().into_owned());
                    }
                }
            }
        }

        return Ok(paths.into());
    }

    fn __system_fn_play_music(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        if args.len() != 1 {
            return Err("play_music expects exactly one argument".into());
        }
        let file_path = match self.execute_node(&args[0])? {
            SystemValue::String(v) => v,
            _ => return Err("play_music expects a string as the file name".into()),
        };

        // 出力ストリームを作成
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let file = File::open(file_path.clone()).unwrap();

        // 音楽ファイルを読み込む
        let source = rodio::Decoder::new(BufReader::new(file)).unwrap();

        // 音楽の長さを取得
        let duration = get_duration(&file_path).unwrap_or_else(|| {
            // 長さが取れない場合は rodio で推定
            source.total_duration().unwrap()
        });

        // 音楽を再生
        let sink = rodio::Sink::try_new(&stream_handle).unwrap();
        sink.append(source);

        // 再生中にスリープ
        std::thread::sleep(duration);
        return Ok(SystemValue::Null);
    }
    fn __system_fn_str(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        if args.len() != 1 {
            return Err("to_str expects exactly one argument".into());
        }
        let n = match self.execute_node(&args[0])? {
            SystemValue::I64(v) => v,
            _ => return Err("to_str expects a string as the file name".into()),
        };
        let string = n.to_string();
        return Ok(string.into());
    }
    #[cfg(target_os = "windows")]
    fn __system_fn_show_msg_box(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        if args.len() != 4 {
            return Err("show_msg_box expects exactly two arguments".into());
        }
        let message_type = match self.execute_node(&args[0])? {
            SystemValue::String(v) => v,
            _ => return Err("show_msg_box expects a string as the file name".into()),
        };
        let title = match self.execute_node(&args[1])? {
            SystemValue::String(v) => v,
            _ => return Err("show_msg_box expects a string as the file name".into()),
        };
        let message = match self.execute_node(&args[2])? {
            SystemValue::String(v) => v,
            _ => return Err("show_msg_box expects a string as the file name".into()),
        };
        let icon = match self.execute_node(&args[3])? {
            SystemValue::String(v) => Some(v),
            _ => None,
        };
        let responce = show_messagebox(&message_type, &title, &message, icon.as_deref());
        return Ok(responce.unwrap().into());
    }
    fn __system_fn_open_recent(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        if !args.is_empty() {
            return Err("open_recent expects no arguments".into());
        }

        // 最近使用したアイテムフォルダのパス
        let recent_folder = std::env::var("APPDATA").unwrap() + r"\Microsoft\Windows\Recent";

        // フォルダ内のファイルを取得
        let paths = std::fs::read_dir(recent_folder)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path().to_string_lossy().into_owned())
            .collect::<Vec<String>>();

        return Ok(paths.into());
    }

    fn __system_fn_sleep(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        if args.len() != 1 {
            return Err("sleep expects exactly one argument".into());
        }
        let duration = match self.execute_node(&args[0])? {
            SystemValue::U64(v) => v,
            _ => return Err("read_file expects a string as the file name".into()),
        };
        sleep(std::time::Duration::from_secs(duration));
        return Ok(SystemValue::Null);
    }

    fn __system_fn_exit(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        if args.len() != 1 {
            return Err("exit expects exactly one argument".into());
        }
        let status = match self.execute_node(&args[0])? {
            SystemValue::I64(n) => n,
            _ => return Err("exit expects a number as the status".into()),
        };
        std::process::exit(status.try_into().unwrap());

        return Ok(SystemValue::Null);
    }

    fn __system_fn_args(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        if !args.is_empty() {
            return Err("args expects no arguments".into());
        }
        let args: Vec<String> = std::env::args().collect();
        let value: SystemValue = SystemValue::from(args);
        return Ok(value);
    }

    fn __system_fn_cmd(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        let mut evaluated_args = Vec::new();
        for arg in args.clone() {
            let evaluated_arg = self.execute_node(&arg)?;
            //debug!("args: {:?}", evaluated_arg);
            evaluated_args.push(evaluated_arg);
        }

        if evaluated_args.len() < 1 {
            return Err("cmd expects at least one argument".into());
        }
        let command = match &evaluated_args[0] {
            SystemValue::String(v) => v.clone(),
            _ => return Err("cmd expects the first argument to be a string".into()),
        };
        let command_args = if evaluated_args.len() > 1 {
            match &evaluated_args[1] {
                SystemValue::Array(v) => v
                    .iter()
                    .filter_map(|item| match item {
                        SystemValue::Pointer(v) => {
                            if let SystemValue::String(s) = *v.clone() {
                                Some(s.clone())
                            } else if let SystemValue::System(SystemType::PathBuf(ref v)) =
                                *v.clone()
                            {
                                Some(v.to_str().unwrap().to_string())
                            } else {
                                None
                            }
                        }
                        _ => None,
                    })
                    .collect(),
                _ => return Err("cmd expects the second argument to be an array of strings".into()),
            }
        } else {
            Vec::new()
        };
        let output = Command::new(command)
            .args(&command_args)
            .output()
            .expect("外部コマンドの実行に失敗しました");
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Ok(SystemValue::Array(vec![
            SystemValue::Pointer(Box::new(SystemValue::String(stdout))),
            SystemValue::Pointer(Box::new(SystemValue::String(stderr))),
        ]));
    }
    fn __system_fn_read_file(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        if args.len() != 1 {
            return Err("read_file expects exactly one argument".into());
        }
        let file_name = match self.execute_node(&args[0])? {
            SystemValue::String(v) => v,
            _ => return Err("read_file expects a string as the file name".into()),
        };
        let mut file = File::open(file_name).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        return Ok(SystemValue::String(contents));
    }

    fn __system_fn_write_file(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        if args.len() != 2 {
            return Err("write_file expects exactly two arguments".into());
        }

        let mut file: Option<Arc<Mutex<File>>> = None;
        let file_name = match self.execute_node(&args[0])? {
            SystemValue::String(v) => v,
            SystemValue::System(SystemType::File(ref _file)) => {
                file = Some(_file.clone());

                String::new()
            }

            _ => return Err("write_file expects a string as the file name".into()),
        };
        let content = match self.execute_node(&args[1])? {
            SystemValue::String(v) => v,
            SystemValue::Array(arr) => arr
                .into_iter()
                .map(|v| match v {
                    SystemValue::String(s) => Ok::<String, String>(s),
                    SystemValue::Pointer(p) => {
                        if let SystemValue::String(s) = *p {
                            Ok(s)
                        } else {
                            Err("write_file expects an array of strings as the content".into())
                        }
                    }
                    _ => Err("write_file expects an array of strings as the content".into()),
                })
                .collect::<Result<Vec<String>, String>>()?
                .join("\n"),
            _ => {
                return Err(
                    "write_file expects a string or an array of strings as the content".into(),
                )
            }
        };
        let mut file = File::create(file_name).unwrap();
        file.write_all(content.as_bytes()).unwrap();

        return Ok(SystemValue::Null);
    }

    fn __system_fn_system_function_lists(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        if !args.is_empty() {
            return Err("system function lists expects exactly no argument".into());
        }
        let lists: Vec<String> = self.system_functions.keys().cloned().collect();
        return Ok(lists.into());
    }

    fn __system_fn_print(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        if args.is_empty() {
            return Err("print expects no argument".into());
        }
        for arg in args.clone() {
            let value = self.execute_node(&arg)?;
            let mut format = format!("{:?}", value);
            if let SystemValue::String(v) = value.clone() {
                if v == "\n" {
                    format.push_str("\n");
                    continue;
                }
                if v == "\t" {
                    format.push_str("\t");
                    continue;
                }
                if v == "\r" {
                    format.push_str("\r");
                    continue;
                }
            }
            print!("{}", format);
        }
        return Ok(SystemValue::Null);
    }
    fn __system_fn_open_file(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        let mut evaluated_args = Vec::new();
        for arg in args.clone() {
            let evaluated_arg = self.execute_node(&arg)?;
            //debug!("args: {:?}", evaluated_arg);
            evaluated_args.push(evaluated_arg);
        }
        let mut options = OpenOptions::new();

        let _file = options.read(true);
        let mut file_name = String::new();
        for value in &evaluated_args {
            match value {
                SystemValue::Bool(ref v) => {
                    _file.write(v.clone());
                }
                SystemValue::String(ref v) => {
                    file_name = v.clone();
                }
                SystemValue::System(SystemType::PathBuf(ref _path)) => {
                    let mut path = _path.clone();
                    file_name = path.to_str().unwrap().to_string();
                }
                _ => {
                    todo!();
                }
            }
        }
        let file = match _file.open(file_name.clone()) {
            Ok(v) => v,
            Err(e) => {
                return Err(e.to_string());
            }
        };
        return Ok(SystemValue::System(SystemType::File(Arc::new(Mutex::new(
            file,
        )))));
    }

    fn __system_fn_read_file_(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        let mut evaluated_args = Vec::new();
        for arg in args.clone() {
            let evaluated_arg = self.execute_node(&arg)?;
            //debug!("args: {:?}", evaluated_arg);
            evaluated_args.push(evaluated_arg);
        }
        for value in &evaluated_args {
            match value {
                SystemValue::System(SystemType::File(ref _file)) => {
                    let mut file = Arc::clone(_file);
                    let mut contents = String::new();
                    file.lock().unwrap().read_to_string(&mut contents).unwrap();

                    return Ok(SystemValue::String(contents.clone()));
                }
                _ => {
                    todo!();
                }
            }
        }
        return Ok(SystemValue::Null);
    }

    fn __system_fn_to_path(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        let mut string = String::new();
        for arg in args.clone() {
            match arg.value {
                NodeValue::DataType(DataType::String(ref v)) => {
                    let path = PathBuf::from(v.clone());
                    return Ok(SystemValue::System(SystemType::PathBuf(path.clone())));
                }
                _ => {
                    todo!();
                }
            }
        }
        return Ok(SystemValue::Null);
    }
    fn __system_fn_create_file(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        let mut evaluated_args = Vec::new();
        for arg in args.clone() {
            let evaluated_arg = self.execute_node(&arg)?;
            //debug!("args: {:?}", evaluated_arg);
            evaluated_args.push(evaluated_arg);
        }
        let mut file_name = String::new();
        for value in &evaluated_args {
            match value {
                SystemValue::String(ref v) => {
                    file_name = v.clone();
                }
                _ => {
                    todo!();
                }
            }
        }
        let file = File::create(file_name.clone()).unwrap();
        return Ok(SystemValue::System(SystemType::File(Arc::new(Mutex::new(
            file,
        )))));
    }

    fn __system_fn_sin(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        let mut evaluated_args = Vec::new();
        for arg in args.clone() {
            let evaluated_arg = self.execute_node(&arg)?;
            //debug!("args: {:?}", evaluated_arg);
            evaluated_args.push(evaluated_arg);
        }

        for value in &evaluated_args {
            match value {
                SystemValue::F32(ref v) => {
                    let value = v.clone();
                    return Ok(value.sin().into());
                }
                SystemValue::F64(ref v) => {
                    let value = v.clone();
                    return Ok(value.sin().into());
                }
                _ => {
                    todo!();
                }
            }
        }
        return Ok(SystemValue::Null);
    }
    fn __system_fn_cos(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        let mut evaluated_args = Vec::new();
        for arg in args.clone() {
            let evaluated_arg = self.execute_node(&arg)?;
            //debug!("args: {:?}", evaluated_arg);
            evaluated_args.push(evaluated_arg);
        }

        for value in &evaluated_args {
            match value {
                SystemValue::F32(ref v) => {
                    let value = v.clone();
                    return Ok(value.cos().into());
                }
                SystemValue::F64(ref v) => {
                    let value = v.clone();
                    return Ok(value.cos().into());
                }
                _ => {
                    todo!();
                }
            }
        }
        return Ok(SystemValue::Null);
    }
    fn __system_fn_tan(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        let mut evaluated_args = Vec::new();
        for arg in args.clone() {
            let evaluated_arg = self.execute_node(&arg)?;
            //debug!("args: {:?}", evaluated_arg);
            evaluated_args.push(evaluated_arg);
        }

        for value in &evaluated_args {
            match value {
                SystemValue::F32(ref v) => {
                    let value = v.clone();
                    return Ok(value.tan().into());
                }
                SystemValue::F64(ref v) => {
                    let value = v.clone();
                    return Ok(value.tan().into());
                }
                _ => {
                    todo!();
                }
            }
        }
        return Ok(SystemValue::Null);
    }
    fn __system_fn_dxlib_init(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        unsafe {
            //let lib = libloading::Library::new("DxLib_x64.dll").unwrap();

            let _lib = match self.execute_node(&args[0])? {
                SystemValue::System(SystemType::Library(ref v)) => v.clone(),
                _ => todo!(),
            };
            let mut lib = _lib.lock().unwrap();
            // シンボルを取得する
            let func: Symbol<unsafe extern "C" fn() -> i32> = lib.get(b"dx_DxLib_Init").unwrap();
            // 関数を呼び出す
            let res = func();
            return Ok(res.into());
        }
    }
    fn __system_fn_dxlib_end(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        unsafe {
            // let lib = libloading::Library::new("DxLib_x64.dll").unwrap();
            let _lib = match self.execute_node(&args[0])? {
                SystemValue::System(SystemType::Library(ref v)) => v.clone(),
                _ => todo!(),
            };
            let mut lib = _lib.lock().unwrap();

            // シンボルを取得する
            let func: Symbol<unsafe extern "C" fn() -> i32> = lib.get(b"dx_DxLib_End").unwrap();
            // 関数を呼び出す
            let res = func();
            return Ok(res.into());
        }
    }

    fn __system_fn_dxlib_process_message(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        unsafe {
            // let lib = libloading::Library::new("DxLib_x64.dll").unwrap();
            let _lib = match self.execute_node(&args[0])? {
                SystemValue::System(SystemType::Library(ref v)) => v.clone(),
                _ => todo!(),
            };
            let mut lib = _lib.lock().unwrap();

            // シンボルを取得する
            let func: Symbol<unsafe extern "C" fn() -> i32> =
                lib.get(b"dx_ProcessMessage").unwrap();
            // 関数を呼び出す
            let res = func();
            return Ok(res.into());
        }
    }
    fn __system_fn_dxlib_change_window_mode(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        unsafe {
            //           let lib = libloading::Library::new("DxLib_x64.dll").unwrap();
            let _lib = match self.execute_node(&args[0])? {
                SystemValue::System(SystemType::Library(ref v)) => v.clone(),
                _ => todo!(),
            };
            let mut lib = _lib.lock().unwrap();

            // シンボルを取得する
            let func: Symbol<unsafe extern "C" fn(i32) -> i32> =
                lib.get(b"dx_ChangeWindowMode").unwrap();
            // 関数を呼び出す
            let res = func(1);
            return Ok(res.into());
        }
    }

    fn __system_fn_load_library(&mut self, args: &Vec<Node>, node: &Node) -> Value {
        let mut evaluated_args = Vec::new();
        for arg in args.clone() {
            let evaluated_arg = self.execute_node(&arg)?;
            //debug!("args: {:?}", evaluated_arg);
            evaluated_args.push(evaluated_arg);
        }
        let mut dll_path = String::new();
        for value in &evaluated_args {
            match value {
                SystemValue::String(ref v) => {
                    dll_path = v.clone();
                }
                _ => todo!(),
            }
        }
        unsafe {
            let lib = libloading::Library::new(&dll_path).unwrap();
            return Ok(SystemValue::System(SystemType::Library(Arc::new(
                Mutex::new(lib),
            ))));
        }
    }
    fn register_system_all_functions(&mut self) {
        self.system_functions.insert(
            "func_lists".to_string(),
            Decoder::__system_fn_system_function_lists,
        );
        {
            self.system_functions.insert(
                "load_library".to_string(),
                Decoder::__system_fn_load_library,
            );

            self.system_functions
                .insert("dx_init".to_string(), Decoder::__system_fn_dxlib_init);
            self.system_functions.insert(
                "dx_process_message".to_string(),
                Decoder::__system_fn_dxlib_process_message,
            );

            self.system_functions
                .insert("dx_end".to_string(), Decoder::__system_fn_dxlib_end);
            self.system_functions.insert(
                "dx_change_window".to_string(),
                Decoder::__system_fn_dxlib_change_window_mode,
            );
        }
        self.system_functions
            .insert("to_cstring".to_string(), Decoder::__system_fn_to_cstring);
        self.system_functions
            .insert("printf".to_string(), Decoder::__system_fn_printf);

        self.system_functions
            .insert("sin".to_string(), Decoder::__system_fn_sin);
        self.system_functions
            .insert("cos".to_string(), Decoder::__system_fn_cos);

        self.system_functions
            .insert("tan".to_string(), Decoder::__system_fn_tan);

        self.system_functions
            .insert("as_ptr".to_string(), Decoder::__system_fn_as_ptr);
        self.system_functions
            .insert("to_path".to_string(), Decoder::__system_fn_to_path);

        self.system_functions
            .insert("open".to_string(), Decoder::__system_fn_open_file);
        self.system_functions
            .insert("create".to_string(), Decoder::__system_fn_create_file);

        self.system_functions
            .insert("print".to_string(), Decoder::__system_fn_print);
        self.system_functions
            .insert("syntax".to_string(), Decoder::__system_fn_syntax);
        self.system_functions
            .insert("line".to_string(), Decoder::__system_fn_line);
        self.system_functions
            .insert("file".to_string(), Decoder::__system_fn_file);
        self.system_functions
            .insert("column".to_string(), Decoder::__system_fn_column);
        self.system_functions
            .insert("list_files".to_string(), Decoder::__system_fn_list_files);
        self.system_functions
            .insert("play_music".to_string(), Decoder::__system_fn_play_music);
        self.system_functions
            .insert("str".to_string(), Decoder::__system_fn_str);
        self.system_functions.insert(
            "show_msg_box".to_string(),
            Decoder::__system_fn_show_msg_box,
        );
        self.system_functions
            .insert("open_recent".to_string(), Decoder::__system_fn_open_recent);
        self.system_functions
            .insert("sleep".to_string(), Decoder::__system_fn_sleep);
        self.system_functions
            .insert("read_file".to_string(), Decoder::__system_fn_read_file_);
        self.system_functions
            .insert("write_file".to_string(), Decoder::__system_fn_write_file);
        self.system_functions
            .insert("exit".to_string(), Decoder::__system_fn_exit);
        self.system_functions
            .insert("args".to_string(), Decoder::__system_fn_args);
        self.system_functions
            .insert("cmd".to_string(), Decoder::__system_fn_cmd);
    }

    fn eval_call(
        &mut self,
        node: &Node,
        name: &String,
        args: &Vec<Node>,
        is_system: &bool,
    ) -> Value {
        let mut result = SystemValue::Null;
        let mut evaluated_args = Vec::new();
        for arg in args {
            let evaluated_arg = self.execute_node(&arg)?;
            evaluated_args.push(evaluated_arg);
        }

        #[cfg(feature = "wip-system")]
        {
            if *is_system {
                if let Some(func) = self.system_functions.get(&name.clone()) {
                    return func(self, &args, &node);
                } else {
                    return Err(compile_error!(
                        "error",
                        self.current_node.clone().unwrap().1.line,
                        self.current_node.clone().unwrap().1.column,
                        &self.current_node.clone().unwrap().0,
                        &self
                            .file_contents
                            .get(&self.current_node.clone().unwrap().0)
                            .unwrap(),
                        "System function '{}' is not registered",
                        name
                    ));
                }
            }
        }

        // 関数情報をメモリマネージャから取得
        let func_address = self
            .context
            .global_context
            .get(name)
            .ok_or_else(|| {
                compile_error!(
                    "error",
                    self.current_node.clone().unwrap().1.line,
                    self.current_node.clone().unwrap().1.column,
                    &self.current_node.clone().unwrap().0,
                    &self
                        .file_contents
                        .get(&self.current_node.clone().unwrap().0)
                        .unwrap(),
                    "Function '{}' is not defined",
                    name
                )
            })?
            .address;

        let func_info = self
            .memory_mgr
            .get_value::<SystemValue>(func_address)
            .unwrap()
            .clone();

        // 関数情報から引数、body、戻り値の型を抽出
        if let SystemValue::Struct(ref func_info_struct) = func_info {
            let arg_addresses = &func_info_struct[0]; // 引数 (SystemValueで格納)
            let _body = &func_info_struct[1]; // 関数のbody (SystemValue::NodeBlockで格納)
            let _return_type = &func_info_struct[2]; // 戻り値の型 (SystemValue::NodeBlockで格納)
                                                     // body が SystemValue::NodeBlockであることを確認
            let body_array = if let SystemValue::__NodeBlock(ref body_elements) = *_body {
                body_elements
            } else {
                return Err("Invalid body format: Expected SystemValue::NodeBlock".to_string());
            };

            // 新しいスタックフレームを作成
            self.memory_mgr.push_stack_frame(name);
            let mut arg_name = String::new();
            // 関数の引数を処理し、スタックフレームに格納
            if let SystemValue::Array(ref func_args) = *arg_addresses {
                for (i, value) in evaluated_args.iter().enumerate() {
                    // 対応する func_args の要素を取得
                    let arg = &func_args[0];

                    // 入れ子のArrayを取り出す
                    if let SystemValue::Array(ref nested_array) = *arg {
                        if let SystemValue::Array(ref arg_tuple) = nested_array[i] {
                            arg_name = if let SystemValue::String(ref v) = &arg_tuple[1] {
                                v.clone()
                            } else {
                                return Err("Invalid argument format".to_string());
                            };
                            let arg_data_name = if let SystemValue::__NodeBlock(nodes) =
                                &arg_tuple[0]
                            {
                                if let Some(node) = nodes.iter().find(|&node| {
                                    matches!(node.value, NodeValue::DataType(DataType::String(_)))
                                }) {
                                    if let NodeValue::DataType(DataType::String(ref v)) = node.value
                                    {
                                        v.clone()
                                    } else {
                                        return Err("Invalid argument format".to_string());
                                    }
                                } else {
                                    return Err("Invalid argument format".to_string());
                                }
                            } else {
                                return Err("Invalid argument format".to_string());
                            };

                            let index = self.memory_mgr.allocate(value.clone());
                            let block = MemoryBlock {
                                id: index,
                                value: Box::new(value.clone()),
                            };
                            self.memory_mgr.add_to_stack_frame(name, block);

                            self.context.local_context.insert(
                                arg_name.clone(),
                                Variable {
                                    value: value.clone(),
                                    data_name: arg_data_name.clone(),
                                    address: index,
                                    is_mutable: false,
                                    size: 0,
                                },
                            );

                            debug!("{:?}", self.context.local_context);
                        }
                    }
                }
            }

            // 関数のbodyを評価する (SystemValue::NodeBlock内の各ノードを評価)
            // 関数のブロックを評価
            //panic!("{:?}", body_array.clone());
            result = self.eval_block(&body_array)?;
            //panic!("{:?}",self.context.local_context);
            // 実行後、スタックフレームをポップ
            self.memory_mgr.pop_stack_frame(name);

            debug!(
                "CallFunction: name = {:?}, args = {:?}, return_value = {:?}",
                name,
                evaluated_args.clone(),
                result
            );
        } else {
            return Err("Invalid function format".to_string());
        }

        Ok(result)
    }
    fn eval_const_declaration(
        &mut self,
        node: &Node,
        var_name: &Box<Node>,
        data_type: &Box<Node>,
        value: &Box<Node>,
        is_local: &bool,
        is_public: &bool,
    ) -> Value {
        // ステートメントフラグのチェック
        if !node.is_statement() {
            return Err(compile_error!(
                "error",
                self.current_node.clone().unwrap().1.line(),
                self.current_node.clone().unwrap().1.column(),
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "Variable declaration must be a statement"
            ));
        }

        //debug!("is_reference: {:?}", is_reference);
        let name = match var_name.value() {
            NodeValue::Variable(_, v, _, _, _) => v,
            _ => String::new(),
        };

        // Variable(Box<Node>, String,bool, bool),// 変数(型名,変数名,可変性フラグ,参照型フラグ)
        let value_is_reference = match &value.clone().value() {
            NodeValue::Variable(_, _, _, v, _) => v.clone(),
            _ => false,
        };

        self.check_reserved_words(&name, RESERVED_WORDS)?;

        let mut v_type: SystemValue = match data_type.value() {
            NodeValue::DataType(DataType::String(v)) => v.clone(),
            _ => String::new(),
        }
        .into();
        let (mut generic_v_name, mut generic_v_lists) = match data_type.value() {
            NodeValue::DataType(DataType::Generic(v, v_lists)) => (v.clone(), v_lists.clone()),
            _ => (String::new(), vec![]),
        };
        // panic!("name: {:?} lists: {:?}",generic_v_name,generic_v_lists);

        //panic!("name: {:?} lists: {:?}",generic_v_name,generic_v_lists);

        let mut v_type_string = String::new();
        let mut v_value: SystemValue = SystemValue::Null;
        if let SystemValue::String(ref v) = v_type.clone() {
            if v.is_empty() {
            } else {
            }
            v_type_string = v.clone();
            v_value = self.execute_node(&value)?;
        }
        let address;

        {
            // 一時的にcontextの借用を解除
            let context = if *is_local {
                &mut self.context.local_context
            } else {
                &mut self.context.global_context
            };

            if context.contains_key(&name) {
                return Err(compile_error!(
                    "error",
                    self.current_node.clone().unwrap().1.line(),
                    self.current_node.clone().unwrap().1.column(),
                    &self.current_node.clone().unwrap().0,
                    &self
                        .file_contents
                        .get(&self.current_node.clone().unwrap().0)
                        .unwrap(),
                    "Variable '{}' is already defined",
                    name
                ));
            }
        }

        if value_is_reference {
            // 参照型の場合、右辺の変数名を取り出してアドレスを取得して直接変更
            address = {
                let context = if *is_local {
                    &mut self.context.local_context
                } else {
                    &mut self.context.global_context
                };

                match value.value() {
                    NodeValue::Variable(_, v, _, _, _) => {
                        if let Some(variable) = context.get(&v) {
                            variable.address
                        } else {
                            return Err(compile_error!(
                                "error",
                                self.current_node.clone().unwrap().1.line(),
                                self.current_node.clone().unwrap().1.column(),
                                &self.current_node.clone().unwrap().0,
                                &self
                                    .file_contents
                                    .get(&self.current_node.clone().unwrap().0)
                                    .unwrap(),
                                "Variable '{}' not found in context",
                                v
                            ));
                        }
                    }
                    _ => {
                        let _address = self.memory_mgr.allocate(v_value.clone());
                        _address
                    }
                }
            };

            let context = if *is_local {
                &mut self.context.local_context
            } else {
                &mut self.context.global_context
            };

            context.insert(
                name.clone(),
                Variable {
                    value: v_value.clone(),
                    data_name: v_type_string.clone(),
                    address,
                    is_mutable: false,
                    size: v_value.size(),
                },
            );
        } else {
            address = self.memory_mgr.allocate(v_value.clone());
            let context = if *is_local {
                &mut self.context.local_context
            } else {
                &mut self.context.global_context
            };

            context.insert(
                name.clone(),
                Variable {
                    value: v_value.clone(),
                    data_name: v_type_string.clone(),
                    address,
                    is_mutable: false,
                    size: v_value.size(),
                },
            );
        }

        debug!("Const Declaration: is_public={:?}, name = {:?}, data_type = {:?}, value = {:?}, address = {:?} size = {:?} is_local: {} value_is_reference: {:?}",is_public, name, v_type, v_value, address,v_value.size(),is_local,value_is_reference);
        let line = self.current_node.clone().unwrap().1.line();
        let column = self.current_node.clone().unwrap().1.column();
        self.context
            .used_context
            .insert(name.clone(), (line, column, false));
        Ok(v_value)
    }

    fn eval_variable_declaration(
        &mut self,
        node: &Node,
        var_name: &Box<Node>,
        data_type: &Box<Node>,
        value: &Box<Node>,
        is_local: &bool,
        is_mutable: &bool,
        is_public: &bool,
    ) -> Value {
        // ステートメントフラグのチェック
        if !node.is_statement() {
            return Err(compile_error!(
                "error",
                self.current_node.clone().unwrap().1.line(),
                self.current_node.clone().unwrap().1.column(),
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "Variable declaration must be a statement"
            ));
        }

        //debug!("is_reference: {:?}", is_reference);
        let name = match var_name.value() {
            NodeValue::Variable(_, v, _, _, _) => v,
            _ => String::new(),
        };

        // Variable(Box<Node>, String,bool, bool),// 変数(型名,変数名,可変性フラグ,参照型フラグ)
        let value_is_mutable = match &value.clone().value() {
            NodeValue::Variable(_, _, v, _, _) => v.clone(),
            _ => false,
        };
        let value_is_reference = match &value.clone().value() {
            NodeValue::Variable(_, _, _, v, _) => v.clone(),
            _ => false,
        };

        self.check_reserved_words(&name, RESERVED_WORDS)?;

        let mut v_type: SystemValue = match data_type.value() {
            NodeValue::DataType(DataType::String(v)) => v.clone(),
            _ => String::new(),
        }
        .into();
        let (mut generic_v_name, mut generic_v_lists) = match data_type.value() {
            NodeValue::DataType(DataType::Generic(v, v_lists)) => (v.clone(), v_lists.clone()),
            _ => (String::new(), vec![]),
        };
        // panic!("name: {:?} lists: {:?}",generic_v_name,generic_v_lists);

        //panic!("name: {:?} lists: {:?}",generic_v_name,generic_v_lists);

        let mut v_type_string = String::new();
        let mut v_value: SystemValue = SystemValue::Null;
        if let SystemValue::String(ref v) = v_type.clone() {
            if v.is_empty() {
            } else {
            }
            v_type_string = v.clone();
            v_value = self.execute_node(&value)?;
        }
        let address;

        {
            // 一時的にcontextの借用を解除
            let context = if *is_local {
                &mut self.context.local_context
            } else {
                &mut self.context.global_context
            };

            if context.contains_key(&name) {
                return Err(compile_error!(
                    "error",
                    self.current_node.clone().unwrap().1.line(),
                    self.current_node.clone().unwrap().1.column(),
                    &self.current_node.clone().unwrap().0,
                    &self
                        .file_contents
                        .get(&self.current_node.clone().unwrap().0)
                        .unwrap(),
                    "Variable '{}' is already defined",
                    name
                ));
            }
        }

        if value_is_reference {
            // 参照型の場合、右辺の変数名を取り出してアドレスを取得して直接変更
            address = {
                let context = if *is_local {
                    &mut self.context.local_context
                } else {
                    &mut self.context.global_context
                };

                match value.value() {
                    NodeValue::Variable(_, v, _, _, _) => {
                        if let Some(variable) = context.get(&v) {
                            variable.address
                        } else {
                            return Err(compile_error!(
                                "error",
                                self.current_node.clone().unwrap().1.line(),
                                self.current_node.clone().unwrap().1.column(),
                                &self.current_node.clone().unwrap().0,
                                &self
                                    .file_contents
                                    .get(&self.current_node.clone().unwrap().0)
                                    .unwrap(),
                                "Variable '{}' not found in context",
                                v
                            ));
                        }
                    }
                    _ => {
                        let _address = self.memory_mgr.allocate(v_value.clone());
                        _address
                    }
                }
            };

            let context = if *is_local {
                &mut self.context.local_context
            } else {
                &mut self.context.global_context
            };

            context.insert(
                name.clone(),
                Variable {
                    value: v_value.clone(),
                    data_name: v_type_string.clone(),
                    address,
                    is_mutable: *is_mutable,
                    size: v_value.size(),
                },
            );
        } else {
            address = self.memory_mgr.allocate(v_value.clone());
            let context = if *is_local {
                &mut self.context.local_context
            } else {
                &mut self.context.global_context
            };

            context.insert(
                name.clone(),
                Variable {
                    value: v_value.clone(),
                    data_name: v_type_string.clone(),
                    address,
                    is_mutable: *is_mutable,
                    size: v_value.size(),
                },
            );
        }

        debug!("VariableDeclaration: is_public = {:?}, name = {:?}, data_type = {:?}, value = {:?}, address = {:?} size = {:?} is_mutable: {} is_local: {} value_is_mutable: {:?} value_is_reference: {:?}", is_public,name, v_type, v_value, address,v_value.size(),is_mutable,is_local,value_is_mutable,value_is_reference);
        let line = self.current_node.clone().unwrap().1.line();
        let column = self.current_node.clone().unwrap().1.column();
        self.context
            .used_context
            .insert(name.clone(), (line, column, false));
        Ok(v_value)
    }

    fn eval_type_declaration(
        &mut self,
        _type_name: &Box<Node>,
        _type: &Box<Node>,
        is_public: bool,
    ) -> Value {
        let name = match _type_name.value() {
            NodeValue::Variable(_, v, _, _, _) => v,
            _ => String::new(),
        };
        if self.context.type_context.contains_key(&name) {
            return Err(compile_error!(
                "error",
                self.current_node.clone().unwrap().1.line(),
                self.current_node.clone().unwrap().1.column(),
                &self.current_node.clone().unwrap().0,
                /*&self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                */
                "",
                "type '{}' is already defined",
                name
            ));
        }
        let v_type = match _type.value() {
            NodeValue::DataType(DataType::String(v)) => v,
            _ => String::new(),
        };

        // 型定義をtype_contextに保存
        self.context
            .type_context
            .insert(name.clone(), v_type.clone());

        debug!(
            "TypeDeclaration: is_public = {:?}, type_name = {:?}, type = {:?}",
            is_public, name, v_type
        );
        Ok(SystemValue::String(name.into()))
    }
    fn eval_variable(&mut self, name: &String) -> Value {
        let line = self.current_node.clone().unwrap().1.line();
        let column = self.current_node.clone().unwrap().1.column();
        self.context
            .used_context
            .insert(name.clone(), (line, column, true));

        if let Some(var) = self.context.local_context.get(name) {
            // ローカルスコープで変数を見つけた場合
            let index = var.address; // アドレスを取得
            let value_size = var.value.size();

            let value = &var.value;
            debug!(
                "Found variable: Name = {} Address = {}, Value size = {}, Heap size = {} value: {:?}",
                name,
                index,
                value_size,
                self.memory_mgr.heap.len(),
                value
            );

            let value = self
                .memory_mgr
                .get_value::<SystemValue>(index)
                .expect("Failed to retrieve value");

            Ok(value.clone())
        } else if let Some(var) = self.context.global_context.get(name) {
            // グローバルスコープで変数を見つけた場合
            let index = var.address; // アドレスを取得
            let value_size = var.value.size();
            let value = &var.value;
            debug!(
                "Found variable: Name = {} Address = {}, Value size = {}, Heap size = {} value: {:?}",
                name,
                index,
                value_size,
                self.memory_mgr.heap.len(),
                value
            );

            let value = self
                .memory_mgr
                .get_value::<SystemValue>(index)
                .expect("Failed to retrieve value");
            Ok(value.clone())
        } else {
            Err(compile_error!(
                "error",
                self.current_node.clone().unwrap().1.line(),
                self.current_node.clone().unwrap().1.column(),
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "Cannot find value {:?} in this scope",
                name.clone(),
            ))
        }
    }
    fn eval_return(&mut self, ret: &Box<Node>) -> Value {
        let ret = self.execute_node(&ret)?;
        debug!("Return: {:?}", ret);
        Ok(ret)
    }
    fn eval_binary_increment(&mut self, lhs: &Box<Node>) -> Value {
        let left_value = match self.execute_node(&lhs)? {
            SystemValue::I32(v) => v,
            _ => -1,
        };
        let var = match lhs.value() {
            NodeValue::Variable(_, v, _, _, _) => v,
            _ => String::new(),
        };

        let variable_data = self
            .context
            .local_context
            .get(&var)
            .cloned()
            .or_else(|| self.context.global_context.get(&var).cloned());
        if let Some(variable) = variable_data {
            let result = left_value + 1;
            self.memory_mgr
                .update_value(variable.address.clone(), result);
            Ok(SystemValue::I32(result.into()))
        } else {
            Ok(SystemValue::Null)
        }
    }
    fn eval_binary_decrement(&mut self, lhs: &Box<Node>) -> Value {
        let left_value = match self.execute_node(&lhs)? {
            SystemValue::I32(v) => v,
            _ => -1,
        };
        let var = match lhs.value() {
            NodeValue::Variable(_, v, _, _, _) => v,
            _ => String::new(),
        };

        let variable_data = self
            .context
            .local_context
            .get(&var)
            .cloned()
            .or_else(|| self.context.global_context.get(&var).cloned());
        if let Some(variable) = variable_data {
            let result = left_value - 1;
            self.memory_mgr
                .update_value(variable.address.clone(), result);
            Ok(SystemValue::I32(result.into()))
        } else {
            Ok(SystemValue::Null)
        }
    }

    fn eval_binary_bit(&mut self, node: &Node) -> Result<SystemValue, String> {
        if let NodeValue::Operator(Operator::BitAnd(left, right))
        | NodeValue::Operator(Operator::BitOr(left, right))
        | NodeValue::Operator(Operator::BitXor(left, right))
        | NodeValue::Operator(Operator::ShiftLeft(left, right))
        | NodeValue::Operator(Operator::ShiftRight(left, right)) = &node.value
        {
            let left_value = self.execute_node(left)?;
            let right_value = self.execute_node(right)?;

            match (&node.value, left_value.clone(), right_value.clone()) {
                (
                    NodeValue::Operator(Operator::BitAnd(_, _)),
                    SystemValue::I64(_),
                    SystemValue::I64(_),
                ) => Ok((left_value & right_value)?),
                (
                    NodeValue::Operator(Operator::BitOr(_, _)),
                    SystemValue::I64(_),
                    SystemValue::I64(_),
                ) => Ok((left_value | right_value)?),
                (
                    NodeValue::Operator(Operator::BitXor(_, _)),
                    SystemValue::I64(_),
                    SystemValue::I64(_),
                ) => Ok((left_value ^ right_value)?),
                (
                    NodeValue::Operator(Operator::ShiftLeft(_, _)),
                    SystemValue::I64(_),
                    SystemValue::I64(_),
                ) => Ok((left_value << right_value)?),
                (
                    NodeValue::Operator(Operator::ShiftRight(_, _)),
                    SystemValue::I64(_),
                    SystemValue::I64(_),
                ) => Ok((left_value >> right_value)?),
                _ => {
                    return Err(compile_error!(
                    "error",
                    node.line,
                    node.column,
                    &self.current_node.clone().unwrap().0,
                    &self
                        .file_contents
                        .get(&self.current_node.clone().unwrap().0)
                        .unwrap(),
                    "Unsupported operation or mismatched types in binary bit operation: {:?} {:?}",
                    left_value,
                    right_value
                ));
                }
            }
        } else {
            Err(compile_error!(
                "error",
                node.line,
                node.column,
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "Unsupported node value: {:?}",
                node.value.clone(),
            ))
        }
    }

    fn eval_binary_condition(&mut self, node: &Node) -> Result<SystemValue, String> {
        if let NodeValue::Operator(Operator::Eq(left, right))
        | NodeValue::Operator(Operator::Ne(left, right))
        | NodeValue::Operator(Operator::Lt(left, right))
        | NodeValue::Operator(Operator::Gt(left, right))
        | NodeValue::Operator(Operator::Le(left, right))
        | NodeValue::Operator(Operator::Ge(left, right)) = &node.value
        {
            let left_value = self.execute_node(left)?;
            let right_value = self.execute_node(right)?;

            match (&node.value, left_value.clone(), right_value.clone()) {
                (
                    NodeValue::Operator(Operator::Eq(_, _)),
                    SystemValue::F64(l),
                    SystemValue::F64(r),
                ) => Ok(SystemValue::Bool(l == r)),
                (
                    NodeValue::Operator(Operator::Eq(_, _)),
                    SystemValue::String(l),
                    SystemValue::String(r),
                ) => Ok(SystemValue::Bool(l == r)),
                (
                    NodeValue::Operator(Operator::Ne(_, _)),
                    SystemValue::F64(l),
                    SystemValue::F64(r),
                ) => Ok(SystemValue::Bool(l != r)),
                (
                    NodeValue::Operator(Operator::Ne(_, _)),
                    SystemValue::String(l),
                    SystemValue::String(r),
                ) => Ok(SystemValue::Bool(l != r)),
                (
                    NodeValue::Operator(Operator::Lt(_, _)),
                    SystemValue::F64(l),
                    SystemValue::F64(r),
                ) => Ok(SystemValue::Bool(l < r)),
                (
                    NodeValue::Operator(Operator::Gt(_, _)),
                    SystemValue::F64(l),
                    SystemValue::F64(r),
                ) => Ok(SystemValue::Bool(l > r)),
                (
                    NodeValue::Operator(Operator::Le(_, _)),
                    SystemValue::F64(l),
                    SystemValue::F64(r),
                ) => Ok(SystemValue::Bool(l <= r)),
                (
                    NodeValue::Operator(Operator::Ge(_, _)),
                    SystemValue::I64(l),
                    SystemValue::I64(r),
                ) => Ok(SystemValue::Bool(l >= r)),
                _ => {
                    return Err(compile_error!(
                        "error",
                        node.line,
                        node.column,
                        &self.current_node.clone().unwrap().0,
                        &self
                            .file_contents
                            .get(&self.current_node.clone().unwrap().0)
                            .unwrap(),
                        "Unsupported operation or mismatched types in condition: {:?} {:?}",
                        left_value,
                        right_value
                    ));
                }
            }
        } else {
            Err(compile_error!(
                "error",
                node.line,
                node.column,
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "Unsupported node value: {:?}",
                node.value.clone(),
            ))
        }
    }
    fn eval_binary_op(&mut self, node: &Node) -> Result<SystemValue, String> {
        if let NodeValue::Operator(Operator::Add(lhs, rhs))
        | NodeValue::Operator(Operator::Sub(lhs, rhs))
        | NodeValue::Operator(Operator::Mul(lhs, rhs))
        | NodeValue::Operator(Operator::Div(lhs, rhs))
        | NodeValue::Operator(Operator::Modulus(lhs, rhs))
        | NodeValue::Operator(Operator::AddAssign(lhs, rhs))
        | NodeValue::Operator(Operator::SubAssign(lhs, rhs))
        | NodeValue::Operator(Operator::MulAssign(lhs, rhs))
        | NodeValue::Operator(Operator::DivAssign(lhs, rhs))
        | NodeValue::Operator(Operator::ModulusAssign(lhs, rhs)) = &node.value
        {
            let left_value = self.execute_node(lhs)?;
            let right_value = self.execute_node(rhs)?;

            match (&node.value, left_value.clone(), right_value.clone()) {
                (NodeValue::Operator(Operator::Add(_, _)), left, right) => match (left, right) {
                    (SystemValue::I32(l), SystemValue::I32(r)) => Ok(SystemValue::I32(l + r)),

                    (SystemValue::I64(l), SystemValue::I64(r)) => Ok(SystemValue::I64(l + r)),
                    (SystemValue::F64(l), SystemValue::F64(r)) => Ok(SystemValue::F64(l + r)),
                    (SystemValue::String(l), SystemValue::String(r)) => {
                        Ok(SystemValue::String(l + &r))
                    }
                    _ => {
                        return Err(compile_error!(
                            "error",
                            node.line,
                            node.column,
                            &self.current_node.clone().unwrap().0,
                            &self
                                .file_contents
                                .get(&self.current_node.clone().unwrap().0)
                                .unwrap(),
                            "Unsupported types for addition: {:?} {:?}",
                            left_value,
                            right_value
                        ));
                    }
                },
                (NodeValue::Operator(Operator::Sub(_, _)), left, right) => match (left, right) {
                    (SystemValue::I32(l), SystemValue::I32(r)) => Ok(SystemValue::I32(l - r)),

                    (SystemValue::I64(l), SystemValue::I64(r)) => Ok(SystemValue::I64(l - r)),
                    (SystemValue::F64(l), SystemValue::F64(r)) => Ok(SystemValue::F64(l - r)),
                    _ => {
                        return Err(compile_error!(
                            "error",
                            node.line,
                            node.column,
                            &self.current_node.clone().unwrap().0,
                            &self
                                .file_contents
                                .get(&self.current_node.clone().unwrap().0)
                                .unwrap(),
                            "Unsupported types for subtraction: {:?} {:?}",
                            left_value,
                            right_value
                        ));
                    }
                },
                (NodeValue::Operator(Operator::Mul(_, _)), left, right) => match (left, right) {
                    (SystemValue::I32(l), SystemValue::I32(r)) => Ok(SystemValue::I32(l * r)),

                    (SystemValue::I64(l), SystemValue::I64(r)) => Ok(SystemValue::I64(l * r)),
                    (SystemValue::F64(l), SystemValue::F64(r)) => Ok(SystemValue::F64(l * r)),
                    _ => {
                        return Err(compile_error!(
                            "error",
                            node.line,
                            node.column,
                            &self.current_node.clone().unwrap().0,
                            &self
                                .file_contents
                                .get(&self.current_node.clone().unwrap().0)
                                .unwrap(),
                            "Unsupported types for multiplication: {:?} {:?}",
                            left_value,
                            right_value
                        ));
                    }
                },
                (NodeValue::Operator(Operator::Div(_, _)), left, right) => match (left, right) {
                    (SystemValue::I32(l), SystemValue::I32(r)) => {
                        if r == 0 {
                            return Err(compile_error!(
                                "error",
                                node.line,
                                node.column,
                                &self.current_node.clone().unwrap().0,
                                &self
                                    .file_contents
                                    .get(&self.current_node.clone().unwrap().0)
                                    .unwrap(),
                                "Division by zero: {:?} {:?}",
                                left_value,
                                right_value
                            ));
                        }
                        Ok(SystemValue::I32(l / r))
                    }

                    (SystemValue::I64(l), SystemValue::I64(r)) => {
                        if r == 0 {
                            return Err(compile_error!(
                                "error",
                                node.line,
                                node.column,
                                &self.current_node.clone().unwrap().0,
                                &self
                                    .file_contents
                                    .get(&self.current_node.clone().unwrap().0)
                                    .unwrap(),
                                "Division by zero: {:?} {:?}",
                                left_value,
                                right_value
                            ));
                        }
                        Ok(SystemValue::I64(l / r))
                    }

                    (SystemValue::F64(l), SystemValue::F64(r)) => {
                        if r == 0.0 {
                            return Err(compile_error!(
                                "error",
                                node.line,
                                node.column,
                                &self.current_node.clone().unwrap().0,
                                &self
                                    .file_contents
                                    .get(&self.current_node.clone().unwrap().0)
                                    .unwrap(),
                                "Division by zero : {:?} {:?}",
                                left_value,
                                right_value
                            ));
                        }
                        Ok(SystemValue::F64(l / r))
                    }

                    _ => {
                        return Err(compile_error!(
                            "error",
                            node.line,
                            node.column,
                            &self.current_node.clone().unwrap().0,
                            &self
                                .file_contents
                                .get(&self.current_node.clone().unwrap().0)
                                .unwrap(),
                            "Unsupported types for division: {:?} {:?}",
                            left_value,
                            right_value
                        ));
                    }
                },
                (NodeValue::Operator(Operator::Modulus(_, _)), left, right) => {
                    match (left, right) {
                        (SystemValue::I32(l), SystemValue::I32(r)) => Ok(SystemValue::I32(l % r)),

                        (SystemValue::I64(l), SystemValue::I64(r)) => Ok(SystemValue::I64(l % r)),
                        (SystemValue::F64(l), SystemValue::F64(r)) => Ok(SystemValue::F64(l % r)),
                        _ => {
                            todo!();
                        }
                    }
                }

                _ => {
                    return Err(compile_error!(
                        "error",
                        node.line,
                        node.column,
                        &self.current_node.clone().unwrap().0,
                        &self
                            .file_contents
                            .get(&self.current_node.clone().unwrap().0)
                            .unwrap(),
                        "Unsupported operation or mismatched types in binary operation: {:?} {:?}",
                        left_value,
                        right_value
                    ));
                }
            }
        } else {
            Err(compile_error!(
                "error",
                node.line,
                node.column,
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "Unsupported node value: {:?}",
                node.value.clone(),
            ))
        }
    }

    fn eval_if_statement(
        &mut self,
        condition: &Box<Node>,
        body: &Box<Node>,
    ) -> Result<SystemValue, String> {
        let condition_result = self.execute_node(&condition)?;
        let mut result = SystemValue::Null;

        if let SystemValue::Bool(value) = condition_result {
            if value {
                result = self.execute_node(&body)?;
            } else if let Some(ref next_node) = condition.next.borrow().as_ref() {
                match (*next_node).value {
                    NodeValue::ControlFlow(ControlFlow::If(ref next_condition, ref next_body)) => {
                        result = self.eval_if_statement(next_condition, next_body)?;
                    }
                    NodeValue::ControlFlow(ControlFlow::Else(ref else_body)) => {
                        result = self.execute_node(&else_body)?;
                    }
                    _ => {}
                }
            }
        }

        Ok(result)
    }

    fn eval_loop_statement(&mut self, body: &Box<Node>) -> Result<SystemValue, String> {
        let mut result = SystemValue::Null;
        loop {
            result = self.execute_node(&body)?;
        }
        Ok(result)
    }

    fn eval_while_statement(
        &mut self,
        condition: &Box<Node>,
        body: &Box<Node>,
    ) -> Result<SystemValue, String> {
        let mut result = SystemValue::Null;
        loop {
            let condition_value = self.execute_node(&condition)?;
            if let SystemValue::Bool(value) = condition_value {
                if value {
                    match self.execute_node(&body) {
                        Ok(val) => {
                            if val == SystemValue::String("break".to_string()) {
                                break;
                            } else if val == SystemValue::String("continue".to_string()) {
                                continue;
                            } else {
                                result = val;
                            }
                        }
                        Err(e) => return Err(e),
                    }
                } else {
                    break;
                }
            } else {
                return Err("Condition must evaluate to a boolean".to_string());
            }
        }
        Ok(result)
    }

    fn eval_for_statement(
        &mut self,
        value: &Box<Node>,
        iterator: &Box<Node>,
        body: &Box<Node>,
    ) -> Result<SystemValue, String> {
        let mut result = SystemValue::Null;

        // イテレータの評価
        let iter_value = self.execute_node(iterator)?;
        if let SystemValue::Array(elements) = iter_value {
            for element in elements {
                // ループ変数に値を設定し、メモリを確保
                let element_address = self.memory_mgr.allocate(element.clone());
                let variable = Variable {
                    value: element.clone(),
                    data_name: String::from(""),
                    address: element_address,
                    is_mutable: true, // 仮に可変とする
                    size: element.size(),
                };
                let var = match value.value {
                    NodeValue::Variable(_, ref v, _, _, _) => v.clone(),
                    _ => String::new(),
                };
                self.context.local_context.insert(var.clone(), variable);

                // ループボディの評価
                match self.execute_node(body) {
                    Ok(val) => {
                        if val == SystemValue::String("break".to_string()) {
                            break;
                        } else if val == SystemValue::String("continue".to_string()) {
                            continue;
                        } else {
                            result = val;
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
        } else {
            return Err(compile_error!(
                "error",
                self.current_node.clone().unwrap().1.line,
                self.current_node.clone().unwrap().1.column,
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "The iterator is not an array",
            ));
        }

        Ok(result)
    }

    pub fn eval_primitive_type(&mut self, node: &Node) -> Result<SystemValue, String> {
        let value = &node.value;
        /*
        // 型を推論
        let inferred_type_result = TypeChecker::infer_type(value);
        debug!("infer_type: {:?}", inferred_type_result);
        match inferred_type_result {
            Ok(type_name) => {
                /*let array_result = match value {
                    NodeValue::DataType(DataType::Array(data_type, array)) => self.eval_array(&data_type, &array)?,
                    _ => SystemValue::Null,
                };*/
                // 型に基づいて値を変換
                let conversion_result = TypeChecker::convert_type_to_value(&type_name, value);
                match conversion_result {
                    Ok(system_value) => Ok(system_value),
                    Err(err) => Err(format!("Type conversion error: {}", err)),
                }
            }
            Err(err) => Err(format!("Type inference error: {}", err)),
        }*/
        match &node.value {
            NodeValue::DataType(DataType::Int(number)) => Ok(SystemValue::I64(*number)),
            NodeValue::DataType(DataType::Float(number)) => Ok(SystemValue::F64(*number)),
            NodeValue::DataType(DataType::String(s)) => {
                // let path = PathBuf::from(s.clone());
                // if path.exists(){
                //     return Ok(SystemValue::System(SystemType::PathBuf(path.clone())));
                // }else{
                return Ok(SystemValue::String(s.clone()));
                // }
            }
            NodeValue::DataType(DataType::Bool(b)) => Ok(SystemValue::Bool(*b)),
            NodeValue::DataType(DataType::Array(data_type, values)) => {
                self.eval_array(&data_type, &values)
            }
            _ => Ok(SystemValue::Null),
        }
    }

    pub fn eval_impl_statement(
        &mut self,
        name: &String,
        members: &Vec<Box<Node>>,
    ) -> Result<SystemValue, String> {
        let context = &mut self.context.global_context;
        if !context.contains_key(name) {
            return Err(compile_error!(
                "error",
                self.current_node.clone().unwrap().1.line(),
                self.current_node.clone().unwrap().1.column(),
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "Struct '{}' is not defined",
                name
            ));
        }

        let struct_var = context.get_mut(name).unwrap();
        let mut structs_map: HashMap<String, SystemValue> = match &struct_var.value {
            SystemValue::Struct(fields) => {
                let mut map = HashMap::new();
                if let SystemValue::Array(members) = &fields[1] {
                    for member in members {
                        if let SystemValue::Tuple(tuple) = member {
                            if let SystemValueTuple::Tuple2(name, value) = &**tuple {
                                if let SystemValue::String(name_str) = &**name {
                                    map.insert(name_str.clone(), (**value).clone());
                                }
                            }
                        }
                    }
                }
                map
            }
            _ => {
                return Err(format!(
                    "The value associated with '{}' is not a Struct.",
                    name
                ));
            }
        };

        let member_map: HashMap<String, SystemValue> = members
            .iter()
            .filter_map(|m| {
                let node_value = &m.value;
                match node_value {
                    NodeValue::Declaration(Declaration::Function(
                        func_name,
                        args,
                        body,
                        return_type,
                        _is_system,
                        is_public,
                    )) => {
                        let member_name = func_name.clone();
                        // 関数の引数、戻り値の型と本体を取得
                        let function_info = SystemValue::Struct(vec![
                            SystemValue::Array(
                                args.iter()
                                    .map(|(arg, _)| SystemValue::__NodeBlock(vec![arg.clone()]))
                                    .collect(),
                            ),
                            SystemValue::Bool(*is_public),
                            SystemValue::__NodeBlock(vec![return_type.clone()]),
                            SystemValue::__NodeBlock(vec![body.clone()]),
                        ]);
                        Some((member_name, function_info))
                    }
                    _ => None,
                }
            })
            .collect();

        structs_map.extend(member_map);

        let updated_struct = SystemValue::Struct(vec![
            SystemValue::String(name.clone()),
            SystemValue::Array(
                structs_map
                    .into_iter()
                    .map(|(k, v)| {
                        SystemValue::Tuple(Box::new(SystemValueTuple::Tuple2(
                            Box::new(SystemValue::String(k)),
                            Box::new(v),
                        )))
                    })
                    .collect(),
            ),
        ]);

        struct_var.value = updated_struct.clone();

        //panic!("{:?}", self.context.global_context);
        debug!(
            "Struct '{}' updated with new members: {:?}",
            name, struct_var.value
        );
        Ok(updated_struct)
    }
    fn eval_struct_statement(
        &mut self,
        name: &String,
        members: &Vec<Box<Node>>,

        is_public: bool,
    ) -> Value {
        let context = &mut self.context.global_context;
        if context.contains_key(&name.clone()) {
            return Err(compile_error!(
                "error",
                self.current_node.clone().unwrap().1.line(),
                self.current_node.clone().unwrap().1.column(),
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "Struct '{}' is already defined",
                name
            ));
        }

        let member_map: HashMap<String, SystemValue> = members
            .iter()
            .filter_map(|m| {
                let member_name = if let NodeValue::Variable(_, ref v, _, _, _) = m.value {
                    v.clone()
                } else {
                    return None;
                };

                let member_type = if let NodeValue::Variable(ref v, _, _, _, _) = m.value {
                    if let NodeValue::DataType(ref data_type) = v.value {
                        match data_type {
                            DataType::Int(_) => SystemValue::String("i64".to_string()),
                            DataType::Float(_) => SystemValue::String("f64".to_string()),
                            DataType::String(_) => SystemValue::String("string".to_string()),
                            DataType::Bool(_) => SystemValue::String("bool".to_string()),
                            DataType::Unit(_) => SystemValue::String("unit".to_string()),
                            _ => SystemValue::String("unknown".to_string()),
                        }
                    } else {
                        SystemValue::String("unknown".to_string())
                    }
                } else {
                    SystemValue::String("unknown".to_string())
                };

                Some((member_name, member_type))
            })
            .collect();

        let member_pointers: Vec<SystemValue> = member_map
            .into_iter()
            .map(|(k, v)| {
                SystemValue::Tuple(Box::new(SystemValueTuple::Tuple2(
                    Box::new(SystemValue::String(k)),
                    Box::new(v),
                )))
            })
            .collect();

        let value = SystemValue::Struct(vec![
            SystemValue::String(name.clone()),
            SystemValue::Array(member_pointers),
        ]);

        let variables = Variable {
            value: value.clone(),
            data_name: name.clone(),
            address: uuid::Uuid::nil(),
            is_mutable: false,
            size: value.to_string().len(),
        };

        self.context.global_context.insert(name.clone(), variables);
        debug!(
            "StructDefined: is_public={:?}, name={:?}, value={:?}, size={:?}",
            is_public,
            name,
            value.clone(),
            value.to_string().len()
        );
        Ok(value)
    }

    fn eval_struct_instance_statement(
        &mut self,
        name: &String,
        init_values: &Vec<(String, Box<Node>)>,
    ) -> Value {
        let context = &mut self.context.global_context;
        if !context.contains_key(&name.clone()) {
            return Err(compile_error!(
                "error",
                self.current_node.clone().unwrap().1.line(),
                self.current_node.clone().unwrap().1.column(),
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "Struct '{}' is not in current scope",
                name
            ));
        }

        let struct_value = context.get(name).unwrap().value.clone();
        if let SystemValue::Struct(ref struct_data) = struct_value {
            if let SystemValue::Array(ref members) = struct_data[1] {
                let mut instance_members: Vec<SystemValue> = Vec::new();

                for (member_name, init_value) in init_values {
                    let member_type = members.iter().find_map(|member| {
                        if let SystemValue::Tuple(ref tuple) = member {
                            if let SystemValueTuple::Tuple2(ref name, _) = **tuple {
                                if let SystemValue::String(ref name_str) = **name {
                                    if name_str == member_name {
                                        return Some(member);
                                    }
                                }
                            }
                        }
                        None
                    });

                    if let Some(_) = member_type {
                        let value = self.execute_node(init_value)?;
                        instance_members.push(SystemValue::Tuple(Box::new(
                            SystemValueTuple::Tuple2(
                                Box::new(SystemValue::Pointer(Box::new(SystemValue::String(
                                    member_name.clone(),
                                )))),
                                Box::new(SystemValue::Pointer(Box::new(value))),
                            ),
                        )));
                    } else {
                        return Err(compile_error!(
                            "error",
                            self.current_node.clone().unwrap().1.line(),
                            self.current_node.clone().unwrap().1.column(),
                            &self.current_node.clone().unwrap().0,
                            &self
                                .file_contents
                                .get(&self.current_node.clone().unwrap().0)
                                .unwrap(),
                            "Member '{}' not found in struct '{}'",
                            member_name,
                            name
                        ));
                    }
                }

                // メンバ関数も追加
                for member in members {
                    if let SystemValue::Tuple(ref tuple) = member {
                        if let SystemValueTuple::Tuple2(ref name, ref value) = **tuple {
                            if let SystemValue::String(ref name_str) = **name {
                                if !instance_members.iter().any(|m| {
                                    if let SystemValue::Tuple(ref t) = m {
                                        if let SystemValueTuple::Tuple2(ref n, _) = **t {
                                            if let SystemValue::Pointer(ref p) = **n {
                                                if let SystemValue::String(ref s) = **p {
                                                    return s == name_str;
                                                }
                                            }
                                        }
                                    }
                                    false
                                }) {
                                    instance_members.push(SystemValue::Tuple(Box::new(
                                        SystemValueTuple::Tuple2(
                                            Box::new(SystemValue::Pointer(Box::new(
                                                SystemValue::String(name_str.clone()),
                                            ))),
                                            Box::new(*value.clone()),
                                        ),
                                    )));
                                }
                            }
                        }
                    }
                }

                let instance_value = SystemValue::Struct(vec![
                    SystemValue::String(name.clone()),
                    SystemValue::Array(instance_members),
                ]);

                return Ok(instance_value);
            }
        }

        Err("Invalid struct format".to_string())
    }
    fn eval_member_access(&mut self, member: &Box<Node>, item: &Box<Node>) -> Value {
        let member_value = self.execute_node(&*member)?;

        let mut ret = SystemValue::Null;
        let item_value =
            match item.value() {
                NodeValue::Call(name, args, is_system) => match name.as_str() {
                    "max" => {
                        if is_system {
                            let mut evaluated_args = Vec::new();
                            for arg in args.clone() {
                                let evaluated_arg = self.execute_node(&arg)?;
                                //debug!("args: {:?}", evaluated_arg);
                                evaluated_args.push(evaluated_arg);
                            }

                            let num1 = match member.value {
                                NodeValue::DataType(DataType::Int(ref v)) => v.clone(),
                                NodeValue::Variable(_, ref v, _, _, _) => {
                                    let variable_data =
                                        self.context.local_context.get(v).cloned().or_else(|| {
                                            self.context.global_context.get(v).cloned()
                                        });
                                    if let Some(mut variable) = variable_data {
                                        if let SystemValue::I64(ref v) = variable.value {
                                            v.clone()
                                        } else {
                                            -1
                                        }
                                    } else {
                                        -1
                                    }
                                }
                                _ => -1,
                            };
                            let num2 = match args[0].clone().value {
                                NodeValue::DataType(DataType::Int(ref v)) => v.clone(),
                                NodeValue::Variable(_, ref v, _, _, _) => {
                                    let variable_data =
                                        self.context.local_context.get(v).cloned().or_else(|| {
                                            self.context.global_context.get(v).cloned()
                                        });
                                    if let Some(mut variable) = variable_data {
                                        if let SystemValue::I64(ref v) = variable.value {
                                            v.clone()
                                        } else {
                                            -1
                                        }
                                    } else {
                                        -1
                                    }
                                }

                                _ => -1,
                            };
                            ret = (num1.max(num2)).into();
                            ret.clone()
                        } else {
                            SystemValue::Null
                        }
                    }
                    "min" => {
                        if is_system {
                            let mut evaluated_args = Vec::new();
                            for arg in args.clone() {
                                let evaluated_arg = self.execute_node(&arg)?;
                                //debug!("args: {:?}", evaluated_arg);
                                evaluated_args.push(evaluated_arg);
                            }

                            let num1 = match member.value {
                                NodeValue::DataType(DataType::Int(ref v)) => v.clone(),
                                NodeValue::Variable(_, ref v, _, _, _) => {
                                    let variable_data =
                                        self.context.local_context.get(v).cloned().or_else(|| {
                                            self.context.global_context.get(v).cloned()
                                        });
                                    if let Some(mut variable) = variable_data {
                                        if let SystemValue::I64(ref v) = variable.value {
                                            v.clone()
                                        } else {
                                            -1
                                        }
                                    } else {
                                        -1
                                    }
                                }
                                _ => -1,
                            };
                            let num2 = match args[0].clone().value {
                                NodeValue::DataType(DataType::Int(ref v)) => v.clone(),
                                NodeValue::Variable(_, ref v, _, _, _) => {
                                    let variable_data =
                                        self.context.local_context.get(v).cloned().or_else(|| {
                                            self.context.global_context.get(v).cloned()
                                        });
                                    if let Some(mut variable) = variable_data {
                                        if let SystemValue::I64(ref v) = variable.value {
                                            v.clone()
                                        } else {
                                            -1
                                        }
                                    } else {
                                        -1
                                    }
                                }

                                _ => -1,
                            };

                            ret = (num1.min(num2)).into();
                            ret.clone()
                        } else {
                            SystemValue::Null
                        }
                    }
                    "split" => {
                        if is_system {
                            let mut evaluated_args = Vec::new();
                            for arg in args {
                                let evaluated_arg = self.execute_node(&arg)?;
                                //debug!("args: {:?}", evaluated_arg);
                                evaluated_args.push(evaluated_arg);
                            }

                            let string = match member.value {
                                NodeValue::Variable(_, ref v, _, _, _) => {
                                    let variable_data =
                                        self.context.local_context.get(v).cloned().or_else(|| {
                                            self.context.global_context.get(v).cloned()
                                        });
                                    if let Some(mut variable) = variable_data {
                                        if let SystemValue::String(ref v) = variable.value {
                                            v.clone()
                                        } else {
                                            String::new()
                                        }
                                    } else {
                                        String::new()
                                    }
                                }
                                NodeValue::DataType(DataType::String(ref v)) => v.clone(),
                                _ => String::new(),
                            };
                            let split_str = match evaluated_args[0] {
                                SystemValue::String(ref v) => v.clone(),
                                _ => String::new(),
                            };
                            let sp = string.split(&split_str);
                            let sp_vec: Vec<&str> = sp.collect();
                            ret = SystemValue::from(sp_vec);
                            ret.clone()
                        } else {
                            SystemValue::Null
                        }
                    }
                    "to_string" => {
                        if is_system {
                            let mut evaluated_args = Vec::new();
                            for arg in args {
                                let evaluated_arg = self.execute_node(&arg)?;
                                //debug!("args: {:?}", evaluated_arg);
                                evaluated_args.push(evaluated_arg);
                            }
                            let mut value = match member.value {
                                NodeValue::Variable(_, ref v, _, _, _) => {
                                    let variable_data =
                                        self.context.local_context.get(v).cloned().or_else(|| {
                                            self.context.global_context.get(v).cloned()
                                        });
                                    if let Some(mut variable) = variable_data {
                                        variable.value
                                    } else {
                                        SystemValue::Null
                                    }
                                }
                                NodeValue::DataType(DataType::Int(ref v)) => SystemValue::I64(*v),
                                NodeValue::DataType(DataType::Float(ref v)) => SystemValue::F64(*v),
                                NodeValue::DataType(DataType::String(ref v)) => {
                                    SystemValue::String(v.clone())
                                }
                                _ => SystemValue::Null,
                            };
                            ret = SystemValue::String(value.to_string());
                            ret.clone()
                        } else {
                            SystemValue::Null
                        }
                    }

                    _ => SystemValue::Null,
                },
                _ => SystemValue::Null,
            };
        //panic!("member {:?} item {:?}",member.value(),item.value());

        Ok(ret)
    }
    fn eval_scope_resolution(&mut self, scope_resolution: &Vec<Box<Node>>) -> Value {
        let context = &mut self.context.global_context;
        let first_name = match scope_resolution[0].value {
            NodeValue::Variable(_, ref v, _, _, _) => v.clone(),

            NodeValue::ScopeResolution(ref scope) => {
                let mut name = String::new();
                for s in scope {
                    name = match s.value {
                        NodeValue::Variable(_, ref v, _, _, _) => v.clone(),
                        _ => return Err("Invalid scope resolution".to_string()),
                    };
                }
                name
            }
            _ => return Err("Invalid scope resolution".to_string()),
        };

        if !context.contains_key(&first_name) {
            return Err(compile_error!(
                "error",
                self.current_node.clone().unwrap().1.line(),
                self.current_node.clone().unwrap().1.column(),
                &self.current_node.clone().unwrap().0,
                &self
                    .file_contents
                    .get(&self.current_node.clone().unwrap().0)
                    .unwrap(),
                "Struct '{}' is not in current scope",
                first_name
            ));
        }

        let mut current_value = context.get(&first_name).unwrap().value.clone();

        for (i, value) in scope_resolution.iter().enumerate() {
            if i == 0 {
                continue;
            }

            let member_name = match value.value {
                NodeValue::Variable(_, ref v, _, _, _) => v.clone(),
                NodeValue::Call(ref func_name, ref args, ref is_system) => {
                    //panic!("{:?} {:?} {:?}",func_name,args,is_system);
                    // 関数呼び出しを処理
                    if let SystemValue::Struct(ref struct_data) = current_value {
                        if let SystemValue::Array(ref members) = struct_data[1] {
                            let member_value = members.iter().find_map(|member| {
                                if let SystemValue::Tuple(ref tuple) = member {
                                    if let SystemValueTuple::Tuple2(ref name, ref value) = **tuple {
                                        if let SystemValue::String(ref name_str) = **name {
                                            if name_str == func_name {
                                                return Some(value.clone());
                                            }
                                        }
                                    }
                                }
                                None
                            });

                            if let Some(member_value) = member_value {
                                if let SystemValue::Struct(ref func_info) = *member_value {
                                    //panic!("{:?}",*member_value);
                                    let _body = &func_info[3]; // 関数のbody (SystemValue::NodeBlockで格納)
                                                               // body が SystemValue::NodeBlockであることを確認
                                    let body_array =
                                        if let SystemValue::__NodeBlock(ref body_elements) = *_body
                                        {
                                            body_elements
                                        } else {
                                            return Err(
                                            "Invalid body format: Expected SystemValue::NodeBlock"
                                                .to_string(),
                                        );
                                        };
                                    //panic!("{:?}",body_array);
                                    // 関数のbodyを評価する (SystemValue::NodeBlock内の各ノードを評価)
                                    return self.eval_block(body_array);
                                } else {
                                    return Err(compile_error!(
                                        "error",
                                        self.current_node.clone().unwrap().1.line(),
                                        self.current_node.clone().unwrap().1.column(),
                                        &self.current_node.clone().unwrap().0,
                                        &self
                                            .file_contents
                                            .get(&self.current_node.clone().unwrap().0)
                                            .unwrap(),
                                        "Function '{}' not found in struct '{}'",
                                        func_name,
                                        first_name
                                    ));
                                }
                            } else {
                                return Err(compile_error!(
                                    "error",
                                    self.current_node.clone().unwrap().1.line(),
                                    self.current_node.clone().unwrap().1.column(),
                                    &self.current_node.clone().unwrap().0,
                                    &self
                                        .file_contents
                                        .get(&self.current_node.clone().unwrap().0)
                                        .unwrap(),
                                    "Function '{}' not found in struct '{}'",
                                    func_name,
                                    first_name
                                ));
                            }
                        }
                    }
                    return Err("Invalid struct format".to_string());
                }
                _ => return Err("Invalid scope resolution".to_string()),
            };

            if let SystemValue::Struct(ref struct_data) = current_value {
                if let SystemValue::Array(ref members) = struct_data[1] {
                    let member_value = members.iter().find_map(|member| {
                        if let SystemValue::Tuple(ref tuple) = member {
                            if let SystemValueTuple::Tuple2(ref name, ref value) = **tuple {
                                if let SystemValue::String(ref name_str) = **name {
                                    if name_str == &member_name {
                                        return Some(value.clone());
                                    }
                                }
                            }
                        }
                        None
                    });

                    if let Some(value) = member_value {
                        current_value = *value;
                    } else {
                        return Err(compile_error!(
                            "error",
                            self.current_node.clone().unwrap().1.line(),
                            self.current_node.clone().unwrap().1.column(),
                            &self.current_node.clone().unwrap().0,
                            &self
                                .file_contents
                                .get(&self.current_node.clone().unwrap().0)
                                .unwrap(),
                            "Member '{}' not found in struct '{}'",
                            member_name,
                            first_name
                        ));
                    }
                }
            } else {
                return Err("Invalid struct format".to_string());
            }
        }

        Ok(current_value)
    }
    // ノードを評価
    fn execute_node(&mut self, node: &Node) -> Value {
        let original_node = self.current_node.clone();
        self.current_node = Some((
            self.current_node.as_ref().unwrap().0.clone(),
            Box::new(node.clone()),
        ));
        let mut result = SystemValue::Null;
        let node_value = node.clone().value();

        //debug!("global_contexts: {:?}", self.context.global_context.clone());
        //debug!("local_contexts: {:?}", self.context.local_context.clone());
        //debug!("used_context: {:?}", self.context.used_context.clone());
        //debug!("current_node: {:?}", self.current_node.clone());
        //debug!("current_node: {:?}", node.clone());
        match &node.value {
            NodeValue::EndStatement => {
                result = SystemValue::Null;
            }
            // nullはDataType::Nullを使うように変更しろよ
            //
            NodeValue::DataType(DataType::Null) => {
                result = SystemValue::Null;
            }

            NodeValue::Null => {
                result = SystemValue::Null;
            }
            NodeValue::Block(block) => {
                result = self.eval_block(&block)?;
            }
            NodeValue::Use(scope_resolution) => {
                result = self.eval_use(scope_resolution)?;
            }

            NodeValue::ScopeResolution(scope_resolution) => {
                result = self.eval_scope_resolution(&scope_resolution)?;
            }
            NodeValue::Declaration(Declaration::Impl(name, members)) => {
                result = self.eval_impl_statement(name, &members)?;
            }

            NodeValue::StructInstance(name, init_values) => {
                result = self.eval_struct_instance_statement(name, &init_values)?;
            }
            NodeValue::Declaration(Declaration::Struct(name, members, is_public)) => {
                result = self.eval_struct_statement(name, &members, *is_public)?;
            }

            NodeValue::MemberAccess(member, item) => {
                result = self.eval_member_access(&member, &item)?;
            }
            NodeValue::Operator(Operator::Range(start, max)) => {
                let _start_value = self.execute_node(start)?;
                let _max_value = self.execute_node(max)?;

                // start_value と max_value の値を取り出す
                let start_value = match _start_value {
                    SystemValue::U64(val) => val,
                    SystemValue::I64(val) => val as u64,
                    _ => {
                        return Err(compile_error!(
                            "error",
                            node.line,
                            node.column,
                            &self.current_node.clone().unwrap().0,
                            &self
                                .file_contents
                                .get(&self.current_node.clone().unwrap().0)
                                .unwrap(),
                            "Invalid type for start_value: expected U64, got {:?}",
                            _start_value
                        ));
                    }
                };

                let max_value = match _max_value {
                    SystemValue::U64(val) => val,
                    SystemValue::I64(val) => val as u64,
                    _ => {
                        return Err(compile_error!(
                            "error",
                            node.line,
                            node.column,
                            &self.current_node.clone().unwrap().0,
                            &self
                                .file_contents
                                .get(&self.current_node.clone().unwrap().0)
                                .unwrap(),
                            "Invalid type for max_value: expected U64, got {:?}",
                            _max_value
                        ));
                    }
                };

                let array: Vec<u64> = (start_value..=max_value).collect();
                result = SystemValue::from(array);
            }
            NodeValue::ControlFlow(ControlFlow::Break) => {
                result = SystemValue::String("break".into());
            }
            NodeValue::ControlFlow(ControlFlow::Continue) => {
                result = SystemValue::String("continue".into());
            }
            NodeValue::ControlFlow(ControlFlow::Loop(body)) => {
                result = self.eval_loop_statement(body)?;
            }
            NodeValue::ControlFlow(ControlFlow::If(condition, body)) => {
                result = self.eval_if_statement(condition, body)?;
            }
            NodeValue::ControlFlow(ControlFlow::While(condition, body)) => {
                result = self.eval_while_statement(condition, body)?;
            }
            NodeValue::ControlFlow(ControlFlow::For(value, iterator, body)) => {
                result = self.eval_for_statement(value, iterator, body)?;
            }
            NodeValue::Operator(Operator::Eq(_, _))
            | NodeValue::Operator(Operator::Ne(_, _))
            | NodeValue::Operator(Operator::Lt(_, _))
            | NodeValue::Operator(Operator::Gt(_, _))
            | NodeValue::Operator(Operator::Le(_, _))
            | NodeValue::Operator(Operator::Ge(_, _))
            | NodeValue::Operator(Operator::And(_, _))
            | NodeValue::Operator(Operator::Or(_, _)) => {
                result = self.eval_binary_condition(&node.clone())?;
            }
            NodeValue::Operator(Operator::BitAnd(_, _))
            | NodeValue::Operator(Operator::BitOr(_, _))
            | NodeValue::Operator(Operator::BitXor(_, _))
            | NodeValue::Operator(Operator::ShiftLeft(_, _))
            | NodeValue::Operator(Operator::ShiftRight(_, _)) => {
                result = self.eval_binary_bit(&node.clone())?;
            }

            NodeValue::DataType(DataType::Int(_))
            | NodeValue::DataType(DataType::Float(_))
            | NodeValue::DataType(DataType::String(_))
            | NodeValue::DataType(DataType::Bool(_))
            | NodeValue::DataType(DataType::Array(_, _)) => {
                result = self.eval_primitive_type(&node.clone())?;
            }
            NodeValue::Include(file_name) => {
                result = self.eval_include(file_name)?;
            }
            NodeValue::MultiComment(content, (line, column)) => {
                self.eval_multi_comment(&content, &(*line, *column))?;
                result = SystemValue::Null;
            }
            NodeValue::SingleComment(content, (line, column)) => {
                self.eval_single_comment(&content, &(*line, *column))?;
                result = SystemValue::Null;
            }
            NodeValue::Call(name, args, is_system) => {
                result = self.eval_call(&node.clone(), name, args, is_system)?;
            }
            NodeValue::Declaration(Declaration::Type(type_name, _type, is_public)) => {
                result = self.eval_type_declaration(type_name, _type, *is_public)?;
            }
            NodeValue::Variable(_, name, _, _, _) => {
                result = self.eval_variable(name)?;
            }
            NodeValue::ControlFlow(ControlFlow::Return(ret)) => {
                result = self.eval_return(ret)?;
            }
            NodeValue::Operator(Operator::Increment(lhs)) => {
                result = self.eval_binary_increment(lhs)?;
            }
            NodeValue::Operator(Operator::Decrement(lhs)) => {
                result = self.eval_binary_decrement(lhs)?;
            }
            NodeValue::Operator(Operator::Add(_, _))
            | NodeValue::Operator(Operator::Sub(_, _))
            | NodeValue::Operator(Operator::Mul(_, _))
            | NodeValue::Operator(Operator::Div(_, _))
            | NodeValue::Operator(Operator::Modulus(_, _))
            | NodeValue::Operator(Operator::AddAssign(_, _))
            | NodeValue::Operator(Operator::SubAssign(_, _))
            | NodeValue::Operator(Operator::MulAssign(_, _))
            | NodeValue::Operator(Operator::DivAssign(_, _))
            | NodeValue::Operator(Operator::ModulusAssign(_, _)) => {
                result = self.eval_binary_op(&node.clone())?;
            }
            NodeValue::Declaration(Declaration::Function(
                name,
                args,
                body,
                return_type,
                is_system,
                is_public,
            )) => {
                result = self.eval_function(name, args, body, return_type, is_system, is_public)?;
            }

            NodeValue::Assign(var_name, value, index) => {
                result = self.eval_assign(&node.clone(), &var_name, &value, &index)?;
            }

            NodeValue::Declaration(Declaration::Variable(
                var_name,
                data_type,
                value,
                is_local,
                is_mutable,
                is_public,
            )) => {
                result = self.eval_variable_declaration(
                    &node.clone(),
                    var_name,
                    data_type,
                    value,
                    is_local,
                    is_mutable,
                    is_public,
                )?;
            }

            NodeValue::Declaration(Declaration::Const(
                var_name,
                data_type,
                value,
                is_local,
                is_public,
            )) => {
                result = self.eval_const_declaration(
                    &node.clone(),
                    var_name,
                    data_type,
                    value,
                    is_local,
                    is_public,
                )?;
            }

            _ => {
                return Err(compile_error!(
                    "error",
                    node.line,
                    node.column,
                    &self.current_node.clone().unwrap().0,
                    &self
                        .file_contents
                        .get(&self.current_node.clone().unwrap().0)
                        .unwrap(),
                    "Unknown node value: {:?}",
                    node.value
                ));
            }
        }
        self.current_node = original_node;
        Ok(result)
    }
}
