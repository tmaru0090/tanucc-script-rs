use crate::parser::syntax::Node;
use crate::types::*;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::fmt;
use std::mem;
use std::ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Not, Shl, Shr, Sub};
use std::rc::Rc;

pub trait Size {
    fn size(&self) -> usize;
}

impl Size for SystemValue {
    fn size(&self) -> usize {
        match self {
            SystemValue::Null => 0,
            SystemValue::Bool(_) => mem::size_of::<bool>(),
            SystemValue::U8(_) => mem::size_of::<u8>(),
            SystemValue::U16(_) => mem::size_of::<u16>(),
            SystemValue::U32(_) => mem::size_of::<u32>(),
            SystemValue::U64(_) => mem::size_of::<u64>(),
            SystemValue::Usize(_) => mem::size_of::<usize>(),
            SystemValue::I8(_) => mem::size_of::<i8>(),
            SystemValue::I16(_) => mem::size_of::<i16>(),
            SystemValue::I32(_) => mem::size_of::<i32>(),
            SystemValue::I64(_) => mem::size_of::<i64>(),
            SystemValue::F32(_) => mem::size_of::<f32>(),
            SystemValue::F64(_) => mem::size_of::<f64>(),
            SystemValue::String(s) => mem::size_of::<String>() + s.len(),
            SystemValue::Array(arr) => {
                mem::size_of::<Vec<SystemValue>>() + arr.iter().map(|v| v.size()).sum::<usize>()
            }
            SystemValue::Pointer(_) => mem::size_of::<Box<SystemValue>>(),
            SystemValue::Tuple(tuple) => match &**tuple {
                SystemValueTuple::Tuple1(v1) => v1.size(),
                SystemValueTuple::Tuple2(v1, v2) => v1.size() + v2.size(),
                SystemValueTuple::Tuple3(v1, v2, v3) => v1.size() + v2.size() + v3.size(),
                SystemValueTuple::Tuple4(v1, v2, v3, v4) => {
                    v1.size() + v2.size() + v3.size() + v4.size()
                }
                SystemValueTuple::Tuple5(v1, v2, v3, v4, v5) => {
                    v1.size() + v2.size() + v3.size() + v4.size() + v5.size()
                }
                SystemValueTuple::Tuple6(v1, v2, v3, v4, v5, v6) => {
                    v1.size() + v2.size() + v3.size() + v4.size() + v5.size() + v6.size()
                }
                SystemValueTuple::Tuple7(v1, v2, v3, v4, v5, v6, v7) => {
                    v1.size()
                        + v2.size()
                        + v3.size()
                        + v4.size()
                        + v5.size()
                        + v6.size()
                        + v7.size()
                }
                SystemValueTuple::Tuple8(v1, v2, v3, v4, v5, v6, v7, v8) => {
                    v1.size()
                        + v2.size()
                        + v3.size()
                        + v4.size()
                        + v5.size()
                        + v6.size()
                        + v7.size()
                        + v8.size()
                }
                SystemValueTuple::Tuple9(v1, v2, v3, v4, v5, v6, v7, v8, v9) => {
                    v1.size()
                        + v2.size()
                        + v3.size()
                        + v4.size()
                        + v5.size()
                        + v6.size()
                        + v7.size()
                        + v8.size()
                        + v9.size()
                }
                SystemValueTuple::Tuple10(v1, v2, v3, v4, v5, v6, v7, v8, v9, v10) => {
                    v1.size()
                        + v2.size()
                        + v3.size()
                        + v4.size()
                        + v5.size()
                        + v6.size()
                        + v7.size()
                        + v8.size()
                        + v9.size()
                        + v10.size()
                }
                SystemValueTuple::Tuple11(v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11) => {
                    v1.size()
                        + v2.size()
                        + v3.size()
                        + v4.size()
                        + v5.size()
                        + v6.size()
                        + v7.size()
                        + v8.size()
                        + v9.size()
                        + v10.size()
                        + v11.size()
                }
                SystemValueTuple::Tuple12(v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11, v12) => {
                    v1.size()
                        + v2.size()
                        + v3.size()
                        + v4.size()
                        + v5.size()
                        + v6.size()
                        + v7.size()
                        + v8.size()
                        + v9.size()
                        + v10.size()
                        + v11.size()
                        + v12.size()
                }
            },
            SystemValue::Struct(fields) => fields.iter().map(|v| v.size()).sum(),
            SystemValue::System(_) => 0,
            SystemValue::__NodeBlock(nodes) => 0,
        }
    }
}
#[cfg(any(feature = "full", feature = "decoder"))]
impl From<Rc<RefCell<Node>>> for SystemValue {
    fn from(node: Rc<RefCell<Node>>) -> Self {
        let node = node.borrow(); // 借用して中身を取得

        match &*node {
            // 借用したノードを参照
            Node {
                value: NodeValue::ControlFlow(ControlFlow::Return(ref return_node)),
                ..
            } => return_node.clone().into(),
            Node {
                value: NodeValue::Variable(_, ref name, _, _, _),
                ..
            } => SystemValue::String(name.clone()),
            Node {
                value: NodeValue::DataType(DataType::String(ref value)),
                ..
            } => SystemValue::String(value.clone()),
            Node {
                value: NodeValue::DataType(DataType::Int(ref value)),
                ..
            } => {
                let int_value = *value;
                if int_value >= 0 {
                    // 符号なしに変換
                    if int_value <= u8::MAX as i64 {
                        SystemValue::U8(int_value as u8)
                    } else if int_value <= u16::MAX as i64 {
                        SystemValue::U16(int_value as u16)
                    } else if int_value <= u32::MAX as i64 {
                        SystemValue::U32(int_value as u32)
                    } else if int_value <= usize::MAX as i64 {
                        SystemValue::Usize(int_value as usize)
                    } else {
                        SystemValue::I64(int_value)
                    }
                } else {
                    // 符号ありに変換
                    if int_value >= i8::MIN as i64 && int_value <= i8::MAX as i64 {
                        SystemValue::I8(int_value as i8)
                    } else if int_value >= i16::MIN as i64 && int_value <= i16::MAX as i64 {
                        SystemValue::I16(int_value as i16)
                    } else if int_value >= i32::MIN as i64 && int_value <= i32::MAX as i64 {
                        SystemValue::I32(int_value as i32)
                    } else {
                        SystemValue::I64(int_value)
                    }
                }
            }
            Node {
                value: NodeValue::DataType(DataType::Float(ref value)),
                ..
            } => {
                let float_value = *value;
                if float_value >= f32::MIN as f64 && float_value <= f32::MAX as f64 {
                    SystemValue::F32(float_value as f32)
                } else {
                    SystemValue::F64(float_value)
                }
            }
            Node {
                value: NodeValue::DataType(DataType::Bool(ref value)),
                ..
            } => SystemValue::Bool(value.clone()),
            Node {
                value: NodeValue::DataType(DataType::Array(ref data_type, ref value)),
                ..
            } => {
                let mut vec: Vec<SystemValue> = vec![];
                for v in value {
                    let array_value = match **v {
                        Node {
                            value: NodeValue::Variable(_, ref name, _, _, _),
                            ..
                        } => SystemValue::String(name.clone()),
                        Node {
                            value: NodeValue::DataType(DataType::String(ref value)),
                            ..
                        } => SystemValue::String(value.clone()),
                        Node {
                            value: NodeValue::DataType(DataType::Int(ref value)),
                            ..
                        } => {
                            let int_value = *value;
                            if int_value >= 0 {
                                if int_value <= u8::MAX as i64 {
                                    SystemValue::U8(int_value as u8)
                                } else if int_value <= u16::MAX as i64 {
                                    SystemValue::U16(int_value as u16)
                                } else if int_value <= u32::MAX as i64 {
                                    SystemValue::U32(int_value as u32)
                                } else if int_value <= usize::MAX as i64 {
                                    SystemValue::Usize(int_value as usize)
                                } else {
                                    SystemValue::I64(int_value)
                                }
                            } else {
                                if int_value >= i8::MIN as i64 && int_value <= i8::MAX as i64 {
                                    SystemValue::I8(int_value as i8)
                                } else if int_value >= i16::MIN as i64
                                    && int_value <= i16::MAX as i64
                                {
                                    SystemValue::I16(int_value as i16)
                                } else if int_value >= i32::MIN as i64
                                    && int_value <= i32::MAX as i64
                                {
                                    SystemValue::I32(int_value as i32)
                                } else {
                                    SystemValue::I64(int_value)
                                }
                            }
                        }
                        Node {
                            value: NodeValue::DataType(DataType::Float(ref value)),
                            ..
                        } => {
                            let float_value = *value;
                            if float_value >= f32::MIN as f64 && float_value <= f32::MAX as f64 {
                                SystemValue::F32(float_value as f32)
                            } else {
                                SystemValue::F64(float_value)
                            }
                        }
                        Node {
                            value: NodeValue::DataType(DataType::Bool(ref value)),
                            ..
                        } => SystemValue::Bool(value.clone()),
                        _ => SystemValue::Null,
                    };
                    vec.push(SystemValue::Pointer(Box::new(array_value)));
                }
                SystemValue::Array(vec)
            }
            _ => SystemValue::Null,
        }
    }
}
#[cfg(any(feature = "full", feature = "decoder"))]
impl From<Box<Node>> for SystemValue {
    fn from(node: Box<Node>) -> Self {
        match *node {
            Node {
                value: NodeValue::ControlFlow(ControlFlow::Return(ref return_node)),
                ..
            } => return_node.clone().into(),
            Node {
                value: NodeValue::Variable(_, ref name, _, _, _),
                ..
            } => SystemValue::String(name.clone()),
            Node {
                value: NodeValue::DataType(DataType::String(ref value)),
                ..
            } => SystemValue::String(value.clone()),

            Node {
                value: NodeValue::DataType(DataType::Int(ref value)),
                ..
            } => {
                let int_value = *value;
                if int_value >= 0 {
                    // 符号なしに変換
                    if int_value <= u8::MAX as i64 {
                        SystemValue::U8(int_value as u8)
                    } else if int_value <= u16::MAX as i64 {
                        SystemValue::U16(int_value as u16)
                    } else if int_value <= u32::MAX as i64 {
                        SystemValue::U32(int_value as u32)
                    } else if int_value <= usize::MAX as i64 {
                        SystemValue::Usize(int_value as usize)
                    } else {
                        SystemValue::I64(int_value)
                    }
                } else {
                    // 符号ありに変換
                    if int_value >= i8::MIN as i64 && int_value <= i8::MAX as i64 {
                        SystemValue::I8(int_value as i8)
                    } else if int_value >= i16::MIN as i64 && int_value <= i16::MAX as i64 {
                        SystemValue::I16(int_value as i16)
                    } else if int_value >= i32::MIN as i64 && int_value <= i32::MAX as i64 {
                        SystemValue::I32(int_value as i32)
                    } else {
                        SystemValue::I64(int_value)
                    }
                }
            }

            Node {
                value: NodeValue::DataType(DataType::Float(ref value)),
                ..
            } => {
                let float_value = *value;
                if float_value >= f32::MIN as f64 && float_value <= f32::MAX as f64 {
                    SystemValue::F32(float_value as f32)
                } else {
                    SystemValue::F64(float_value)
                }
            }
            Node {
                value: NodeValue::DataType(DataType::Bool(ref value)),
                ..
            } => SystemValue::Bool(value.clone()),

            Node {
                value: NodeValue::DataType(DataType::Array(ref data_type, ref value)),
                ..
            } => {
                let mut vec: Vec<SystemValue> = vec![];
                for v in value {
                    let array_value = match **v {
                        Node {
                            value: NodeValue::Variable(_, ref name, _, _, _),
                            ..
                        } => SystemValue::String(name.clone()),
                        Node {
                            value: NodeValue::DataType(DataType::String(ref value)),
                            ..
                        } => SystemValue::String(value.clone()),

                        Node {
                            value: NodeValue::DataType(DataType::Int(ref value)),
                            ..
                        } => {
                            let int_value = *value;
                            if int_value >= 0 {
                                // 符号なしに変換
                                if int_value <= u8::MAX as i64 {
                                    SystemValue::U8(int_value as u8)
                                } else if int_value <= u16::MAX as i64 {
                                    SystemValue::U16(int_value as u16)
                                } else if int_value <= u32::MAX as i64 {
                                    SystemValue::U32(int_value as u32)
                                } else if int_value <= usize::MAX as i64 {
                                    SystemValue::Usize(int_value as usize)
                                } else {
                                    SystemValue::I64(int_value)
                                }
                            } else {
                                // 符号ありに変換
                                if int_value >= i8::MIN as i64 && int_value <= i8::MAX as i64 {
                                    SystemValue::I8(int_value as i8)
                                } else if int_value >= i16::MIN as i64
                                    && int_value <= i16::MAX as i64
                                {
                                    SystemValue::I16(int_value as i16)
                                } else if int_value >= i32::MIN as i64
                                    && int_value <= i32::MAX as i64
                                {
                                    SystemValue::I32(int_value as i32)
                                } else {
                                    SystemValue::I64(int_value)
                                }
                            }
                        }

                        Node {
                            value: NodeValue::DataType(DataType::Float(ref value)),
                            ..
                        } => {
                            let float_value = *value;
                            if float_value >= f32::MIN as f64 && float_value <= f32::MAX as f64 {
                                SystemValue::F32(float_value as f32)
                            } else {
                                SystemValue::F64(float_value)
                            }
                        }

                        Node {
                            value: NodeValue::DataType(DataType::Bool(ref value)),
                            ..
                        } => SystemValue::Bool(value.clone()),

                        _ => SystemValue::Null,
                    };
                    vec.push(SystemValue::Pointer(Box::new(array_value)));
                }
                SystemValue::Array(vec)
            }

            _ => SystemValue::Null,
        }
    }
}

