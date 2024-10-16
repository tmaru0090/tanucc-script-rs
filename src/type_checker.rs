use crate::types::DataType;
use crate::types::NodeValue;
use crate::types::SystemValue;
use crate::types::Value;
use crate::types::R;

#[derive(Debug, Clone)]
pub struct TypeChecker;
impl TypeChecker {
    // Type inference: infers the type based on NodeValue
    pub fn infer_type(value: &NodeValue) -> R<String, String> {
        match value {
            // Integer types
            NodeValue::DataType(DataType::Int(val)) => {
                if *val <= i8::MAX as i64 && *val >= i8::MIN as i64 {
                    Ok("i8".to_string())
                } else if *val <= i16::MAX as i64 && *val >= i16::MIN as i64 {
                    Ok("i16".to_string())
                } else if *val <= i32::MAX as i64 && *val >= i32::MIN as i64 {
                    Ok("i32".to_string())
                } else if *val <= i64::MAX && *val >= i64::MIN {
                    Ok("i64".to_string())
                } else if *val >= 0 && *val <= u8::MAX as i64 {
                    Ok("u8".to_string())
                } else if *val >= 0 && *val <= u16::MAX as i64 {
                    Ok("u16".to_string())
                } else if *val >= 0 && *val <= u32::MAX as i64 {
                    Ok("u32".to_string())
                } else if *val >= 0 && *val <= u64::MAX as i64 {
                    Ok("u64".to_string())
                } else {
                    Err("Integer value out of bounds for known types".to_string())
                }
            }
            // Array type
            NodeValue::DataType(DataType::Array(box_type, ref values)) => {
                // 各要素の型を収集する
                let mut element_types = Vec::new();
                for value in values {
                    match TypeChecker::infer_type(&value.value()) {
                        Ok(typ) => element_types.push(typ),
                        Err(err) => return Err(err), // エラーがあればそのまま返す
                    }
                }
                // 収集した型を結合してarray型を形成
                let type_string = format!("array<{}>", element_types.join(","));
                Ok(type_string)
            }
            // Float types
            NodeValue::DataType(DataType::Float(val)) => {
                if *val <= f32::MAX as f64 && *val >= f32::MIN as f64 {
                    Ok("f32".to_string())
                } else if *val <= f64::MAX && *val >= f64::MIN {
                    Ok("f64".to_string())
                } else {
                    Err("Float value out of bounds for known types".to_string())
                }
            }

            NodeValue::DataType(DataType::Generic(name, lists)) => {
                Ok(format!("{}<{}>", name, lists.join(",")))
            }
            // String type
            NodeValue::DataType(DataType::String(_)) => Ok("string".to_string()),

            // Boolean type
            NodeValue::DataType(DataType::Bool(_)) => Ok("bool".to_string()),

            // Operator type
            NodeValue::Operator(_) => Ok("operator".to_string()),

            // Control flow type
            NodeValue::ControlFlow(_) => Ok("control_flow".to_string()),

            // Unknown or unsupported type
            _ => Err("Unable to infer type".to_string()),
        }
    }

