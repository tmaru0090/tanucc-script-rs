// 予約済みキーワード
pub static RESERVED_WORDS: &[&str] = &[
    "if", "else", "while", "for", "break", "continue", "i32", "i64", "f32", "f64", "u32", "u64",
    "type", "let", "l", "var", "v", "fn", "mut", "loop", "=", "+", "++", "-", "--", "+=", "-=",
    "*", "*=", "/", "/=", "{", "}", "[", "]", "mod", "use", "bool", "struct", "enum", "%", "&",
    "&=", "|", "|=", "^", "~", "^=",
];

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(any(feature = "full", feature = "parser"))]
use crate::parser::syntax::Node;
// トークンの種類
#[derive(PartialEq, Debug, Clone, Serialize)]
pub enum TokenType {
    Add,                                       // 加算
    Sub,                                       // 減算
    Mul,                                       // 乗算
    Div,                                       // 除算
    Increment,                                 // 増加
    Decrement,                                 // 減少
    AddAssign,                                 // 加算代入
    SubAssign,                                 // 減算代入
    MulAssign,                                 // 乗算代入
    DivAssign,                                 // 除算代入
    Eq,                                        // 等価性
    Ne,                                        // 不等価性
    Lt,                                        // より小さい
    Gt,                                        // より大きい
    Le,                                        // 以下
    Ge,                                        // 以上
    And,                                       // 論理積
    Or,                                        // 論理和
    BitAnd,                                    // ビット単位の論理積
    BitOr,                                     // ビット単位の論理和
    BitXor,                                    // ビット単位の排他的論理和
    BitNot,                                    // ビット単位の補数
    ShiftLeft,                                 // ビットシフト左
    ShiftRight,                                // ビットシフト右
    BitAndAssign,                              // ビット単位の論理積と代入
    BitOrAssign,                               // ビット単位の論理和と代入
    BitXorAssign,                              // ビット単位の排他的論理和と代入
    ShiftLeftAssign,                           // ビットシフト左と代入
    ShiftRightAssign,                          // ビットシフト右と代入
    Ident,                                     // 識別子
    Number,                                    // 数値
    LeftParen,                                 // 左括弧
    RightParen,                                // 右括弧
    RightCurlyBrace,                           // 左波括弧
    LeftCurlyBrace,                            // 右波括弧
    LeftSquareBrace,                           // 左鍵括弧
    RightSquareBrace,                          // 右鍵括弧
    Conma,                                     // コンマ
    Equals,                                    // 代入
    AtSign,                                    // @
    Semi,                                      // セミコロン
    Colon,                                     // コロン
    DoubleQuote,                               // ダブルクオーテーション
    SingleQuote,                               // シングルクオーテーション
    SingleComment(String, (usize, usize)),     // 単一コメント
    MultiComment(Vec<String>, (usize, usize)), // 複数行コメント
    RightArrow,                                // 右矢印
    Eof,                                       // トークンの終わり
    Range,                                     // 範囲指定
    ScopeResolution,                           //  スコープ解決
}
// 制御構造
#[cfg(any(feature = "full", feature = "parser"))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum ControlFlow {
    If(Box<Node>, Box<Node>),             // if文(条件,ボディ)
    Else(Box<Node>),                      // else文(ボディ)
    ElseIf(Box<Node>, Box<Node>),         // else if文(条件,ボディ)
    Loop(Box<Node>),                      // loop文(ボディ)
    While(Box<Node>, Box<Node>),          // while文(条件,ボディ)
    For(Box<Node>, Box<Node>, Box<Node>), // for文(初期化式(値),(コレクション値|イテレータ|配列),ボディ)
    Return(Box<Node>),                    // return文(値)
    Break,                                // break文
    Continue,                             // continue文
}