#[cfg(any(feature = "full", feature = "decoder"))]
impl From<Node> for SystemValue {
    fn from(node: Node) -> Self {
        match node {
            Node {
                value: NodeValue::ControlFlow(ControlFlow::Return(return_node)),
                ..
            } => {
                return_node.into() // Box<Node>からNodeに変更
            }

            Node {
                value: NodeValue::Variable(_, ref name, _, _, _),
                ..
            } => SystemValue::String(name.clone()),

            Node {
                value: NodeValue::DataType(DataType::String(ref value)),
                ..
            } => SystemValue::String(value.clone()),

            Node {
                value: NodeValue::DataType(DataType::Int(ref value)),
                ..
            } => {
                let int_value = *value;
                if int_value >= 0 {
                    // 符号なしに変換
                    if int_value <= u8::MAX as i64 {
                        SystemValue::U8(int_value as u8)
                    } else if int_value <= u16::MAX as i64 {
                        SystemValue::U16(int_value as u16)
                    } else if int_value <= u32::MAX as i64 {
                        SystemValue::U32(int_value as u32)
                    } else if int_value <= usize::MAX as i64 {
                        SystemValue::Usize(int_value as usize)
                    } else {
                        SystemValue::I64(int_value)
                    }
                } else {
                    // 符号ありに変換
                    if int_value >= i8::MIN as i64 && int_value <= i8::MAX as i64 {
                        SystemValue::I8(int_value as i8)
                    } else if int_value >= i16::MIN as i64 && int_value <= i16::MAX as i64 {
                        SystemValue::I16(int_value as i16)
                    } else if int_value >= i32::MIN as i64 && int_value <= i32::MAX as i64 {
                        SystemValue::I32(int_value as i32)
                    } else {
                        SystemValue::I64(int_value)
                    }
                }
            }

            Node {
                value: NodeValue::DataType(DataType::Float(ref value)),
                ..
            } => {
                let float_value = *value;
                if float_value >= f32::MIN as f64 && float_value <= f32::MAX as f64 {
                    SystemValue::F32(float_value as f32)
                } else {
                    SystemValue::F64(float_value)
                }
            }

            Node {
                value: NodeValue::DataType(DataType::Bool(ref value)),
                ..
            } => SystemValue::Bool(value.clone()),

            Node {
                value: NodeValue::DataType(DataType::Array(ref data_type, ref value)),
                ..
            } => {
                let mut vec: Vec<SystemValue> = vec![];
                for v in value {
                    let array_value = match **v {
                        Node {
                            value: NodeValue::Variable(_, ref name, _, _, _),
                            ..
                        } => SystemValue::String(name.clone()),
                        Node {
                            value: NodeValue::DataType(DataType::String(ref value)),
                            ..
                        } => SystemValue::String(value.clone()),

                        Node {
                            value: NodeValue::DataType(DataType::Int(ref value)),
                            ..
                        } => {
                            let int_value = *value;
                            if int_value >= 0 {
                                // 符号なしに変換
                                if int_value <= u8::MAX as i64 {
                                    SystemValue::U8(int_value as u8)
                                } else if int_value <= u16::MAX as i64 {
                                    SystemValue::U16(int_value as u16)
                                } else if int_value <= u32::MAX as i64 {
                                    SystemValue::U32(int_value as u32)
                                } else if int_value <= usize::MAX as i64 {
                                    SystemValue::Usize(int_value as usize)
                                } else {
                                    SystemValue::I64(int_value)
                                }
                            } else {
                                // 符号ありに変換
                                if int_value >= i8::MIN as i64 && int_value <= i8::MAX as i64 {
                                    SystemValue::I8(int_value as i8)
                                } else if int_value >= i16::MIN as i64
                                    && int_value <= i16::MAX as i64
                                {
                                    SystemValue::I16(int_value as i16)
                                } else if int_value >= i32::MIN as i64
                                    && int_value <= i32::MAX as i64
                                {
                                    SystemValue::I32(int_value as i32)
                                } else {
                                    SystemValue::I64(int_value)
                                }
                            }
                        }

                        Node {
                            value: NodeValue::DataType(DataType::Float(ref value)),
                            ..
                        } => {
                            let float_value = *value;
                            if float_value >= f32::MIN as f64 && float_value <= f32::MAX as f64 {
                                SystemValue::F32(float_value as f32)
                            } else {
                                SystemValue::F64(float_value)
                            }
                        }

                        Node {
                            value: NodeValue::DataType(DataType::Bool(ref value)),
                            ..
                        } => SystemValue::Bool(value.clone()),

                        _ => SystemValue::Null,
                    };
                    vec.push(SystemValue::Pointer(Box::new(array_value)));
                }
                SystemValue::Array(vec)
            }

            _ => SystemValue::Null,
        }
    }
}
// Addトレイトの実装
impl Add for SystemValue {
    type Output = Result<SystemValue, String>;

