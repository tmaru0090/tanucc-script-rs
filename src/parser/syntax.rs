use crate::compile_error;
use crate::error::*;
use crate::lexer::tokenizer::Token;
use crate::traits::*;
use crate::types::*;
use anyhow::{anyhow, Context, Result as R};
use log::{debug, error, info};
use property_rs::Property;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
#[derive(Debug, PartialEq, Clone, Property)]
pub struct Node {
    #[property(get)]
    pub value: NodeValue,
    #[property(get)]
    //pub next: Option<Box<Node>>,
    pub next: Rc<RefCell<Option<Box<Node>>>>,
    #[property(get)]
    pub line: usize,
    #[property(get)]
    pub column: usize,
    #[property(get)]
    pub is_statement: bool,
}
/*
pub struct NodeIter<'a> {
    current: Option<&'a Node>,
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = &'a Node;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.current {
            self.current = node.next.borrow().as_deref();
            Some(node)
        } else {
            None
        }
    }
}


impl<'a> NodeIter<'a> {
    pub fn new(node: &'a Node) -> Self {
        NodeIter {
            current: Some(node),
        }
    }
}

impl Node {
    pub fn iter(&self) -> NodeIter {
        NodeIter::new(self)
    }
}
*/

pub struct NodeIter {
    current: Option<Rc<RefCell<Node>>>,
}

impl Iterator for NodeIter {
    type Item = Rc<RefCell<Node>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.current.take() {
            // nextフィールドからOption<Box<Node>>を取り出す
            if let Some(next_node) = node.borrow().next.borrow().as_ref() {
                self.current = Some(Rc::new(RefCell::new(*next_node.clone()))); // Box<Node>をRc<RefCell<Node>>に変換
            } else {
                self.current = None; // 次のノードがない場合
            }
            Some(node)
        } else {
            None
        }
    }
}
impl NodeIter {
    pub fn new(node: Rc<RefCell<Node>>) -> Self {
        NodeIter {
            current: Some(node),
        }
    }
}

pub struct NodeIterMut {
    current: Option<Rc<RefCell<Node>>>,
}

impl Iterator for NodeIterMut {
    type Item = Rc<RefCell<Node>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.current.take() {
            // nextフィールドからOption<Box<Node>>を取り出す
            if let Some(next_node) = node.borrow_mut().next.borrow_mut().as_ref() {
                self.current = Some(Rc::new(RefCell::new(*(next_node).clone())));
            // Box<Node>をRc<RefCell<Node>>に変換
            } else {
                self.current = None; // 次のノードがない場合
            }
            Some(node)
        } else {
            None
        }
    }
}

impl NodeIterMut {
    pub fn new(node: Rc<RefCell<Node>>) -> Self {
        NodeIterMut {
            current: Some(node),
        }
    }
}

impl Node {
    pub fn iter_mut(&self) -> NodeIterMut {
        NodeIterMut::new(Rc::new(RefCell::new(self.clone()))) // ここを変更
    }
}
impl Node {
    pub fn iter(&self) -> NodeIter {
        NodeIter::new(Rc::new(RefCell::new(self.clone()))) // ここを変更
    }
}

impl Default for Node {
    fn default() -> Self {
        Node {
            value: NodeValue::default(),
            next: Rc::new(RefCell::new(None)), // Rc<RefCell<Option<Box<Node>>>>として初期化
            line: 0,
            column: 0,
            is_statement: false,
        }
    }
}
impl Node {
    pub fn new(value: NodeValue, next: Option<Box<Node>>, line: usize, column: usize) -> Self {
        Node {
            value,
            next: Rc::new(RefCell::new(next)), // nextをRc<RefCell<Option<Box<Node>>>>として初期化
            line,
            column,
            is_statement: false,
        }
    }