#[cfg(any(feature = "full", feature = "parser"))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Operator {
    Eq(Box<Node>, Box<Node>),               // 等価(左辺,右辺)
    Ne(Box<Node>, Box<Node>),               // 不等価(左辺,右辺)
    Lt(Box<Node>, Box<Node>),               // より小さい(左辺,右辺)
    Gt(Box<Node>, Box<Node>),               // より大きい(左辺,右辺)
    Le(Box<Node>, Box<Node>),               // 以下(左辺,右辺)
    Ge(Box<Node>, Box<Node>),               // 以上(左辺,右辺)
    And(Box<Node>, Box<Node>),              // 論理積(左辺,右辺)
    Or(Box<Node>, Box<Node>),               // 論理和(左辺,右辺)
    Add(Box<Node>, Box<Node>),              // 加算(左辺,右辺)
    Sub(Box<Node>, Box<Node>),              // 減算(左辺,右辺)
    Mul(Box<Node>, Box<Node>),              // 乗算(左辺,右辺)
    Div(Box<Node>, Box<Node>),              // 除算(左辺,右辺)
    Increment(Box<Node>),                   // 増加(左辺)
    Decrement(Box<Node>),                   // 減少(左辺)
    AddAssign(Box<Node>, Box<Node>),        // 加算代入(左辺,右辺)
    SubAssign(Box<Node>, Box<Node>),        // 減算代入(左辺,右辺)
    MulAssign(Box<Node>, Box<Node>),        // 乗算代入(左辺,右辺)
    DivAssign(Box<Node>, Box<Node>),        // 除算代入(左辺,右辺)
    BitAnd(Box<Node>, Box<Node>),           // ビット単位の論理積(左辺,右辺)
    BitOr(Box<Node>, Box<Node>),            // ビット単位の論理和(左辺,右辺)
    BitXor(Box<Node>, Box<Node>),           // ビット単位の排他的論理和(左辺,右辺)
    BitNot(Box<Node>),                      // ビット単位の補数(左辺,右辺)
    ShiftLeft(Box<Node>, Box<Node>),        // ビットシフト左(左辺,右辺)
    ShiftRight(Box<Node>, Box<Node>),       // ビットシフト右(左辺,右辺)
    BitAndAssign(Box<Node>, Box<Node>),     // ビット単位の論理積と代入(左辺,右辺)
    BitOrAssign(Box<Node>, Box<Node>),      // ビット単位の論理和と代入(左辺,右辺)
    BitXorAssign(Box<Node>, Box<Node>),     // ビット単位の排他的論理和と代入(左辺,右辺)
    ShiftLeftAssign(Box<Node>, Box<Node>),  // ビットシフト左と代入(左辺,右辺)
    ShiftRightAssign(Box<Node>, Box<Node>), // ビットシフト右と代入(左辺,右辺)
    Range(Box<Node>, Box<Node>),            // 範囲指定(左辺,右辺)
}

// 基本型
#[cfg(any(feature = "full", feature = "parser"))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    Int(i64),       // 数値型(64bit整数値)
    Float(f64),     // 浮動小数点型(64bit小数値)
    String(String), // 文字列型(String)
    Bool(bool),     // ブーリアン値(bool)
    Unit(()),       // Unit値(())
}

// 定義
#[cfg(any(feature = "full", feature = "parser"))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Declaration {
    Variable(Box<Node>, Box<Node>, Box<Node>, bool, bool), // 変数定義()
    Struct(String, Vec<Box<Node>>),                        // 構造体定義()
    Impl(String, Vec<Box<Node>>),                          // 構造体実装()
    Function(String, Vec<(Box<Node>, String)>, Box<Node>, Box<Node>, bool), // 関数定義()
    CallBackFunction(String, Vec<(Box<Node>, String)>, Box<Node>, Box<Node>, bool), // コールバック関数定義()
    Type(Box<Node>, Box<Node>),       // 型定義,型エイリアス()
    Array(Box<Node>, Vec<Box<Node>>), // 配列(型名,値)
}

// NodeValue
#[cfg(any(feature = "full", feature = "parser"))]
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum NodeValue {
    ControlFlow(ControlFlow),                  // 制御フロー
    Operator(Operator),                        // 演算子
    DataType(DataType),                        // 型
    Declaration(Declaration),                  // 定義
    Assign(Box<Node>, Box<Node>, Box<Node>),   // 代入
    Block(Vec<Box<Node>>),                     // ブロック
    Variable(Box<Node>, String, bool, bool),   // 変数(型名,変数名,可変性フラグ,参照型フラグ)
    Call(String, Vec<Node>, bool),             // 関数呼び出し
    ScopeResolution(Vec<Box<Node>>),           // スコープ解決
    MultiComment(Vec<String>, (usize, usize)), // 複数行コメント
    SingleComment(String, (usize, usize)),     // 単一コメント
    Include(String),                           // インクルード
    Mod(String),                               // モジュール宣言
    ModDeclaration(String, Vec<Box<Node>>),    // モジュール定義
    Use(String, Box<Node>),                    // インポート宣言
    EndStatement,                              // ステートメントの終わり
    Null,                                      // 値なし
    Unknown,                                   // 不明な値(通常到達はしない値)
}
// デフォルト値(デフォルト値はNull)
#[cfg(any(feature = "full", feature = "parser"))]
impl Default for NodeValue {
    fn default() -> Self {
        NodeValue::Null
    }
}

impl From<Box<Node>> for DataType {
    fn from(node: Box<Node>) -> Self {
        match *node {
            Node {
                value: NodeValue::Variable(_, ref name, _, _),
                ..
            } => DataType::String(name.clone()),
            _ => DataType::Unit(()), // 他のケースに対するデフォルト処理
        }
    }
}