    fn add(self, other: SystemValue) -> Self::Output {
        match (self, other) {
            (SystemValue::U8(a), SystemValue::U8(b)) => Ok(SystemValue::U8(a + b)),
            (SystemValue::U16(a), SystemValue::U16(b)) => Ok(SystemValue::U16(a + b)),
            (SystemValue::U32(a), SystemValue::U32(b)) => Ok(SystemValue::U32(a + b)),

            (SystemValue::U64(a), SystemValue::U64(b)) => Ok(SystemValue::U64(a + b)),
            (SystemValue::I8(a), SystemValue::I8(b)) => Ok(SystemValue::I8(a + b)),
            (SystemValue::I16(a), SystemValue::I16(b)) => Ok(SystemValue::I16(a + b)),
            (SystemValue::I32(a), SystemValue::I32(b)) => Ok(SystemValue::I32(a + b)),
            (SystemValue::I64(a), SystemValue::I64(b)) => Ok(SystemValue::I64(a + b)),
            (SystemValue::F32(a), SystemValue::F32(b)) => Ok(SystemValue::F32(a + b)),
            (SystemValue::F64(a), SystemValue::F64(b)) => Ok(SystemValue::F64(a + b)),
            (SystemValue::String(a), SystemValue::String(b)) => Ok(SystemValue::String(a + &b)),
            _ => Err("加算できない型です".to_string()),
        }
    }
}

