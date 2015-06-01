use std::collections::hash_map::HashMap;
use std::io;

pub enum FunctionArgs {
  None,
  Args1([proto::BasicType; 1]),
  Args2([proto::BasicType; 2]),
  Args3([proto::BasicType; 3]),
}

impl FunctionArgs {
  pub fn as_slice(&self) -> Option<&[proto::BasicType]> {
    use self::FunctionArgs::*;

    macro_rules! args_arm {
      ($arr:expr) => (
        Some(&$arr[..])
      )
    }

    match self {
      &None           => Option::None,
      &Args1(ref arr) => args_arm!(arr),
      &Args2(ref arr) => args_arm!(arr),
      &Args3(ref arr) => args_arm!(arr),
    }
  }

  pub fn len(&self) -> usize {
    match self.as_slice() {
      Some(ref slice) => slice.len(),
      None            => 0
    }
  }

  pub fn generate_cpp(&self, out_stream: &mut io::Write) -> io::Result<()> {
    if let Some(slice) = self.as_slice() {
      let mut i = 0u32;

      for btype in slice.iter() {
        if i > 0 {
          try!(out_stream.write_all(b", "));
        }

        try!(btype.generate_cpp(out_stream));

        try!(write!(out_stream, " arg{}", i));
        i += 1;
      }
    }

    Ok(())
  }

  pub fn generate_rs(&self, out_stream: &mut io::Write) -> io::Result<()> {
    if let Some(slice) = self.as_slice() {
      let mut i = 0u32;

      for btype in slice.iter() {
        if i > 0 {
          try!(out_stream.write_all(b", "));
        }

        try!(write!(out_stream, "arg{}: ", i));
        try!(btype.generate_rs(out_stream));

        i += 1;
      }
    }

    Ok(())
  }
}

pub struct Function {
  pub ret: proto::BasicType,
  pub args: FunctionArgs
}

pub struct Method {
  func: Function,
  is_const: bool
}

pub fn method(func: Function, is_const: bool) -> Method {
  Method {
    func: func,
    is_const: is_const
  }
}

pub struct Class {
  methods: HashMap<&'static [u8], Method>
}

pub fn class() -> Class {
  Class {
    methods: HashMap::new()
  }
}

#[macro_export]
macro_rules! class_methods {
  ($cls:expr => { mutable methods: $( $t:tt )* } ) => ({
    let mut cls = $cls;
    class_methods!(cls, false, $( $t )* )
  });

  ($cls:expr => { const methods: $( $t:tt )* } ) => ({
    let mut cls = $cls;
    class_methods!(cls, true, $( $t )* )
  });

  ($cls:expr , $is_const:expr, ) => (
    $cls
  );

  ($cls:expr , $is_const:expr , mutable methods: $( $t:tt )* ) => (
    class_methods!($cls, false, $( $t )* )
  );

  ($cls:expr , $is_const:expr , const methods: $( $t:tt )* ) => (
    class_methods!($cls, true, $( $t )* )
  );

  ($cls:expr , $is_const:expr , $rtype:expr , $mname:expr ; $( $t:tt )* ) => ({
    $cls.add_method($mname, $is_const, function!($rtype));
    class_methods!($cls, $is_const, $( $t )*)
  });

  ($cls:expr , $is_const:expr , $rtype:expr , $mname:expr , $( $arg:expr ),+ ; $( $t:tt )* ) => ({
    $cls.add_method($mname, $is_const, function!($rtype, $( $arg ),+ ));
    class_methods!($cls, $is_const, $( $t )*)
  });
}

impl Class {
  pub fn add_method(&mut self,
                    name: &'static [u8],
                    is_const: bool,
                    function: Function) {
    self.methods.insert(name, method(function, is_const));
  }

  pub fn generate_cpp(&self,
                      namespace: &Vec<&'static [u8]>,
                      name: &'static [u8],
                      out_stream: &mut io::Write) -> io::Result<()> {
    for (method_name, method_desc) in self.methods.iter() {
      let function_desc = &method_desc.func;

      try!(out_stream.write_all(b"extern \"C\"\n"));

      try!(function_desc.ret.generate_cpp(out_stream));

      try!(out_stream.write_all(b" "));
      for ns_part in namespace.iter() {
        try!(out_stream.write_all(ns_part));
        try!(out_stream.write_all(b"_"));
      }
      try!(out_stream.write_all(name));
      try!(out_stream.write_all(b"_"));
      try!(out_stream.write_all(method_name));
      try!(out_stream.write_all(b"("));

      for ns_part in namespace.iter() {
        try!(out_stream.write_all(ns_part));
        try!(out_stream.write_all(b"::"));
      }
      try!(out_stream.write_all(name));
      if method_desc.is_const {
        try!(out_stream.write_all(b" const"));
      }
      try!(out_stream.write_all(b"*"));

      try!(out_stream.write_all(b" ctx"));

      let arg_len = function_desc.args.len();
      if arg_len > 0 {
        try!(out_stream.write_all(b", "));
        try!(function_desc.args.generate_cpp(out_stream));
      }

      try!(out_stream.write_all(b")"));

      try!(out_stream.write_all(b" {\n  "));
      if !function_desc.ret.is_void() {
        try!(out_stream.write_all(b"return "));
      }
      try!(out_stream.write_all(b"ctx->"));
      try!(out_stream.write_all(method_name));
      try!(out_stream.write_all(b"("));

      for i in (0..arg_len) {
        if i > 0 {
          try!(out_stream.write_all(b", "));
        }
        try!(write!(out_stream, "arg{}", i));
      }

      try!(out_stream.write_all(b");\n"));
      try!(out_stream.write_all(b"}"));
      try!(out_stream.write_all(b"\n\n"));
    }

    Ok(())
  }