    // 値の型変換
    pub fn convert_to_value(value: &NodeValue) -> Value {
        match value {
            // 整数型
            NodeValue::DataType(DataType::Int(val)) => {
                if *val <= i8::MAX as i64 && *val >= i8::MIN as i64 {
                    Ok(SystemValue::I8(*val as i8))
                } else if *val <= i16::MAX as i64 && *val >= i16::MIN as i64 {
                    Ok(SystemValue::I16(*val as i16))
                } else if *val <= i32::MAX as i64 && *val >= i32::MIN as i64 {
                    Ok(SystemValue::I32(*val as i32))
                } else if *val <= i64::MAX && *val >= i64::MIN {
                    Ok(SystemValue::I64(*val))
                } else if *val >= 0 && *val <= u8::MAX as i64 {
                    Ok(SystemValue::U8(*val as u8))
                } else if *val >= 0 && *val <= u16::MAX as i64 {
                    Ok(SystemValue::U16(*val as u16))
                } else if *val >= 0 && *val <= u32::MAX as i64 {
                    Ok(SystemValue::U32(*val as u32))
                } else if *val >= 0 && *val <= u64::MAX as i64 {
                    Ok(SystemValue::U64(*val as u64))
                } else {
                    Err("Integer value out of bounds for known types".to_string())
                }
            }

            // 浮動小数点型
            NodeValue::DataType(DataType::Float(val)) => {
                if *val <= f32::MAX as f64 && *val >= f32::MIN as f64 {
                    Ok(SystemValue::F32(*val as f32))
                } else if *val <= f64::MAX && *val >= f64::MIN {
                    Ok(SystemValue::F64(*val))
                } else {
                    Err("Float value out of bounds for known types".to_string())
                }
            }
            // 文字列型
            NodeValue::DataType(DataType::String(s)) => Ok(SystemValue::String(s.clone())),

            // ブール型
            NodeValue::DataType(DataType::Bool(b)) => Ok(SystemValue::Bool(*b)),

            // 未対応の型
            _ => Err("Unsupported NodeValue type for conversion".to_string()),
        }
    }
    // Type conversion: converts NodeValue into a SystemValue based on the expected type
    pub fn convert_type_to_value(type_name: &String, value: &NodeValue) -> Value {
        match type_name.as_str() {
            "i64" => {
                if let NodeValue::DataType(DataType::Int(val)) = value {
                    Ok(SystemValue::I64(*val))
                } else {
                    Err(format!("Failed to convert to type i64: {:?}", value))
                }
            }
            "f64" => {
                if let NodeValue::DataType(DataType::Float(val)) = value {
                    Ok(SystemValue::F64(*val))
                } else {
                    Err(format!("Failed to convert to type f64: {:?}", value))
                }
            }
            "string" => {
                if let NodeValue::DataType(DataType::String(val)) = value {
                    Ok(SystemValue::String(val.clone()))
                } else {
                    Err(format!("Failed to convert to type String: {:?}", value))
                }
            }
            "bool" => {
                if let NodeValue::DataType(DataType::Bool(val)) = value {
                    Ok(SystemValue::Bool(*val))
                } else {
                    Err(format!("Failed to convert to type bool: {:?}", value))
                }
            }
            "usize" => {
                if let NodeValue::DataType(DataType::Int(val)) = value {
                    Ok(SystemValue::Usize(*val as usize))
                } else {
                    Err(format!("Failed to convert to type usize: {:?}", value))
                }
            }
            "u8" => {
                if let NodeValue::DataType(DataType::Int(val)) = value {
                    Ok(SystemValue::U8(*val as u8))
                } else {
                    Err(format!("Failed to convert to type u8: {:?}", value))
                }
            }
            "u16" => {
                if let NodeValue::DataType(DataType::Int(val)) = value {
                    Ok(SystemValue::U16(*val as u16))
                } else {
                    Err(format!("Failed to convert to type u16: {:?}", value))
                }
            }
            "u32" => {
                if let NodeValue::DataType(DataType::Int(val)) = value {
                    Ok(SystemValue::U32(*val as u32))
                } else {
                    Err(format!("Failed to convert to type u32: {:?}", value))
                }
            }
            "u64" => {
                if let NodeValue::DataType(DataType::Int(val)) = value {
                    Ok(SystemValue::U64(*val as u64))
                } else {
                    Err(format!("Failed to convert to type u64: {:?}", value))
                }
            }
            "i8" => {
                if let NodeValue::DataType(DataType::Int(val)) = value {
                    Ok(SystemValue::I8(*val as i8))
                } else {
                    Err(format!("Failed to convert to type i8: {:?}", value))
                }
            }
            "i16" => {
                if let NodeValue::DataType(DataType::Int(val)) = value {
                    Ok(SystemValue::I16(*val as i16))
                } else {
                    Err(format!("Failed to convert to type i16: {:?}", value))
                }
            }
            "i32" => {
                if let NodeValue::DataType(DataType::Int(val)) = value {
                    Ok(SystemValue::I32(*val as i32))
                } else {
                    Err(format!("Failed to convert to type i32: {:?}", value))
                }
            }
            "f32" => {
                if let NodeValue::DataType(DataType::Float(val)) = value {
                    Ok(SystemValue::F32(*val as f32))
                } else {
                    Err(format!("Failed to convert to type f32: {:?}", value))
                }
            }
            _ => Err(format!("Unknown type conversion request: {}", type_name)),
        }
    }

    // Assignment type check: checks if the type of a given value matches the expected type
    pub fn check_assign_type(type_name: &String, value: &SystemValue) -> R<(), String> {
        match (type_name.as_str(), value) {
            ("array", SystemValue::Array(_)) => Ok(()),
            ("i64", SystemValue::I64(_)) => Ok(()),
            ("f64", SystemValue::F64(_)) => Ok(()),
            ("string", SystemValue::String(_)) => Ok(()),
            ("bool", SystemValue::Bool(_)) => Ok(()),
            ("usize", SystemValue::Usize(_)) => Ok(()),
            ("u8", SystemValue::U8(_)) => Ok(()),
            ("u16", SystemValue::U16(_)) => Ok(()),
            ("u32", SystemValue::U32(_)) => Ok(()),
            ("u64", SystemValue::U64(_)) => Ok(()),
            ("i8", SystemValue::I8(_)) => Ok(()),
            ("i16", SystemValue::I16(_)) => Ok(()),
            ("i32", SystemValue::I32(_)) => Ok(()),
            ("f32", SystemValue::F32(_)) => Ok(()),
            _ => Err(format!(
                "Type mismatch: expected {}, found {:?}",
                type_name, value
            )),
        }
    }
}