// Subトレイトの実装
impl Sub for SystemValue {
    type Output = Result<SystemValue, String>;

    fn sub(self, other: SystemValue) -> Self::Output {
        match (self, other) {
            (SystemValue::U8(a), SystemValue::U8(b)) => Ok(SystemValue::U8(a - b)),
            (SystemValue::U16(a), SystemValue::U16(b)) => Ok(SystemValue::U16(a - b)),
            (SystemValue::U32(a), SystemValue::U32(b)) => Ok(SystemValue::U32(a - b)),
            (SystemValue::U64(a), SystemValue::U64(b)) => Ok(SystemValue::U64(a - b)),
            (SystemValue::I8(a), SystemValue::I8(b)) => Ok(SystemValue::I8(a - b)),
            (SystemValue::I16(a), SystemValue::I16(b)) => Ok(SystemValue::I16(a - b)),
            (SystemValue::I32(a), SystemValue::I32(b)) => Ok(SystemValue::I32(a - b)),

            (SystemValue::I64(a), SystemValue::I64(b)) => Ok(SystemValue::I64(a - b)),
            (SystemValue::F64(a), SystemValue::F64(b)) => Ok(SystemValue::F64(a - b)),
            _ => Err("減算できない型です".to_string()),
        }
    }
}

// Mulトレイトの実装
impl Mul for SystemValue {
    type Output = Result<SystemValue, String>;

    fn mul(self, other: SystemValue) -> Self::Output {
        match (self, other) {
            (SystemValue::U8(a), SystemValue::U8(b)) => Ok(SystemValue::U8(a * b)),
            (SystemValue::U16(a), SystemValue::U16(b)) => Ok(SystemValue::U16(a * b)),
            (SystemValue::U32(a), SystemValue::U32(b)) => Ok(SystemValue::U32(a * b)),
            (SystemValue::U64(a), SystemValue::U64(b)) => Ok(SystemValue::U64(a * b)),
            (SystemValue::I8(a), SystemValue::I8(b)) => Ok(SystemValue::I8(a * b)),
            (SystemValue::I16(a), SystemValue::I16(b)) => Ok(SystemValue::I16(a * b)),
            (SystemValue::I32(a), SystemValue::I32(b)) => Ok(SystemValue::I32(a * b)),

            (SystemValue::I64(a), SystemValue::I64(b)) => Ok(SystemValue::I64(a * b)),
            (SystemValue::F64(a), SystemValue::F64(b)) => Ok(SystemValue::F64(a * b)),
            _ => Err("乗算できない型です".to_string()),
        }
    }
}

// Divトレイトの実装
impl Div for SystemValue {
    type Output = Result<SystemValue, String>;

