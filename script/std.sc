// std.script
type u8 = "u8";
type u16 = "u16";
type u32 = "u32";
type u64 = "u64";
type i8 = "i8";
type i16 = "i16";
type i32 = "i32";
type i64 = "i64";
type f32 = "f32";

type p_u8 = "pu8";
type p_u16 = "pu16";
type p_u32 = "pu32";
type p_u64 = "pu64";
type p_i8 = "pi8";
type p_i16 = "pi16";
type p_i32 = "pi32";
type p_i64 = "pi64";
type p_f32 = "pf32";

type string = "string";
type void = "unit";
type array = "array";
type bool = "bool";
struct Vector;
struct HashMap;
sturct IO;
struct File;
struct Memory;
struct __UTF8_String{
  value:&str,
}
impl __UTF8_String{
  fn new()->Self{
    __UTF8_Stirng{
      value:Default(),  
    }
}
struct String{
    value:&__UTF8_String,
}
impl String{
    fn new()->Self{
        String{
            value:__UTF8_String::new(),
        }
    }
}