    pub fn is_next(&self) -> bool {
        // Refを借りて中身のOptionを確認する
        self.next.borrow().is_some()
    }
    pub fn set_next(&self, next: Rc<RefCell<Option<Box<Node>>>>) {
        *self.next.borrow_mut() = Some(next.borrow().clone().unwrap());
    }
}
#[derive(Debug, Clone)]
pub struct Parser<'a> {
    input_content: String,
    input_path: String,
    tokens: &'a Vec<Token>,
    i: usize,
    is_statement: bool,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a Vec<Token>, input_path: &str, input_content: String) -> Self {
        Parser {
            tokens,
            i: 0,
            input_path: input_path.to_string(),
            input_content,
            is_statement: false,
        }
    }
    pub fn input_content(&self) -> String {
        self.input_content.clone()
    }
    pub fn input_path(&self) -> String {
        self.input_path.clone()
    }
    pub fn new_add(left: Box<Node>, right: Box<Node>, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::Operator(Operator::Add(left, right)),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_sub(left: Box<Node>, right: Box<Node>, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::Operator(Operator::Sub(left, right)),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_mul(left: Box<Node>, right: Box<Node>, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::Operator(Operator::Mul(left, right)),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_div(left: Box<Node>, right: Box<Node>, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::Operator(Operator::Div(left, right)),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_int(value: i64, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::DataType(DataType::Int(value)),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_float(value: f64, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::DataType(DataType::Float(value)),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_string(value: String, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::DataType(DataType::String(value)),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_bool(value: bool, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::DataType(DataType::Bool(value)),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_unit(line: usize, column: usize) -> Box<Node> {
        let node = Node::new(NodeValue::DataType(DataType::Unit(())), None, line, column);
        Box::new(node)
    }
    pub fn new_user_syntax(
        name: String,
        syntax: Box<Node>,
        line: usize,
        column: usize,
    ) -> Box<Node> {
        let node = Node::new(NodeValue::UserSyntax(name, syntax), None, line, column);
        Box::new(node)
    }

    pub fn new_include(include_file: String, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(NodeValue::Include(include_file), None, line, column);
        Box::new(node)
    }

    pub fn new_function(
        func_name: String,
        args: Vec<(Box<Node>, String)>,
        body: Box<Node>,
        return_type: Box<Node>,
        is_system: bool,
        line: usize,
        column: usize,
    ) -> Box<Node> {
        let node = Node::new(
            NodeValue::Declaration(Declaration::Function(
                func_name,
                args,
                body,
                return_type,
                is_system,
            )),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_variable(name: String, expr: Box<Node>, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::Variable(
                Parser::<'a>::new_null(line, column),
                name.clone(),
                false,
                false,
                None,
            ),
            Some(expr),
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_return(expr: Box<Node>, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::ControlFlow(ControlFlow::Return(expr)),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn new_unknown(&self) -> Box<Node> {
        let node = Node::new(
            NodeValue::Unknown,
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        Box::new(node)
    }

    pub fn new_null(line: usize, column: usize) -> Box<Node> {
        let node = Node::new(NodeValue::Null, None, line, column);
        Box::new(node)
    }

    pub fn new_block(block: Vec<Box<Node>>, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(NodeValue::Block(block), None, line, column);
        Box::new(node)
    }

    pub fn new_assign(left: Box<Node>, right: Box<Node>, line: usize, column: usize) -> Box<Node> {
        let node = Node::new(
            NodeValue::Assign(left, right, Box::new(Node::default())),
            None,
            line,
            column,
        );
        Box::new(node)
    }

    pub fn from_parse(
        tokens: &Vec<Token>,
        input_path: &str,
        input_content: String,
    ) -> R<Box<Node>, String> {
        let mut parser = Parser::new(tokens, input_path, input_content);
        parser.parse()
    }

    fn current_token(&self) -> Option<&Token> {
        self.tokens.get(self.i)
    }

    fn peek_next_token(&mut self, i: usize) -> Option<Token> {
        if (self.i + i) < self.tokens.len() {
            Some(self.tokens[self.i + i].clone())
        } else {
            None
        }
    }
    fn previous_token(&mut self, i: usize) -> Option<Token> {
        if (self.i - 1) < self.tokens.len() {
            Some(self.tokens[self.i - i].clone())
        } else {
            None
        }
    }

    fn next_token(&mut self) {
        self.i += 1;
    }

    fn term(&mut self) -> R<Box<Node>, String> {
        let mut node = self.factor()?;
        while matches!(
            self.current_token().unwrap().token_type(),
            TokenType::Mul | TokenType::Div | TokenType::MulAssign | TokenType::DivAssign
        ) {
            let op = self.current_token().unwrap().clone();
            self.next_token();
            let rhs = self.factor()?;
            node = Box::new(Node::new(
                NodeValue::Operator(match op.token_type() {
                    TokenType::Mul => Operator::Mul(node, rhs),
                    TokenType::Div => Operator::Div(node, rhs),
                    TokenType::MulAssign => Operator::MulAssign(node, rhs),
                    TokenType::DivAssign => Operator::DivAssign(node, rhs),
                    _ => panic!(
                        "{}",
                        compile_error!(
                            "error",
                            op.line(),
                            op.column(),
                            &self.input_path(),
                            &self.input_content(),
                            "Unexpected token: {:?}",
                            self.current_token().unwrap()
                        )
                    ),
                }),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            ));
        }
        Ok(node)
    }

    fn expr(&mut self) -> R<Box<Node>, String> {
        let mut node = self.term()?;
        while matches!(
            self.current_token().unwrap().token_type(),
            TokenType::Add
                | TokenType::Sub
                | TokenType::AddAssign
                | TokenType::SubAssign
                | TokenType::Increment
                | TokenType::Decrement
                | TokenType::BitAnd
                | TokenType::BitOr
                | TokenType::BitXor
                | TokenType::ShiftLeft
                | TokenType::ShiftRight
        ) {
            let op = self.current_token().unwrap().clone();
            self.next_token();
            let rhs = self.term()?;
            node = Box::new(Node::new(
                NodeValue::Operator(match op.token_type() {
                    TokenType::Add => Operator::Add(node, rhs),
                    TokenType::Sub => Operator::Sub(node, rhs),
                    TokenType::AddAssign => Operator::AddAssign(node, rhs),
                    TokenType::SubAssign => Operator::SubAssign(node, rhs),
                    TokenType::Increment => Operator::Increment(node),
                    TokenType::Decrement => Operator::Decrement(node),
                    TokenType::BitAnd => Operator::BitAnd(node, rhs),
                    TokenType::BitOr => Operator::BitOr(node, rhs),
                    TokenType::BitXor => Operator::BitXor(node, rhs),
                    TokenType::ShiftLeft => Operator::ShiftLeft(node, rhs),
                    TokenType::ShiftRight => Operator::ShiftRight(node, rhs),
                    _ => panic!(
                        "{}",
                        compile_error!(
                            "error",
                            op.line(),
                            op.column(),
                            &self.input_path(),
                            &self.input_content(),
                            "Unexpected token: {:?}",
                            self.current_token().unwrap()
                        )
                    ),
                }),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            ));
        }
        Ok(node)
    }

    fn factor(&mut self) -> R<Box<Node>, String> {
        let mut token = self.current_token().unwrap().clone();
        let mut is_system = false;
        let mut node = Node::default();
        let mut is_reference = false;
        let mut is_mutable = false;
        let mut is_dereference = false;
        let mut generic_type_name: Vec<String> = vec![];

        if token.token_type() == TokenType::AtSign {
            self.next_token();
            token = self.current_token().unwrap().clone();
            is_system = true;
        }
        if token.token_type() == TokenType::BitAnd {
            self.next_token();
            token = self.current_token().unwrap().clone();
            is_reference = true;
            if token.token_value() == "mut" {
                self.next_token();
                token = self.current_token().unwrap().clone();
                is_mutable = true;
            }
        }
        if token.token_type() == TokenType::Mul {
            self.next_token();
            token = self.current_token().unwrap().clone();
            is_dereference = true;
            return Ok(self.parse_single_statement().unwrap()?);
            // panic!("{:?}",self.current_token());
        }
        match self.current_token().unwrap().token_type() {
            TokenType::MultiComment(content, (line, column)) => {
                self.next_token();
                node = Node::new(
                    NodeValue::MultiComment(content, (line, column)),
                    None,
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                );
            }
            TokenType::SingleComment(content, (line, column)) => {
                self.next_token();
                node = Node::new(
                    NodeValue::SingleComment(content, (line, column)),
                    None,
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                );
            }
            TokenType::DoubleQuote | TokenType::SingleQuote => {
                if self.peek_next_token(1).unwrap().token_type() == TokenType::Dot {
                    let ident_token = self.current_token().unwrap().clone();
                    self.next_token();
                    node = *self.parse_member_access(&ident_token)?;
                    return Ok(Box::new(node));
                }

                if let Ok(string) = token.token_value().parse::<String>() {
                    self.next_token();
                    node = Node::new(
                        NodeValue::DataType(DataType::String(string)),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    );
                } else {
                    return Err(compile_error!(
                    "error",
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                    &self.input_path(),
                    &self.input_content(),
                    "Unexpected end of input_content, no closing DoubleQuote or SingleQuote found: {:?}",
                    self.current_token().unwrap()
                ));
                }
            }
            TokenType::Number => {
                if self.peek_next_token(1).unwrap().token_type() == TokenType::Dot {
                    let ident_token = self.current_token().unwrap().clone();
                    self.next_token();
                    node = *self.parse_member_access(&ident_token)?;
                    return Ok(Box::new(node));
                }
                if let Ok(number) = token.token_value().parse::<i64>() {
                    self.next_token();
                    node = Node::new(
                        NodeValue::DataType(DataType::Int(number)),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    );
                } else if let Ok(number) = token.token_value().parse::<f64>() {
                    self.next_token();
                    node = Node::new(
                        NodeValue::DataType(DataType::Float(number)),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    );
                }
            }

            TokenType::Ident => {
                if let Ok(bool_value) = token.token_value().parse::<bool>() {
                    if self.peek_next_token(1).unwrap().token_type() == TokenType::Dot {
                        let ident_token = self.current_token().unwrap().clone();
                        self.next_token();
                        node = *self.parse_member_access(&ident_token)?;
                        return Ok(Box::new(node));
                    } else {
                        self.next_token();
                        node = Node::new(
                            NodeValue::DataType(DataType::Bool(bool_value)),
                            None,
                            self.current_token().unwrap().line(),
                            self.current_token().unwrap().column(),
                        );
                    }
                } else {
                    let ident_token = self.current_token().unwrap().clone();
                    self.next_token();
                    if self.current_token().unwrap().token_type() == TokenType::LeftCurlyBrace {
                        node = *self.parse_struct_instance(&ident_token)?;
                        return Ok(Box::new(node));
                    }

                    if self.current_token().unwrap().token_type() == TokenType::LeftParen {
                        node = *self.parse_function_call(token.clone(), is_system)?;
                        return Ok(Box::new(node));
                    }

                    if self.current_token().unwrap().token_type() == TokenType::ScopeResolution {
                        node = *self.parse_scope_resolution(&ident_token)?;
                        return Ok(Box::new(node));
                    }

                    if self.current_token().unwrap().token_type() == TokenType::Dot {
                        node = *self.parse_member_access(&ident_token)?;
                        return Ok(Box::new(node));
                    }

                    if self.current_token().unwrap().token_type() == TokenType::Lt {
                        self.next_token();
                        while self.current_token().unwrap().token_type() != TokenType::Gt {
                            generic_type_name.push(self.current_token().unwrap().token_value());
                            self.next_token();
                            if self.current_token().unwrap().token_type() == TokenType::Conma {
                                self.next_token(); // ',' をスキップ
                            }
                        }

                        //panic!("{:?}", generic_type_name);
                        self.next_token();
                        //panic!("{:?}",self.current_token());
                    }

                    let mut data_type = Parser::<'a>::new_null(
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    );

                    if self.current_token().unwrap().token_type() == TokenType::Colon {
                        self.next_token();
                        data_type = self.parse_data_type()?;
                    }
                    let _generic_type_name = if generic_type_name.is_empty() {
                        None
                    } else {
                        Some(generic_type_name)
                    };
                    node = Node::new(
                        NodeValue::Variable(
                            data_type,
                            token.token_value().clone(),
                            is_mutable,
                            is_reference,
                            _generic_type_name,
                        ),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    );
                }
            }
            TokenType::LeftParen => {
                self.next_token();
                node = *self.expr()?;
                if self.current_token().unwrap().token_type() != TokenType::RightParen {
                    return Err(compile_error!(
                        "error",
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                        &self.input_path(),
                        &self.input_content(),
                        "no closing parenthesis in factor: {:?}",
                        self.current_token().unwrap()
                    ));
                } else {
                    self.next_token();
                }
                return Ok(Box::new(node));
            }

            TokenType::LeftCurlyBrace => {
                node = *self.parse_block()?;
                return Ok(Box::new(node));
            }
            TokenType::LeftSquareBrace => {
                let data_type = Parser::<'a>::new_null(
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                );
                node = *self.parse_array(&data_type)?;
                return Ok(Box::new(node));
            }

            _ => {
                return Err(compile_error!(
                    "error",
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                    &self.input_path(),
                    &self.input_content(),
                    "Unexpected token in factor: {:?}",
                    self.current_token()
                ));
            }
        }
        Ok(Box::new(node))
    }

    fn parse_scope_resolution(&mut self, ident_token: &Token) -> R<Box<Node>, String> {
        let mut scope_resolution = vec![];

        // 最初のトークンはident_tokenをVariableとして扱う
        scope_resolution.push(Box::new(Node::new(
            NodeValue::Variable(
                Parser::<'a>::new_null(ident_token.line(), ident_token.column()),
                ident_token.token_value().clone(),
                false,
                false,
                None,
            ),
            None,
            ident_token.line(),
            ident_token.column(),
        )));

        // 二個目以降はself.exprを呼ぶ
        while self.current_token().unwrap().token_type() == TokenType::ScopeResolution {
            self.next_token(); // ::
            let scope = if self.peek_next_token(1).unwrap().token_type() == TokenType::LeftParen {
                let ident_token = self.current_token().unwrap().clone();
                self.next_token(); // ident
                self.parse_function_call(ident_token, false)?
            } else {
                self.expr()?
            };
            scope_resolution.push(scope);
        }

        Ok(Box::new(Node::new(
            NodeValue::ScopeResolution(scope_resolution),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn parse_function_call(&mut self, token: Token, is_system: bool) -> R<Box<Node>, String> {
        self.next_token(); // '(' をスキップ
        let mut args = Vec::new();
        while self.current_token().unwrap().token_type() != TokenType::RightParen {
            let arg = self.expr()?;
            args.push(*arg);
            if self.current_token().unwrap().token_type() == TokenType::Conma {
                self.next_token(); // ',' をスキップ
            }
        }
        self.next_token(); // ')' をスキップ

        if self.current_token().unwrap().token_type() == TokenType::Semi {
            self.is_statement = true;
        }

        Ok(Box::new(Node {
            value: NodeValue::Call(token.token_value().clone(), args, is_system),
            next: Rc::new(RefCell::new(None)), // nextをRc<RefCell<Option<Box<Node>>>>として初期化
            line: self.current_token().unwrap().line(),
            column: self.current_token().unwrap().column(),
            is_statement: self.is_statement,
        }))
    }

    fn parse_callback_function_definition(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // 'callback' をスキップ
        if self.current_token().unwrap().token_value() == "fn" {
            self.next_token(); // 'fn' をスキップ
            let mut is_system = false;
            if self.current_token().unwrap().token_type() == TokenType::AtSign {
                self.next_token(); // '@' をスキップ
                is_system = true;
            }

            let name = self.current_token().unwrap().token_value().clone();
            self.next_token(); // 関数名をスキップ
            self.next_token(); // '(' をスキップ
            let mut args: Vec<(Box<Node>, String)> = Vec::new();
            let mut return_type = Parser::<'a>::new_null(
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            );
            while self.current_token().unwrap().token_type() != TokenType::RightParen {
                let arg = self.expr()?;
                let mut data_type = Parser::<'a>::new_null(
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                );
                if self.current_token().unwrap().token_type() == TokenType::Colon {
                    self.next_token(); // ':' をスキップ
                    data_type = self.expr()?;
                    data_type = Box::new(Node::new(
                        NodeValue::DataType(DataType::from(data_type)),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    ));
                }
                let arg_name = match arg.value() {
                    NodeValue::Variable(_, ref name, _, _, _) => name.clone(),
                    _ => return Err("Invalid argument name".to_string()),
                };
                args.push((data_type, arg_name));
                if self.current_token().unwrap().token_type() == TokenType::Conma {
                    self.next_token(); // ',' をスキップ
                }
            }
            self.next_token(); // ')' をスキップ
            if self.current_token().unwrap().token_type() == TokenType::RightArrow {
                return_type = self.parse_return_type()?;
            }
            let body = self.parse_block()?; // ブロックの解析

            return Ok(Box::new(Node::new(
                NodeValue::Declaration(Declaration::CallBackFunction(
                    name,
                    args,
                    Box::new(*body),
                    return_type,
                    is_system,
                )),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            )));
        }
        Ok(Box::new(Node::default()))
    }

    fn parse_function_definition(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // 'fn' をスキップ
        let mut is_system = false;
        if self.current_token().unwrap().token_type() == TokenType::AtSign {
            self.next_token(); // '@' をスキップ
            is_system = true;
        }
        let name = self.current_token().unwrap().token_value().clone();
        self.next_token(); // 関数名をスキップ
        self.next_token(); // '(' をスキップ
        let mut args: Vec<(Box<Node>, String)> = Vec::new();
        let mut return_type = Parser::<'a>::new_null(
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        while self.current_token().unwrap().token_type() != TokenType::RightParen {
            let arg = self.expr()?;
            let mut data_type = Parser::<'a>::new_null(
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            );
            let arg_name = match arg.value() {
                NodeValue::Variable(_, ref name, _, _, _) => name.clone(),
                _ => return Err("Invalid argument name".to_string()),
            };
            args.push((data_type, arg_name));
            if self.current_token().unwrap().token_type() == TokenType::Conma {
                self.next_token(); // ',' をスキップ
            }

            debug!("{:?}", self.current_token());
        }
        self.next_token(); // ')' をスキップ
        if self.current_token().unwrap().token_type() == TokenType::RightArrow {
            return_type = self.parse_return_type()?;
        }

        let body = self.parse_block()?; // ブロックの解析

        Ok(Box::new(Node::new(
            NodeValue::Declaration(Declaration::Function(
                name,
                args,
                Box::new(*body),
                return_type,
                is_system,
            )),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn parse_condition(&mut self) -> R<Box<Node>, String> {
        let mut node = self.expr()?; // 基本の式を解析

        while matches!(
            self.current_token().unwrap().token_type(),
            TokenType::Eq
                | TokenType::Ne
                | TokenType::Lt
                | TokenType::Gt
                | TokenType::Le
                | TokenType::Ge
                | TokenType::And
                | TokenType::Or
        ) {
            let op = self.current_token().unwrap().clone();
            self.next_token();
            let rhs = self.expr()?; // 条件演算子の右側の式を解析

            node = Box::new(Node::new(
                match op.token_type() {
                    TokenType::Eq => NodeValue::Operator(Operator::Eq(node, rhs)),
                    TokenType::Ne => NodeValue::Operator(Operator::Ne(node, rhs)),
                    TokenType::Lt => NodeValue::Operator(Operator::Lt(node, rhs)),
                    TokenType::Gt => NodeValue::Operator(Operator::Gt(node, rhs)),
                    TokenType::Le => NodeValue::Operator(Operator::Le(node, rhs)),
                    TokenType::Ge => NodeValue::Operator(Operator::Ge(node, rhs)),
                    TokenType::And => NodeValue::Operator(Operator::And(node, rhs)),
                    TokenType::Or => NodeValue::Operator(Operator::Or(node, rhs)),

                    _ => panic!(
                        "{}",
                        compile_error!(
                            "error",
                            op.line(),
                            op.column(),
                            &self.input_path(),
                            &self.input_content(),
                            "Unexpected token: {:?}",
                            self.current_token().unwrap()
                        )
                    ),
                },
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            ));
        }
        Ok(node)
    }

    fn parse_loop_statement(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // 'loop' をスキップ
        self.next_token(); // { をスキップ

        let body = self.parse_block()?; // ブロックの解析
        Ok(Box::new(Node::new(
            NodeValue::ControlFlow(ControlFlow::Loop(body)),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn parse_break(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // break
        Ok(Box::new(Node::new(
            NodeValue::ControlFlow(ControlFlow::Break),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn parse_continue(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // continue
        Ok(Box::new(Node::new(
            NodeValue::ControlFlow(ControlFlow::Continue),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn parse_if_statement(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // 'if' をスキップ
        let mut condition = Parser::<'a>::new_null(
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        if self.current_token().unwrap().token_type() != TokenType::LeftCurlyBrace {
            condition = self.parse_condition()?;
        }
        self.next_token(); // { をスキップ
        let body = self.parse_block()?; // ブロックの解析

        let mut if_node = Node {
            value: NodeValue::ControlFlow(ControlFlow::If(Box::new(*condition), Box::new(*body))),
            next: Rc::new(RefCell::new(None)), // nextをRc<RefCell<Option<Box<Node>>>>として初期化
            line: self.current_token().unwrap().line(),
            column: self.current_token().unwrap().column(),
            is_statement: true,
        };

        // 'else' または 'else if' の処理
        if let Some(next_token) = self.peek_next_token(1) {
            if next_token.token_value() == "else" {
                self.next_token(); // 'else' をスキップ
                if let Some(next_token) = self.peek_next_token(1) {
                    if next_token.token_value() == "if" {
                        // 'else if' の処理
                        self.next_token(); // 'if' をスキップ
                        let else_if_node = self.parse_if_statement()?;
                        // Rc<RefCell<Option<Box<Node>>>>に次のノードを設定
                        *if_node.next.borrow_mut() = Some(else_if_node);
                    } else {
                        // 'else' の処理
                        self.next_token(); // { をスキップ
                        let else_body = self.parse_block()?;
                        let else_node = Node {
                            value: NodeValue::ControlFlow(ControlFlow::Else(Box::new(*else_body))),
                            next: Rc::new(RefCell::new(None)), // nextを初期化
                            line: self.current_token().unwrap().line(),
                            column: self.current_token().unwrap().column(),
                            is_statement: true,
                        };
                        // Rc<RefCell<Option<Box<Node>>>>に次のノードを設定
                        *if_node.next.borrow_mut() = Some(Box::new(else_node));
                    }
                }
            }
        }
        Ok(Box::new(if_node))
    }

    fn parse_for_statement(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // for
        let var = self.current_token().unwrap().token_value().clone();
        self.next_token(); // var
        self.next_token(); // in

        let start_token = self.current_token().unwrap().token_value().clone();
        let iterator_node = if self.peek_next_token(1).unwrap().token_type() == TokenType::Range {
            self.next_token(); // skip ..
            self.next_token();
            let end_token = self.current_token().unwrap().token_value().clone();
            Box::new(Node::new(
                NodeValue::Operator(Operator::Range(
                    Box::new(Node::new(
                        NodeValue::DataType(DataType::Int(start_token.parse().unwrap())),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    )),
                    Box::new(Node::new(
                        NodeValue::DataType(DataType::Int(end_token.parse().unwrap())),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    )),
                )),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            ))
        } else {
            Box::new(Node::new(
                NodeValue::Variable(
                    Parser::<'a>::new_null(
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    ),
                    start_token,
                    false,
                    false,
                    None,
                ),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            ))
        };
        self.next_token(); // { をスキップ
        let body = self.parse_block()?;
        Ok(Box::new(Node::new(
            NodeValue::ControlFlow(ControlFlow::For(
                Box::new(Node::new(
                    NodeValue::Variable(
                        Parser::<'a>::new_null(
                            self.current_token().unwrap().line(),
                            self.current_token().unwrap().column(),
                        ),
                        var,
                        false,
                        false,
                        None,
                    ),
                    None,
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                )),
                iterator_node,
                body,
            )),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn parse_return_type(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // '->' をスキップ
        let return_type = self.expr()?;
        Ok(Box::new(Node::new(
            NodeValue::DataType(DataType::from(return_type)),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn parse_while_statement(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // 'while' をスキップ
        let mut condition = Parser::<'a>::new_null(
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        if self.current_token().unwrap().token_type() != TokenType::LeftCurlyBrace {
            condition = self.parse_condition()?;
        }
        self.next_token(); // { をスキップ
        let body = self.parse_block()?; // ブロックの解析
        Ok(Box::new(Node::new(
            NodeValue::ControlFlow(ControlFlow::While(Box::new(*condition), Box::new(*body))),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn parse_block(&mut self) -> R<Box<Node>, String> {
        if self.current_token().unwrap().token_type() == TokenType::LeftCurlyBrace {
            self.next_token(); // '{' をスキップ
        }
        let mut nodes = Vec::new();
        while self.current_token().unwrap().token_type() != TokenType::RightCurlyBrace {
            if self.current_token().unwrap().token_type() == TokenType::Eof {
                return Err(compile_error!(
                    "error",
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                    &self.input_path(),
                    &self.input_content(),
                    "Unexpected end of input, no closing curly brace found: {:?}",
                    self.current_token().unwrap()
                ));
            }
            let statements = self.parse_statement()?;
            nodes.push(statements);
        }
        if self.current_token().unwrap().token_type() != TokenType::RightCurlyBrace {
            return Err(compile_error!(
                "error",
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
                &self.input_path(),
                &self.input_content(),
                "no closing curly brace in block: {:?}",
                self.current_token().unwrap()
            ));
        } else {
            self.next_token(); // '}' をスキップ
            Ok(Box::new(Node::new(
                NodeValue::Block(nodes),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            )))
        }
    }

    fn parse_data_type(&mut self) -> R<Box<Node>, String> {
        if self.peek_next_token(1).unwrap().token_type() != TokenType::Eof
            && self.peek_next_token(1).unwrap().token_type() != TokenType::Conma
            && self.peek_next_token(1).unwrap().token_type() != TokenType::RightCurlyBrace
        {
            self.next_token(); // 変数名 をスキップ
        }

        let data_type = self.expr()?;
        Ok(Box::new(Node::new(
            NodeValue::DataType(DataType::from(data_type)),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn parse_type_declaration(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // type
        let _type_name = self.current_token().unwrap().token_value().clone();
        self.next_token(); // name
        self.next_token(); // =
        let value_node = self.expr()?;

        Ok(Box::new(Node {
            value: NodeValue::Declaration(Declaration::Type(
                Box::new(Node::new(
                    NodeValue::Variable(
                        Parser::<'a>::new_null(
                            self.current_token().unwrap().line(),
                            self.current_token().unwrap().column(),
                        ),
                        _type_name,
                        false,
                        false,
                        None,
                    ),
                    None,
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                )),
                value_node,
            )),
            next: Rc::new(RefCell::new(None)), // nextをRc<RefCell<Option<Box<Node>>>>で初期化
            line: self.current_token().unwrap().line(),
            column: self.current_token().unwrap().column(),
            is_statement: self.is_statement,
        }))
    }
    fn parse_variable_declaration(&mut self) -> R<Box<Node>, String> {
        self.next_token();
        let mut is_mutable = false;
        if let Some(token) = self.current_token() {
            if token.token_value() == "mut" || token.token_value() == "mutable" {
                self.next_token();
                is_mutable = true;
            }
        }
        let var = self.current_token().unwrap().token_value().clone();
        let mut data_type = Box::new(Node::new(
            NodeValue::DataType(DataType::from(Parser::<'a>::new_null(
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            ))),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        ));
        let mut value_node = Parser::<'a>::new_null(
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        if self.peek_next_token(1).unwrap().token_type() == TokenType::Colon {
            self.next_token();
            data_type = self.parse_data_type()?;
        }
        if self.current_token().unwrap().token_type() == TokenType::Semi {
            self.is_statement = true;
            let mut is_local = false;
            let mut brace_count = 0;
            for i in (0..self.i).rev() {
                match self.tokens[i].token_type() {
                    TokenType::LeftCurlyBrace => brace_count += 1,
                    TokenType::RightCurlyBrace => brace_count -= 1,
                    _ => {}
                }
                if brace_count > 0 {
                    is_local = true;
                    break;
                }
            }

            return Ok(Box::new(Node {
                value: NodeValue::Declaration(Declaration::Variable(
                    Box::new(Node::new(
                        NodeValue::Variable(
                            Parser::<'a>::new_null(
                                self.current_token().unwrap().line(),
                                self.current_token().unwrap().column(),
                            ),
                            var,
                            false,
                            false,
                            None,
                        ),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    )),
                    data_type,
                    value_node,
                    is_local,
                    is_mutable,
                )),
                next: Rc::new(RefCell::new(None)), // nextをRc<RefCell<Option<Box<Node>>>>で初期化
                line: self.current_token().unwrap().line(),
                column: self.current_token().unwrap().column(),
                is_statement: self.is_statement,
            }));
        }
        self.next_token();
        if self.current_token().unwrap().token_type() == TokenType::Equals {
            self.next_token();
        }
        if self.current_token().unwrap().token_type() == TokenType::Semi {
            self.is_statement = true;
            let mut is_local = false;
            let mut brace_count = 0;
            for i in (0..self.i).rev() {
                match self.tokens[i].token_type() {
                    TokenType::LeftCurlyBrace => brace_count += 1,
                    TokenType::RightCurlyBrace => brace_count -= 1,
                    _ => {}
                }
                if brace_count > 0 {
                    is_local = true;
                    break;
                }
            }

            return Ok(Box::new(Node {
                value: NodeValue::Declaration(Declaration::Variable(
                    Box::new(Node::new(
                        NodeValue::Variable(
                            Parser::<'a>::new_null(
                                self.current_token().unwrap().line(),
                                self.current_token().unwrap().column(),
                            ),
                            var,
                            false,
                            false,
                            None,
                        ),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    )),
                    data_type,
                    value_node,
                    is_local,
                    is_mutable,
                )),
                next: Rc::new(RefCell::new(None)), // nextをRc<RefCell<Option<Box<Node>>>>で初期化
                line: self.current_token().unwrap().line(),
                column: self.current_token().unwrap().column(),
                is_statement: self.is_statement,
            }));
        }
        value_node = self.expr()?;
        if self.current_token().unwrap().token_type() == TokenType::Semi {
            self.is_statement = true;
        }
        let mut is_local = false;
        let mut brace_count = 0;
        for i in (0..self.i).rev() {
            match self.tokens[i].token_type() {
                TokenType::LeftCurlyBrace => brace_count += 1,
                TokenType::RightCurlyBrace => brace_count -= 1,
                _ => {}
            }
            if brace_count > 0 {
                is_local = true;
                break;
            }
        }

        return Ok(Box::new(Node {
            value: NodeValue::Declaration(Declaration::Variable(
                Box::new(Node::new(
                    NodeValue::Variable(
                        Parser::<'a>::new_null(
                            self.current_token().unwrap().line(),
                            self.current_token().unwrap().column(),
                        ),
                        var,
                        false,
                        false,
                        None,
                    ),
                    None,
                    self.current_token().unwrap().line(),
                    self.current_token().unwrap().column(),
                )),
                data_type,
                value_node,
                is_local,
                is_mutable,
            )),
            next: Rc::new(RefCell::new(None)), // nextをRc<RefCell<Option<Box<Node>>>>で初期化
            line: self.current_token().unwrap().line(),
            column: self.current_token().unwrap().column(),
            is_statement: self.is_statement,
        }));
    }

    fn parse_array(&mut self, data_type: &Box<Node>) -> R<Box<Node>, String> {
        self.next_token(); // [ をスキップ
        let mut value_vec = vec![];
        while self.current_token().unwrap().token_type() != TokenType::RightSquareBrace {
            value_vec.push(self.expr()?);
            if self.current_token().unwrap().token_type() == TokenType::Conma {
                self.next_token(); // ',' をスキップ
            }
        }
        self.next_token(); // ] をスキップ
        Ok(Box::new(Node::new(
            NodeValue::DataType(DataType::Array(data_type.clone(), value_vec)),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }

    fn parse_assign_variable(&mut self) -> R<Box<Node>, String> {
        let var = self.current_token().unwrap().token_value().clone();
        let data_type = Parser::<'a>::new_null(
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        let mut value_node = Parser::<'a>::new_null(
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        let mut index = Parser::<'a>::new_null(
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );

        self.next_token(); // var
        if self.current_token().unwrap().token_type() == TokenType::LeftSquareBrace {
            self.next_token(); // [
            index = self.expr()?;

            self.next_token(); // ]
            self.next_token(); // =

            value_node = self.expr()?;

            Ok(Box::new(Node {
                value: NodeValue::Assign(
                    Box::new(Node::new(
                        NodeValue::Variable(
                            Parser::<'a>::new_null(
                                self.current_token().unwrap().line(),
                                self.current_token().unwrap().column(),
                            ),
                            var,
                            false,
                            false,
                            None,
                        ),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    )),
                    value_node,
                    index,
                ),
                next: Rc::new(RefCell::new(None)), // nextをRc<RefCell<Option<Box<Node>>>>で初期化
                line: self.current_token().unwrap().line(),
                column: self.current_token().unwrap().column(),
                is_statement: self.is_statement,
            }))
        } else {
            self.next_token(); // =

            value_node = self.expr()?;
            if self.current_token().unwrap().token_type() == TokenType::Semi {
                self.is_statement = true;
            }

            Ok(Box::new(Node {
                value: NodeValue::Assign(
                    Box::new(Node::new(
                        NodeValue::Variable(
                            Parser::<'a>::new_null(
                                self.current_token().unwrap().line(),
                                self.current_token().unwrap().column(),
                            ),
                            var,
                            false,
                            false,
                            None,
                        ),
                        None,
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    )),
                    value_node,
                    index,
                ),
                next: Rc::new(RefCell::new(None)), // nextをRc<RefCell<Option<Box<Node>>>>で初期化
                line: self.current_token().unwrap().line(),
                column: self.current_token().unwrap().column(),
                is_statement: self.is_statement,
            }))
        }
    }

    fn parse_return(&mut self) -> R<Box<Node>, String> {
        self.next_token();
        let mut ret_value = Box::new(Node::default());
        ret_value = self.expr()?;

        Ok(Box::new(Node {
            value: NodeValue::ControlFlow(ControlFlow::Return(ret_value)),
            next: Rc::new(RefCell::new(None)), // nextをRc<RefCell<Option<Box<Node>>>>で初期化
            line: self.current_token().unwrap().line(),
            column: self.current_token().unwrap().column(),
            is_statement: self.is_statement,
        }))
    }

    fn parse_include(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // include
        let include_file_path = self.current_token().unwrap().token_value().clone();
        let include_node = Node::new(
            NodeValue::Include(include_file_path),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        );
        self.next_token();

        Ok(Box::new(include_node))
    }
    fn parse_impl_definition(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // impl
        let var = self.current_token().unwrap().token_value().clone();
        let mut member: Vec<Box<Node>> = Vec::new();

        self.next_token(); // var
        if self.current_token().unwrap().token_type() == TokenType::LeftCurlyBrace {
            self.next_token(); // {
            while self.current_token().unwrap().token_type() != TokenType::RightCurlyBrace {
                let member_value = self.parse_single_statement().unwrap()?;
                member.push(member_value);
            }
            self.next_token(); // }
            Ok(Box::new(Node::new(
                NodeValue::Declaration(Declaration::Impl(var.clone(), member.clone())),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            )))
        } else {
            Err(String::from(""))
        }
    }
    fn parse_struct_instance(&mut self, ident_token: &Token) -> R<Box<Node>, String> {
        let struct_name = ident_token.token_value().clone();
        let mut field_value = vec![];
        self.next_token(); // {
        while self.current_token().unwrap().token_type() != TokenType::Eof
            && self.current_token().unwrap().token_type() != TokenType::RightCurlyBrace
            && self.current_token().unwrap().token_type() != TokenType::Semi
        {
            if self.current_token().unwrap().token_type() == TokenType::Conma {
                self.next_token();
                continue;
            }
            if self.current_token().unwrap().token_type() == TokenType::Ident {
                //panic!("{:?}", self.current_token());
                let name = self.current_token().unwrap().token_value();
                self.next_token();
                if self.current_token().unwrap().token_type() == TokenType::Colon {
                    self.next_token(); // :
                    let value = self.expr()?;
                    //panic!("{:?}", self.current_token());
                    field_value.push((name.clone(), value));
                }
            }
            self.next_token();
        }

        //panic!("{:?} {:?}", self.current_token(), field_value);
        Ok(Box::new(Node::new(
            NodeValue::StructInstance(struct_name, field_value),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        )))
    }
    fn parse_struct_definition(&mut self) -> R<Box<Node>, String> {
        self.next_token(); // struct
        let var = self.current_token().unwrap().token_value().clone();
        let mut member: Vec<Box<Node>> = Vec::new();

        self.next_token(); // var
        if self.current_token().unwrap().token_type() == TokenType::LeftCurlyBrace {
            self.next_token(); // {
            while self.current_token().unwrap().token_type() != TokenType::RightCurlyBrace {
                let member_value = self.expr()?;
                member.push(member_value);
                if self.current_token().unwrap().token_type() == TokenType::Conma {
                    self.next_token(); // ',' をスキップ
                }
            }
            self.next_token(); // }
            Ok(Box::new(Node::new(
                NodeValue::Declaration(Declaration::Struct(var.clone(), member.clone())),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            )))
        } else {
            if self.current_token().unwrap().token_type() == TokenType::Semi {
                self.is_statement = true;
            }
            Ok(Box::new(Node::new(
                NodeValue::Declaration(Declaration::Struct(
                    var.clone(),
                    vec![Parser::<'a>::new_null(
                        self.current_token().unwrap().line(),
                        self.current_token().unwrap().column(),
                    )],
                )),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            )))
        }
    }

    fn parse_member_access(&mut self, ident_token: &Token) -> R<Box<Node>, String> {
        let member = if let Ok(v) = ident_token.token_value().parse::<bool>() {
            Box::new(Node::new(
                NodeValue::DataType(DataType::String(ident_token.token_value())),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            ))
        } else if let Ok(number) = ident_token.token_value().parse::<i64>() {
            Box::new(Node::new(
                NodeValue::DataType(DataType::Int(number)),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            ))
        } else if let Ok(number) = ident_token.token_value().parse::<f64>() {
            Box::new(Node::new(
                NodeValue::DataType(DataType::Float(number)),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            ))
        } else if ident_token.token_type() == TokenType::DoubleQuote
            || ident_token.token_type() == TokenType::SingleQuote
        {
            Box::new(Node::new(
                NodeValue::DataType(DataType::String(ident_token.token_value())),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            ))
        } else {
            Box::new(Node::new(
                NodeValue::Variable(
                    Box::new(Node::default()),
                    ident_token.token_value(),
                    false,
                    false,
                    None,
                ),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            ))
        };
        self.next_token();
        let item = self.expr()?;
        let mut node = Box::new(Node::new(
            NodeValue::MemberAccess(member.clone(), item),
            None,
            self.current_token().unwrap().line(),
            self.current_token().unwrap().column(),
        ));

        while self.current_token().unwrap().token_type() == TokenType::Dot {
            self.next_token();
            let next_item = self.expr()?;
            node = Box::new(Node::new(
                NodeValue::MemberAccess(node.clone(), next_item),
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            ));
        }

        Ok(node)
    }
    fn parse_primitive_type(&mut self, token: &Token) -> R<Box<Node>, String> {
        Ok(Box::new(Node::default()))
    }
    pub fn parse_single_statement(&mut self) -> Option<R<Box<Node>, String>> {
        let result = if self.current_token().unwrap().token_value() == "callback" {
            self.parse_callback_function_definition()
        } else if self.current_token().unwrap().token_value() == "struct" {
            self.parse_struct_definition()
        } else if self.current_token().unwrap().token_value() == "impl" {
            self.parse_impl_definition()
        } else if self.current_token().unwrap().token_value() == "fn" {
            self.parse_function_definition()
        } else if self.current_token().unwrap().token_value() == "while" {
            self.parse_while_statement()
        } else if self.current_token().unwrap().token_value() == "if" {
            self.parse_if_statement()
        } else if self.current_token().unwrap().token_type() == TokenType::Ident
            && self.current_token().unwrap().token_value() == "for"
            && self.peek_next_token(2).unwrap().token_value() == "in"
        {
            self.parse_for_statement()
        } else if self.current_token().unwrap().token_value() == "loop" {
            self.parse_loop_statement()
        } else if self.current_token().unwrap().token_type() == TokenType::Ident
            && (self.current_token().unwrap().token_value() == "let"
                || self.current_token().unwrap().token_value() == "var"
                || self.current_token().unwrap().token_value() == "l"
                || self.current_token().unwrap().token_value() == "v")
        {
            self.parse_variable_declaration()
        } else if self.current_token().unwrap().token_type() == TokenType::Ident
            && self.peek_next_token(2).unwrap().token_type() == TokenType::Equals
            && self.current_token().unwrap().token_value() == "type"
        {
            self.parse_type_declaration()
        } else if self.current_token().unwrap().token_type() == TokenType::Ident
            && self.peek_next_token(1).unwrap().token_type() == TokenType::Equals
            || self.current_token().unwrap().token_type() == TokenType::Ident
                && self.peek_next_token(1).unwrap().token_type() == TokenType::LeftSquareBrace
        {
            self.parse_assign_variable()
        } else if self.current_token().unwrap().token_type() == TokenType::Ident
            && self.current_token().unwrap().token_value() == "return"
        {
            self.parse_return()
        } else if self.current_token().unwrap().token_type() == TokenType::Ident
            && self.current_token().unwrap().token_value() == "break"
        {
            self.parse_break()
        } else if self.current_token().unwrap().token_type() == TokenType::Ident
            && self.current_token().unwrap().token_value() == "continue"
        {
            self.parse_continue()
        } else if self.current_token().unwrap().token_type() == TokenType::Ident
            && self.current_token().unwrap().token_value() == "include"
        {
            self.parse_include()
        } else if self.current_token().unwrap().token_type() == TokenType::LeftCurlyBrace {
            self.parse_block()
        } else if self.current_token().unwrap().token_type() == TokenType::Semi {
            self.is_statement = true;
            self.next_token();
            return Some(Ok(Box::new(Node::new(
                NodeValue::EndStatement,
                None,
                self.current_token().unwrap().line(),
                self.current_token().unwrap().column(),
            )))); // ステートメントを終了
        } else {
            self.is_statement = false;
            self.expr()
        };

        Some(result)
    }
    /*
    fn parse_statement(&mut self) -> R<Box<Node>, String> {
        self.parse_statement_recursive(None)
    }

    fn parse_statement_recursive(
        &mut self,
        previous_node: Option<Box<Node>>,
    ) -> R<Box<Node>, String> {
        if self.current_token().unwrap().token_type() == TokenType::Eof
            || self.current_token().unwrap().token_type() == TokenType::RightCurlyBrace
        {
            return previous_node.ok_or("No statements found".to_string());
        }

        if let Some(statement) = self.parse_single_statement() {
            let node = statement?;
            if let Some(mut prev) = previous_node {
                // ここで新しいノードを追加する
                let mut current = prev.as_mut();
                while current.next.is_some() {
                    current = current.next.as_mut().unwrap();
                }
                current.set_next(Some(node.clone()));
                self.parse_statement_recursive(Some(prev))
            } else {
                self.parse_statement_recursive(Some(node))
            }
        } else {
            Err("Failed to parse statement".to_string())
        }
    }
    pub fn parse(&mut self) -> R<Box<Node>, String> {
        self.parse_statement()
    };*/

    fn parse_statement_recursive(
        &mut self,
        previous_node: Option<Box<Node>>,
    ) -> R<Box<Node>, String> {
        if self.current_token().unwrap().token_type() == TokenType::Eof
            || self.current_token().unwrap().token_type() == TokenType::RightCurlyBrace
        {
            return previous_node.ok_or("No statements found".to_string());
        }

        if let Some(statement) = self.parse_single_statement() {
            let node = statement?;
            if let Some(prev) = previous_node {
                // 新しいノードを追加する
                let mut current_ref = prev.clone(); // クローンを作成

                // 最後のノードまで進む
                loop {
                    let next_node = current_ref.next.borrow().clone(); // 次のノードを取得
                    match next_node {
                        Some(ref next) => {
                            current_ref = next.clone(); // 現在のノードを更新
                        }
                        None => break, // 次のノードがない場合はループを終了
                    }
                }

                // 新しいノードを設定
                current_ref.set_next(Rc::new(RefCell::new(Some(node.clone()))));
                self.parse_statement_recursive(Some(prev)) // prevを再利用
            } else {
                self.parse_statement_recursive(Some(node))
            }
        } else {
            Err("Failed to parse statement".to_string())
        }
    }

    fn parse_statement(&mut self) -> R<Box<Node>, String> {
        self.parse_statement_recursive(None)
    }
    pub fn parse(&mut self) -> R<Box<Node>, String> {
        self.parse_statement()
    }
}