    fn div(self, other: SystemValue) -> Self::Output {
        match (self, other) {
            (SystemValue::U8(a), SystemValue::U8(b)) => {
                if b == 0 {
                    Err("ゼロで割ることはできません".to_string())
                } else {
                    Ok(SystemValue::U8(a / b))
                }
            }

            (SystemValue::U16(a), SystemValue::U16(b)) => {
                if b == 0 {
                    Err("ゼロで割ることはできません".to_string())
                } else {
                    Ok(SystemValue::U16(a / b))
                }
            }

            (SystemValue::U32(a), SystemValue::U32(b)) => {
                if b == 0 {
                    Err("ゼロで割ることはできません".to_string())
                } else {
                    Ok(SystemValue::U32(a / b))
                }
            }

            (SystemValue::U64(a), SystemValue::U64(b)) => {
                if b == 0 {
                    Err("ゼロで割ることはできません".to_string())
                } else {
                    Ok(SystemValue::U64(a / b))
                }
            }

            (SystemValue::I8(a), SystemValue::I8(b)) => {
                if b == 0 {
                    Err("ゼロで割ることはできません".to_string())
                } else {
                    Ok(SystemValue::I8(a / b))
                }
            }

            (SystemValue::I16(a), SystemValue::I16(b)) => {
                if b == 0 {
                    Err("ゼロで割ることはできません".to_string())
                } else {
                    Ok(SystemValue::I16(a / b))
                }
            }

            (SystemValue::I32(a), SystemValue::I32(b)) => {
                if b == 0 {
                    Err("ゼロで割ることはできません".to_string())
                } else {
                    Ok(SystemValue::I32(a / b))
                }
            }

            (SystemValue::I64(a), SystemValue::I64(b)) => {
                if b == 0 {
                    Err("ゼロで割ることはできません".to_string())
                } else {
                    Ok(SystemValue::I64(a / b))
                }
            }
            (SystemValue::F64(a), SystemValue::F64(b)) => {
                if b == 0.0 {
                    Err("ゼロで割ることはできません".to_string())
                } else {
                    Ok(SystemValue::F64(a / b))
                }
            }
            _ => Err("除算できない型です".to_string()),
        }
    }
}

// BitAndトレイトの実装
impl BitAnd for SystemValue {
    type Output = Result<SystemValue, String>;

    fn bitand(self, other: SystemValue) -> Self::Output {
        match (self, other) {
            (SystemValue::I8(a), SystemValue::I8(b)) => Ok(SystemValue::I8(a & b)),
            (SystemValue::I16(a), SystemValue::I16(b)) => Ok(SystemValue::I16(a & b)),
            (SystemValue::I32(a), SystemValue::I32(b)) => Ok(SystemValue::I32(a & b)),
            (SystemValue::I64(a), SystemValue::I64(b)) => Ok(SystemValue::I64(a & b)),

            (SystemValue::U8(a), SystemValue::U8(b)) => Ok(SystemValue::U8(a & b)),
            (SystemValue::U16(a), SystemValue::U16(b)) => Ok(SystemValue::U16(a & b)),
            (SystemValue::U32(a), SystemValue::U32(b)) => Ok(SystemValue::U32(a & b)),
            (SystemValue::U64(a), SystemValue::U64(b)) => Ok(SystemValue::U64(a & b)),
            _ => Err("ビットAND演算できない型です".to_string()),
        }
    }
}

// BitOrトレイトの実装
impl BitOr for SystemValue {
    type Output = Result<SystemValue, String>;

    fn bitor(self, other: SystemValue) -> Self::Output {
        match (self, other) {
            (SystemValue::I8(a), SystemValue::I8(b)) => Ok(SystemValue::I8(a | b)),
            (SystemValue::I16(a), SystemValue::I16(b)) => Ok(SystemValue::I16(a | b)),
            (SystemValue::I32(a), SystemValue::I32(b)) => Ok(SystemValue::I32(a | b)),
            (SystemValue::I64(a), SystemValue::I64(b)) => Ok(SystemValue::I64(a | b)),

            (SystemValue::U8(a), SystemValue::U8(b)) => Ok(SystemValue::U8(a | b)),
            (SystemValue::U16(a), SystemValue::U16(b)) => Ok(SystemValue::U16(a | b)),
            (SystemValue::U32(a), SystemValue::U32(b)) => Ok(SystemValue::U32(a | b)),
            (SystemValue::U64(a), SystemValue::U64(b)) => Ok(SystemValue::U64(a | b)),

            _ => Err("ビットOR演算できない型です".to_string()),
        }
    }
}

// BitXorトレイトの実装
impl BitXor for SystemValue {
    type Output = Result<SystemValue, String>;

    fn bitxor(self, other: SystemValue) -> Self::Output {
        match (self, other) {
            (SystemValue::I8(a), SystemValue::I8(b)) => Ok(SystemValue::I8(a ^ b)),
            (SystemValue::I16(a), SystemValue::I16(b)) => Ok(SystemValue::I16(a ^ b)),
            (SystemValue::I32(a), SystemValue::I32(b)) => Ok(SystemValue::I32(a ^ b)),
            (SystemValue::I64(a), SystemValue::I64(b)) => Ok(SystemValue::I64(a ^ b)),

            (SystemValue::U8(a), SystemValue::U8(b)) => Ok(SystemValue::U8(a ^ b)),
            (SystemValue::U16(a), SystemValue::U16(b)) => Ok(SystemValue::U16(a ^ b)),
            (SystemValue::U32(a), SystemValue::U32(b)) => Ok(SystemValue::U32(a ^ b)),
            (SystemValue::U64(a), SystemValue::U64(b)) => Ok(SystemValue::U64(a ^ b)),

            _ => Err("ビットXOR演算できない型です".to_string()),
        }
    }
}

// Notトレイトの実装
impl Not for SystemValue {
    type Output = Result<SystemValue, String>;

    fn not(self) -> Self::Output {
        match self {
            (SystemValue::I8(a)) => Ok(SystemValue::I8(!a)),
            (SystemValue::I16(a)) => Ok(SystemValue::I16(!a)),
            (SystemValue::I32(a)) => Ok(SystemValue::I32(!a)),
            (SystemValue::I64(a)) => Ok(SystemValue::I64(!a)),

            (SystemValue::U8(a)) => Ok(SystemValue::U8(!a)),
            (SystemValue::U16(a)) => Ok(SystemValue::U16(!a)),
            (SystemValue::U32(a)) => Ok(SystemValue::U32(!a)),
            (SystemValue::U64(a)) => Ok(SystemValue::U64(!a)),

            _ => Err("ビット反転できない型です".to_string()),
        }
    }
}

impl Shl<u32> for SystemValue {
    type Output = Result<SystemValue, String>;