  pub fn generate_rs(&self,
                     namespace: &Vec<&'static [u8]>,
                     name: &'static [u8],
                     out_stream: &mut io::Write) -> io::Result<()> {
    try!(out_stream.write_all(
      b"extern {\n"
    ));

    for (method_name, method_desc) in self.methods.iter() {
      let function_desc = &method_desc.func;

      try!(out_stream.write_all(b"  pub fn "));
      for ns_part in namespace.iter() {
        try!(out_stream.write_all(ns_part));
        try!(out_stream.write_all(b"_"));
      }
      try!(out_stream.write_all(name));
      try!(out_stream.write_all(b"_"));
      try!(out_stream.write_all(method_name));
      try!(out_stream.write_all(b"("));

      try!(out_stream.write_all(b"ctx: *"));
      try!(out_stream.write_all(if method_desc.is_const {
        b"const "
      } else {
        b"mut "
      }));
      try!(out_stream.write_all(b"c_void"));

      let arg_len = function_desc.args.len();
      if arg_len > 0 {
        try!(out_stream.write_all(b", "));
        try!(function_desc.args.generate_rs(out_stream));
      }

      try!(out_stream.write_all(b")"));

      if !function_desc.ret.is_void() {
        try!(out_stream.write_all(b" -> "));
        try!(function_desc.ret.generate_rs(out_stream));
      }
      try!(out_stream.write_all(b";\n"));
    }

    try!(out_stream.write_all(b"}\n"));

    Ok(())
  }
}

#[macro_export]
macro_rules! function {
  ($rtype:expr) => ($crate::Function {
    ret: $rtype,
    args: $crate::FunctionArgs::None
  });

  ($rtype:expr, $arg1:expr) => ($crate::Function {
    ret: $rtype,
    args: $crate::FunctionArgs::Args1([$arg1])
  });

  ($rtype:expr, $arg1:expr, $arg2:expr) => ($crate::Function {
    ret: $rtype,
    args: $crate::FunctionArgs::Args2([$arg1, $arg2])
  })
}

#[macro_export]
macro_rules! void_function {
  ( $ ( $ arg : expr ),* ) => (
    function!($crate::proto::void() $ (, $ arg ) * )
  );
}

pub mod proto {
  use std::io;

  pub enum BasicType {
    Simple(CType),
    MutPointer(CType),
    ConstPointer(CType)
  }

  impl BasicType {

    pub fn is_void(&self) -> bool {
      if let &BasicType::Simple(CType::Void) = self {
        return true;
      }

      false
    }

    pub fn generate_cpp(&self, out_stream: &mut io::Write) -> io::Result<()> {
      use self::BasicType::*;

      match self {
        &Simple(ref t)       => t.generate_cpp(out_stream),
        &MutPointer(ref t)   => {
          try!(t.generate_cpp(out_stream));
          out_stream.write_all(b"*")
        },
        &ConstPointer(ref t) => {
          try!(t.generate_cpp(out_stream));
          out_stream.write_all(b" const*")
        }
      } // match self
    } // generate_cpp

    pub fn generate_rs(&self, out_stream: &mut io::Write) -> io::Result<()> {
      use self::BasicType::*;

      match self {
        &Simple(ref t)       => t.generate_rs(out_stream),
        &MutPointer(ref t)   => {
          try!(out_stream.write_all(b"*mut "));
          t.generate_rs(out_stream)
        },
        &ConstPointer(ref t) => {
          try!(out_stream.write_all(b"*const "));
          t.generate_rs(out_stream)
        }
      } // match self
    } // generate_rs
  }
  
  pub enum CType {
    Void,
    UChar,
    UInt,
    SizeT
  }

  pub use self::CType::*;

  impl CType {
    pub fn generate_cpp(&self, out_stream: &mut io::Write) -> io::Result<()> {
      use self::CType::*;

      out_stream.write_all(match self {
        &Void       => b"void",
        &UChar      => b"unsigned char",
        &SizeT      => b"size_t",
        &UInt       => b"unsigned int",
      }) // write_all(match...)
    } // generate_cpp

    pub fn generate_rs(&self, out_stream: &mut io::Write) -> io::Result<()> {
      use self::CType::*;

      out_stream.write_all(match self {
        &Void       => b"c_void",
        &UChar      => b"c_uchar",
        &SizeT      => b"size_t",
        &UInt       => b"c_uint",
      }) // write_all(match...)
    } // generate_rs
  }

  pub fn void() -> BasicType {
    BasicType::Simple(CType::Void)
  }

  pub fn size_t() -> BasicType {
    BasicType::Simple(CType::SizeT)
  }

  pub fn uint() -> BasicType {
    BasicType::Simple(CType::UInt)
  }

  pub fn mut_ptr(t: CType) -> BasicType {
    BasicType::MutPointer(t)
  }

  pub fn const_ptr(t: CType) -> BasicType {
    BasicType::ConstPointer(t)
  }
}
//#define RCPP_NEW(rcpp_t) \
//  extern "C" \
//  rcpp_t * new_ ## rcpp_t () { \
//    return new rcpp_t (); \
//  }
//