    fn shl(self, rhs: u32) -> Self::Output {
        match self {
            (SystemValue::I8(a)) => Ok(SystemValue::I8(a << rhs)),
            (SystemValue::I16(a)) => Ok(SystemValue::I16(a << rhs)),
            (SystemValue::I32(a)) => Ok(SystemValue::I32(a << rhs)),
            (SystemValue::I64(a)) => Ok(SystemValue::I64(a << rhs)),

            (SystemValue::U8(a)) => Ok(SystemValue::U8(a << rhs)),
            (SystemValue::U16(a)) => Ok(SystemValue::U16(a << rhs)),
            (SystemValue::U32(a)) => Ok(SystemValue::U32(a << rhs)),
            (SystemValue::U64(a)) => Ok(SystemValue::U64(a << rhs)),

            _ => Err("ビットシフト左できない型です".to_string()),
        }
    }
}

impl Shr<u32> for SystemValue {
    type Output = Result<SystemValue, String>;

    fn shr(self, rhs: u32) -> Self::Output {
        match self {
            (SystemValue::I8(a)) => Ok(SystemValue::I8(a << rhs)),
            (SystemValue::I16(a)) => Ok(SystemValue::I16(a >> rhs)),
            (SystemValue::I32(a)) => Ok(SystemValue::I32(a >> rhs)),
            (SystemValue::I64(a)) => Ok(SystemValue::I64(a >> rhs)),

            (SystemValue::U8(a)) => Ok(SystemValue::U8(a >> rhs)),
            (SystemValue::U16(a)) => Ok(SystemValue::U16(a >> rhs)),
            (SystemValue::U32(a)) => Ok(SystemValue::U32(a >> rhs)),
            (SystemValue::U64(a)) => Ok(SystemValue::U64(a >> rhs)),

            _ => Err("ビットシフト右できない型です".to_string()),
        }
    }
}

impl Shl<SystemValue> for SystemValue {
    type Output = Result<SystemValue, String>;

    fn shl(self, rhs: SystemValue) -> Self::Output {
        match rhs {
            SystemValue::I32(b) => {
                if b < 0 {
                    return Err("シフト量は負であってはいけません".to_string());
                }
                let shift = b as u32;

                match self {
                    SystemValue::I8(a) => Ok(SystemValue::I8(a << shift)),
                    SystemValue::I16(a) => Ok(SystemValue::I16(a << shift)),
                    SystemValue::I32(a) => Ok(SystemValue::I32(a << shift)),
                    SystemValue::I64(a) => Ok(SystemValue::I64(a << shift)),
                    SystemValue::U8(a) => Ok(SystemValue::U8(a << shift)),
                    SystemValue::U16(a) => Ok(SystemValue::U16(a << shift)),
                    SystemValue::U32(a) => Ok(SystemValue::U32(a << shift)),
                    SystemValue::U64(a) => Ok(SystemValue::U64(a << shift)),
                    _ => Err("ビットシフト左できない型です".to_string()),
                }
            }
            _ => Err("シフト量はI32型でなければなりません".to_string()),
        }
    }
}

impl Shr<SystemValue> for SystemValue {
    type Output = Result<SystemValue, String>;

    fn shr(self, rhs: SystemValue) -> Self::Output {
        match rhs {
            SystemValue::I32(b) => {
                if b < 0 {
                    return Err("シフト量は負であってはいけません".to_string());
                }
                let shift = b as u32;

                match self {
                    SystemValue::I8(a) => Ok(SystemValue::I8(a >> shift)),
                    SystemValue::I16(a) => Ok(SystemValue::I16(a >> shift)),
                    SystemValue::I32(a) => Ok(SystemValue::I32(a >> shift)),
                    SystemValue::I64(a) => Ok(SystemValue::I64(a >> shift)),
                    SystemValue::U8(a) => Ok(SystemValue::U8(a >> shift)),
                    SystemValue::U16(a) => Ok(SystemValue::U16(a >> shift)),
                    SystemValue::U32(a) => Ok(SystemValue::U32(a >> shift)),
                    SystemValue::U64(a) => Ok(SystemValue::U64(a >> shift)),
                    _ => Err("ビットシフト右できない型です".to_string()),
                }
            }
            _ => Err("シフト量はI32型でなければなりません".to_string()),
        }
    }
}
impl PartialEq for SystemValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SystemValue::Usize(a), SystemValue::Usize(b)) => a == b,
            (SystemValue::U8(a), SystemValue::U8(b)) => a == b,
            (SystemValue::U16(a), SystemValue::U16(b)) => a == b,
            (SystemValue::U32(a), SystemValue::U32(b)) => a == b,
            (SystemValue::U64(a), SystemValue::U64(b)) => a == b,
            (SystemValue::I8(a), SystemValue::I8(b)) => a == b,
            (SystemValue::I16(a), SystemValue::I16(b)) => a == b,
            (SystemValue::I32(a), SystemValue::I32(b)) => a == b,
            (SystemValue::I64(a), SystemValue::I64(b)) => a == b,
            (SystemValue::F32(a), SystemValue::F32(b)) => a == b,
            (SystemValue::F64(a), SystemValue::F64(b)) => a == b,
            (SystemValue::String(a), SystemValue::String(b)) => a == b,
            (SystemValue::Bool(a), SystemValue::Bool(b)) => a == b,
            (SystemValue::Null, SystemValue::Null) => true,
            _ => false,
        }
    }
}

impl Eq for SystemValue {}

// Orderingを用いた比較
impl PartialOrd for SystemValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (SystemValue::I32(a), SystemValue::I32(b)) => a.partial_cmp(b),
            (SystemValue::I64(a), SystemValue::I64(b)) => a.partial_cmp(b),
            (SystemValue::F32(a), SystemValue::F32(b)) => a.partial_cmp(b),
            (SystemValue::F64(a), SystemValue::F64(b)) => a.partial_cmp(b),
            (SystemValue::U32(a), SystemValue::U32(b)) => a.partial_cmp(b),
            (SystemValue::U64(a), SystemValue::U64(b)) => a.partial_cmp(b),
            (SystemValue::String(a), SystemValue::String(b)) => a.partial_cmp(b),
            _ => None, // 型が異なる場合は比較できない
        }
    }
}

impl Ord for SystemValue {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

#[cfg(any(feature = "full", feature = "decoder"))]
impl From<String> for SystemValue {
    fn from(s: String) -> Self {
        SystemValue::String(s)
    }
}

// Vec<Box<Node>>からSystemValueへの変換
#[cfg(any(feature = "full", feature = "decoder"))]
impl From<Vec<Box<Node>>> for SystemValue {
    fn from(nodes: Vec<Box<Node>>) -> Self {
        let mut vec: Vec<SystemValue> = vec![];
        for node in nodes {
            let system_value: SystemValue = (node).clone().into(); // Box<Node>をSystemValueに変換
            vec.push(SystemValue::Pointer(Box::new(system_value)));
        }
        SystemValue::Array(vec)
    }
}

// Vec<String>からSystemValueへの変換
#[cfg(any(feature = "full", feature = "decoder"))]
impl From<Vec<String>> for SystemValue {
    fn from(strings: Vec<String>) -> Self {
        let vec: Vec<SystemValue> = strings
            .into_iter()
            .map(|s| SystemValue::Pointer(Box::new(SystemValue::String(s)))) // 各StringをSystemValue::Stringに変換
            .collect();
        SystemValue::Array(vec)
    }
}
// Vec<&str>からSystemValueへの変換
#[cfg(any(feature = "full", feature = "decoder"))]
impl From<Vec<&str>> for SystemValue {
    fn from(strings: Vec<&str>) -> Self {
        let vec: Vec<SystemValue> = strings
            .into_iter()
            .map(|s| SystemValue::Pointer(Box::new(SystemValue::String(s.to_string())))) // 各StringをSystemValue::Stringに変換
            .collect();
        SystemValue::Array(vec)
    }
}

// Vec<u8>からSystemValueへの変換
#[cfg(any(feature = "full", feature = "decoder"))]
impl From<Vec<u8>> for SystemValue {
    fn from(bytes: Vec<u8>) -> Self {
        let vec: Vec<SystemValue> = bytes
            .into_iter()
            .map(|b| SystemValue::Pointer(Box::new(SystemValue::U8(b))))
            .collect();
        SystemValue::Array(vec)
    }
}

// Vec<u16>からSystemValueへの変換
#[cfg(any(feature = "full", feature = "decoder"))]
impl From<Vec<u16>> for SystemValue {
    fn from(nums: Vec<u16>) -> Self {
        let vec: Vec<SystemValue> = nums
            .into_iter()
            .map(|n| SystemValue::Pointer(Box::new(SystemValue::U16(n))))
            .collect();
        SystemValue::Array(vec)
    }
}

// Vec<u32>からSystemValueへの変換
#[cfg(any(feature = "full", feature = "decoder"))]
impl From<Vec<u32>> for SystemValue {
    fn from(nums: Vec<u32>) -> Self {
        let vec: Vec<SystemValue> = nums
            .into_iter()
            .map(|n| SystemValue::Pointer(Box::new(SystemValue::U32(n))))
            .collect();
        SystemValue::Array(vec)
    }
}

// Vec<u64>からSystemValueへの変換
#[cfg(any(feature = "full", feature = "decoder"))]
impl From<Vec<u64>> for SystemValue {
    fn from(nums: Vec<u64>) -> Self {
        let vec: Vec<SystemValue> = nums
            .into_iter()
            .map(|n| SystemValue::Pointer(Box::new(SystemValue::U64(n))))
            .collect();
        SystemValue::Array(vec)
    }
}

// Vec<i8>からSystemValueへの変換
#[cfg(any(feature = "full", feature = "decoder"))]
impl From<Vec<i8>> for SystemValue {
    fn from(nums: Vec<i8>) -> Self {
        let vec: Vec<SystemValue> = nums
            .into_iter()
            .map(|n| SystemValue::Pointer(Box::new(SystemValue::I8(n))))
            .collect();
        SystemValue::Array(vec)
    }
}

// Vec<i16>からSystemValueへの変換
#[cfg(any(feature = "full", feature = "decoder"))]
impl From<Vec<i16>> for SystemValue {
    fn from(nums: Vec<i16>) -> Self {
        let vec: Vec<SystemValue> = nums
            .into_iter()
            .map(|n| SystemValue::Pointer(Box::new(SystemValue::I16(n))))
            .collect();
        SystemValue::Array(vec)
    }
}

// Vec<i32>からSystemValueへの変換
#[cfg(any(feature = "full", feature = "decoder"))]
impl From<Vec<i32>> for SystemValue {
    fn from(nums: Vec<i32>) -> Self {
        let vec: Vec<SystemValue> = nums
            .into_iter()
            .map(|n| SystemValue::Pointer(Box::new(SystemValue::I32(n))))
            .collect();
        SystemValue::Array(vec)
    }
}

// Vec<f32>からSystemValueへの変換
#[cfg(any(feature = "full", feature = "decoder"))]
impl From<Vec<f32>> for SystemValue {
    fn from(nums: Vec<f32>) -> Self {
        let vec: Vec<SystemValue> = nums
            .into_iter()
            .map(|n| SystemValue::Pointer(Box::new(SystemValue::F32(n))))
            .collect();
        SystemValue::Array(vec)
    }
}

// Vec<f64>からSystemValueへの変換
#[cfg(any(feature = "full", feature = "decoder"))]
impl From<Vec<f64>> for SystemValue {
    fn from(nums: Vec<f64>) -> Self {
        let vec: Vec<SystemValue> = nums
            .into_iter()
            .map(|n| SystemValue::Pointer(Box::new(SystemValue::F64(n))))
            .collect();
        SystemValue::Array(vec)
    }
}

// Vec<bool>からSystemValueへの変換
#[cfg(any(feature = "full", feature = "decoder"))]
impl From<Vec<bool>> for SystemValue {
    fn from(bools: Vec<bool>) -> Self {
        let vec: Vec<SystemValue> = bools
            .into_iter()
            .map(|b| SystemValue::Pointer(Box::new(SystemValue::Bool(b))))
            .collect();
        SystemValue::Array(vec)
    }
}
impl From<usize> for SystemValue {
    fn from(value: usize) -> Self {
        SystemValue::Usize(value)
    }
}

impl From<u8> for SystemValue {
    fn from(value: u8) -> Self {
        SystemValue::U8(value)
    }
}

impl From<u16> for SystemValue {
    fn from(value: u16) -> Self {
        SystemValue::U16(value)
    }
}

impl From<u32> for SystemValue {
    fn from(value: u32) -> Self {
        SystemValue::U32(value)
    }
}

impl From<u64> for SystemValue {
    fn from(value: u64) -> Self {
        SystemValue::U64(value)
    }
}

impl From<i8> for SystemValue {
    fn from(value: i8) -> Self {
        SystemValue::I8(value)
    }
}

impl From<i16> for SystemValue {
    fn from(value: i16) -> Self {
        SystemValue::I16(value)
    }
}

impl From<i32> for SystemValue {
    fn from(value: i32) -> Self {
        SystemValue::I32(value)
    }
}

impl From<i64> for SystemValue {
    fn from(value: i64) -> Self {
        SystemValue::I64(value)
    }
}

impl From<f32> for SystemValue {
    fn from(value: f32) -> Self {
        SystemValue::F32(value)
    }
}

impl From<f64> for SystemValue {
    fn from(value: f64) -> Self {
        SystemValue::F64(value)
    }
}

impl From<bool> for SystemValue {
    fn from(value: bool) -> Self {
        SystemValue::Bool(value)
    }
}

// 1要素のタプル
impl<T1: Into<SystemValue>> From<(T1,)> for SystemValue {
    fn from(tuple: (T1,)) -> Self {
        let (first,) = tuple;
        SystemValue::Tuple(Box::new(SystemValueTuple::Tuple1(Box::new(first.into()))))
    }
}

// 2要素のタプル
impl<T1: Into<SystemValue>, T2: Into<SystemValue>> From<(T1, T2)> for SystemValue {
    fn from(tuple: (T1, T2)) -> Self {
        let (first, second) = tuple;
        SystemValue::Tuple(Box::new(SystemValueTuple::Tuple2(
            Box::new(first.into()),
            Box::new(second.into()),
        )))
    }
}

// 3要素のタプル
impl<T1: Into<SystemValue>, T2: Into<SystemValue>, T3: Into<SystemValue>> From<(T1, T2, T3)>
    for SystemValue
{
    fn from(tuple: (T1, T2, T3)) -> Self {
        let (first, second, third) = tuple;
        SystemValue::Tuple(Box::new(SystemValueTuple::Tuple3(
            Box::new(first.into()),
            Box::new(second.into()),
            Box::new(third.into()),
        )))
    }
}

// 4要素のタプル
impl<
        T1: Into<SystemValue>,
        T2: Into<SystemValue>,
        T3: Into<SystemValue>,
        T4: Into<SystemValue>,
    > From<(T1, T2, T3, T4)> for SystemValue
{
    fn from(tuple: (T1, T2, T3, T4)) -> Self {
        let (first, second, third, fourth) = tuple;
        SystemValue::Tuple(Box::new(SystemValueTuple::Tuple4(
            Box::new(first.into()),
            Box::new(second.into()),
            Box::new(third.into()),
            Box::new(fourth.into()),
        )))
    }
}

// 5要素のタプル
impl<
        T1: Into<SystemValue>,
        T2: Into<SystemValue>,
        T3: Into<SystemValue>,
        T4: Into<SystemValue>,
        T5: Into<SystemValue>,
    > From<(T1, T2, T3, T4, T5)> for SystemValue
{
    fn from(tuple: (T1, T2, T3, T4, T5)) -> Self {
        let (first, second, third, fourth, fifth) = tuple;
        SystemValue::Tuple(Box::new(SystemValueTuple::Tuple5(
            Box::new(first.into()),
            Box::new(second.into()),
            Box::new(third.into()),
            Box::new(fourth.into()),
            Box::new(fifth.into()),
        )))
    }
}

// デフォルト値(デフォルト値はI32(0))
#[cfg(any(feature = "full", feature = "decoder"))]
impl Default for SystemValue {
    fn default() -> Self {
        SystemValue::I32(0)
    }
}

#[cfg(any(feature = "full", feature = "decoder"))]
impl fmt::Display for SystemValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SystemValue::Null => write!(f, "null"),
            SystemValue::Bool(b) => write!(f, "{}", b),
            SystemValue::Usize(n) => write!(f, "{}", n),
            SystemValue::U8(n) => write!(f, "{}", n),
            SystemValue::U16(n) => write!(f, "{}", n),
            SystemValue::U32(n) => write!(f, "{}", n),
            SystemValue::U64(n) => write!(f, "{}", n),
            SystemValue::I8(n) => write!(f, "{}", n),
            SystemValue::I16(n) => write!(f, "{}", n),
            SystemValue::I32(n) => write!(f, "{}", n),
            SystemValue::I64(n) => write!(f, "{}", n),
            SystemValue::F32(n) => write!(f, "{}", n),
            SystemValue::F64(n) => write!(f, "{}", n),
            SystemValue::String(s) => write!(f, "{}", s),
            SystemValue::Array(arr) => {
                let elements: Vec<String> = arr.iter().map(|v| format!("{}", v)).collect();
                write!(f, "[{}]", elements.join(", "))
            }
            SystemValue::Pointer(ptr) => write!(f, "{}", ptr),
            SystemValue::Tuple(vec) => {
                //  let elements: Vec<String> = vec.iter().map(|v| format!("{}", v)).collect();
                write!(f, "")
            }
            SystemValue::Struct(_) => {
                write!(f, "")
            }

            SystemValue::System(_) => todo!(),
            SystemValue::__NodeBlock(nodes) => write!(f, ""),
        }
    }
}
