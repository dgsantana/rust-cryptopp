use std::collections::hash_map::HashMap;
use std::io;
use std::io::Write;
use std::convert::From;
use std::error;
use std::fs::File;
use std::borrow::Borrow;

#[derive(Debug)]
pub enum Error {
  IO(io::Error),
  Unexpected(String)
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<io::Error> for Error {
  fn from(e: io::Error) -> Error {
    Error::IO(e)
  }
}

impl From<std::string::FromUtf8Error> for Error {
  fn from(e: std::string::FromUtf8Error) -> Error {
    let desc = error::Error::description(&e);
    Error::Unexpected(String::from(desc))
  }
}

pub fn generate_prelude<T: Write>(mut stream: T) -> Result<()> {
  try!(stream.write_all(b"
    pub trait CPPContext {
      fn mut_ctx(&self) -> *mut c_void;
      fn ctx(&self) -> *const c_void {
        self.mut_ctx()
      }
    }
  \n\n"));

  Ok(())
}

pub struct Context<T, U> {
  pub cpp_stream: T,
  pub rs_binding_stream: U
}

impl<T: Write, U: Write> Context<T, U> {
  pub fn new(cpp_stream: T,
             rs_binding_stream: U) -> Context<T, U> {
    Context {
      cpp_stream: cpp_stream,
      rs_binding_stream: rs_binding_stream
    }
  }
}

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

  pub fn generate_proto_cpp(&self, out_stream: &mut Write) -> Result<()> {
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

  pub fn generate_apply_cpp(&self, out_stream: &mut Write) -> Result<()> {
    if let Some(slice) = self.as_slice() {
      let mut i = 0u32;

      for btype in slice.iter() {
        if i > 0 {
          try!(out_stream.write_all(b", "));
        }

        try!(out_stream.write_all(b" "));
        if btype.is_ref() {
          try!(out_stream.write_all(b"*"));
        }

        try!(write!(out_stream, "arg{}", i));
        i += 1;
      }
    }

    Ok(())
  }

  pub fn generate_proto_rs(&self, out_stream: &mut Write) -> Result<()> {
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

#[macro_export]
macro_rules! class {
  ( $name:expr => $b:tt ) => ({
    let anon_class = class_bindings_block!($crate::class() => $b);
    $crate::NamedClass::new(
      vec![],
      $name,
      anon_class
    )
  });

  ( $ns:expr, $name:expr => $b:tt ) => ({
    let anon_class = class_bindings_block!($crate::class() => $b);
    $crate::NamedClass::new(
      $ns,
      $name,
      anon_class
    )
  });

  ( $name:expr , $anon_class:expr ) => ({
    $crate::NamedClass::new(
      vec![],
      $name,
      $anon_class
    )
  });

  ( $ns:expr, $name:expr , $anon_class:expr ) => ({
    $crate::NamedClass::new(
      $ns,
      $name,
      $anon_class
    )
  });

}

#[macro_export]
macro_rules! prototype_class {
  ( $b:tt ) => (
    class_bindings_block!($crate::class() => $b)
  );
}

pub struct NamedClass<'a, T> {
  pub namespace:  Vec<&'a [u8]>,
  pub name:       &'a [u8],
  pub anon_class: T,
}

impl<'a, T: Borrow<Class>> NamedClass<'a, T> {
  pub fn new(namespace:  Vec<&'a [u8]>,
             name:       &'a [u8],
             anon_class: T) -> NamedClass<'a, T> {
    NamedClass {
      namespace: namespace,
      name: name,
      anon_class: anon_class,
    }
  }

  pub fn c_path(&self) -> Result<String> {
    let mut c_path: Vec<u8> = Vec::new();
    try!(generate_c_path(&self.namespace, self.name, &mut c_path));
    let s = try!(String::from_utf8(c_path));

    Ok(s)
  }

  fn fstream_from_c_path(&self, filepath: &std::path::Path) -> Result<File> {
    let mut fname = try!(self.c_path());
    fname.push_str(".rs");
    let stream = try!(File::create(filepath.join(fname)));

    Ok(stream)
  }

  pub fn generate_struct(&self,
                         filepath: &std::path::Path,
                         name: &'a [u8]) -> Result<()> {
    let mut stream = try!(self.fstream_from_c_path(filepath));
    self.write_struct(name, &mut stream)
  }

  pub fn write_struct<U: Write>(&self, name: &'a [u8], mut stream: U) -> Result<()> {
    try!(stream.write_all(b"use std;\n\n"));

    try!(stream.write_all(b"pub struct "));
    try!(stream.write_all(name));
    try!(stream.write_all(b" {\n  ctx: *mut c_void\n}\n"));

    try!(stream.write_all(b"impl Drop for "));
    try!(stream.write_all(name));
    try!(stream.write_all(b" {\n  fn drop(&mut self) {\n"));
    try!(stream.write_all(b"    unsafe { cpp::del_"));
    try!(generate_c_path(&self.namespace, self.name, &mut stream));
    try!(stream.write_all(b"(self.ctx) };\n  }\n}\n"));

    try!(stream.write_all(b"impl "));
    try!(stream.write_all(name));
    try!(stream.write_all(b" {\n  pub fn new() -> "));
    try!(stream.write_all(name));
    try!(stream.write_all(b" {\n    let ctx = unsafe { cpp::new_"));
    try!(generate_c_path(&self.namespace, self.name, &mut stream));
    try!(stream.write_all(b"() };\n\n    "));
    try!(stream.write_all(name));
    try!(stream.write_all(b" { ctx: ctx }\n  }\n}\n\n"));

    try!(stream.write_all(b"impl cpp::CPPContext for "));
    try!(stream.write_all(name));
    try!(stream.write_all(b" {\n  fn mut_ctx(&self) -> *mut c_void { self.ctx }\n}"));
    try!(stream.write_all(b"\n\n"));

    try!(stream.write_all(b"impl std::default::Default for "));
    try!(stream.write_all(name));
    try!(stream.write_all(b" {\n  fn default() -> "));
    try!(stream.write_all(name));
    try!(stream.write_all(b" { "));
    try!(stream.write_all(name));
    try!(stream.write_all(b"::new() }\n}"));
    try!(stream.write_all(b"\n\n"));

    Ok(())
  }

//  pub fn generate_trait(&self,
//                        filepath: &std::path::Path,
//                        name: &'a [u8]) -> Result<()> {
//    let mut stream = try!(self.fstream_from_c_path(filepath));
//    self.write_trait(name, &mut stream)
//  }
//
//  pub fn write_trait<T: Write>(&self, name: &'a [u8], mut stream: T) -> Result<()> {
//    unimplemented!()
//  }

  pub fn generate_bindings<U: Write, V: Write>(&self, context: &mut Context<U, V>)
      -> Result<()> {
    self.anon_class.borrow().generate(&self.namespace,
                             self.name,
                             &mut context.cpp_stream,
                             &mut context.rs_binding_stream)
  }
}

#[macro_export]
macro_rules! class_bindings_block {
  ($cls:expr => { $( $t:tt )* } ) => ({
    let mut cls = $cls;
    class_bindings!(cls, $( $t )*)
  })
}

#[macro_export]
macro_rules! class_bindings {
  ($cls:expr, ) => (
    $cls
  );

  ($cls:expr, mutable methods { $( $t:tt )* }  $( $rest:tt )* ) => ({
    let mut cls = class_methods!($cls, false, $( $t )*);
    class_bindings!(cls, $( $rest )* )
  });

  ($cls:expr, constant methods { $( $t:tt )* }  $( $rest:tt )* ) => ({
    let mut cls = class_methods!($cls, true, $( $t )*);
    class_bindings!(cls, $( $rest )* )
  });

  ($cls:expr, constructors { $( $t:tt )* }  $( $rest:tt )* ) => ({
    let mut cls = class_ctors!($cls, $( $t )*);
    class_bindings!(cls, $( $rest )* )
  });
}

#[macro_export]
macro_rules! class_methods {
  ($cls:expr , $is_const:expr , ) => (
    $cls
  );

  ($cls:expr , $is_const:expr , $rtype:expr , $mname:expr ; $( $rest:tt )* ) => ({
    $cls.add_method($mname, $is_const, function!($rtype) );
    class_methods!($cls, $is_const, $( $rest )* )
  });

  ($cls:expr , $is_const:expr , $rtype:expr , $mname:expr, $( $args:expr ),+ ; $( $rest:tt )* ) => ({
    $cls.add_method($mname, $is_const, function!($rtype, $( $args ),+ ) );
    class_methods!($cls, $is_const, $( $rest )* )
  });
}

#[macro_export]
macro_rules! class_ctors {
  ($cls:expr , ) => (
    $cls
  );

  ($cls:expr , $mname:expr ; $( $rest:tt )* ) => ({
    $cls.add_constructor($mname, function_args!() );
    class_ctors!($cls, $( $rest )* )
  });

  ($cls:expr , $mname:expr , $( $args:expr ),+ ; $( $rest:tt )* ) => ({
    $cls.add_constructor($mname, function_args!($( $args ),+ ) );
    class_ctors!($cls, $( $rest )* )
  });
}

pub struct Class {
  methods: HashMap<&'static [u8], Method>,
  ctors: HashMap<&'static [u8], FunctionArgs>
}

pub fn class() -> Class {
  Class {
    methods: HashMap::new(),
    ctors:   HashMap::new()
  }
}

fn c_path_ns_part(ns_part: &[u8], out: &mut Write) -> Result<()> {
  for b in ns_part {
    if *b == 0x5f {
      try!(out.write_all(b"__"));
    } else {
      try!(out.write_all(&[*b]));
    }
  }

  Ok(())
}

fn generate_c_path(namespace: &Vec<&[u8]>,
                   name: &[u8],
                   out: &mut Write) -> Result<()> {
  for ns_part in namespace.iter() {
    try!(c_path_ns_part(ns_part, out));
    try!(out.write_all(b"_"));
  }
  try!(out.write_all(name));

  Ok(())
}

fn generate_cpp_path(namespace: &Vec<&[u8]>,
                    name: &[u8],
                    out: &mut Write) -> Result<()> {
  for ns_part in namespace.iter() {
    try!(out.write_all(ns_part));
    try!(out.write_all(b"::"));
  }
  try!(out.write_all(name));

  Ok(())
}

impl Class {
  pub fn add_method(&mut self,
                    name: &'static [u8],
                    is_const: bool,
                    function: Function) {
    self.methods.insert(name, method(function, is_const));
  }

  pub fn add_constructor(&mut self, name: &'static [u8], args: FunctionArgs) {
    self.ctors.insert(name, args);
  }

  pub fn generate_cpp(&self,
                      namespace: &Vec<&[u8]>,
                      name: &[u8],
                      out_stream: &mut Write) -> Result<()> {
    try!(self.generate_cpp_ctors(namespace, name, out_stream));
    self.generate_cpp_methods(namespace, name, out_stream)
  }

  fn generate_cpp_ctors(&self,
                        namespace: &Vec<&[u8]>,
                        name: &[u8],
                        out_stream: &mut Write) -> Result<()> {
    for (ctor_name, ctor_args) in self.ctors.iter() {
      try!(out_stream.write_all(b"extern \"C\"\n"));

      try!(generate_cpp_path(namespace, name, out_stream));
      try!(out_stream.write_all(b" *"));

      try!(out_stream.write_all(b" new_"));
      if ctor_name.len() > 0 {
        try!(out_stream.write_all(ctor_name));
        try!(out_stream.write_all(b"_"));
      }
      try!(generate_c_path(namespace, name, out_stream));

      try!(out_stream.write_all(b"("));

      if ctor_args.len() > 0 {
        try!(ctor_args.generate_proto_cpp(out_stream));
      }

      try!(out_stream.write_all(b")"));

      try!(out_stream.write_all(b" {\n  "));
      try!(out_stream.write_all(b"return new "));

      try!(generate_cpp_path(namespace, name, out_stream));

      try!(out_stream.write_all(b"("));

      try!(ctor_args.generate_apply_cpp(out_stream));

      try!(out_stream.write_all(b");\n"));
      try!(out_stream.write_all(b"}"));
      try!(out_stream.write_all(b"\n\n"));
    }

    try!(out_stream.write_all(b"extern \"C\"\n"));
    try!(out_stream.write_all(b"void del_"));

    try!(generate_c_path(namespace, name, out_stream));
    try!(out_stream.write_all(b"("));

    try!(generate_cpp_path(namespace, name, out_stream));
    try!(out_stream.write_all(b"*"));
    try!(out_stream.write_all(b" ctx"));
    try!(out_stream.write_all(b")"));
    try!(out_stream.write_all(b" {\n  delete ctx;\n}\n\n"));

    Ok(())
  }

  fn generate_cpp_methods(&self,
                          namespace: &Vec<&[u8]>,
                          name: &[u8],
                          out_stream: &mut Write) -> Result<()> {
    for (method_name, method_desc) in self.methods.iter() {
      let function_desc = &method_desc.func;

      try!(out_stream.write_all(b"extern \"C\"\n"));

      try!(function_desc.ret.generate_cpp(out_stream));

      try!(out_stream.write_all(b" mth_"));
      try!(generate_c_path(namespace, name, out_stream));
      try!(out_stream.write_all(b"_"));
      try!(out_stream.write_all(method_name));
      try!(out_stream.write_all(b"("));

      try!(generate_cpp_path(namespace, name, out_stream));
      if method_desc.is_const {
        try!(out_stream.write_all(b" const"));
      }
      try!(out_stream.write_all(b"*"));
      try!(out_stream.write_all(b" ctx"));

      if function_desc.args.len() > 0 {
        try!(out_stream.write_all(b", "));
        try!(function_desc.args.generate_proto_cpp(out_stream));
      }
      try!(out_stream.write_all(b")"));

      try!(out_stream.write_all(b" {\n  "));
      if !function_desc.ret.is_void() {
        try!(out_stream.write_all(b"return "));
      }
      try!(out_stream.write_all(b"ctx->"));
      try!(out_stream.write_all(method_name));
      try!(out_stream.write_all(b"("));

      try!(function_desc.args.generate_apply_cpp(out_stream));

      try!(out_stream.write_all(b");\n"));
      try!(out_stream.write_all(b"}"));
      try!(out_stream.write_all(b"\n\n"));
    }

    Ok(())
  }

  pub fn generate_rs(&self,
                     namespace: &Vec<&[u8]>,
                     name: &[u8],
                     out_stream: &mut Write) -> Result<()> {
    try!(out_stream.write_all(
      b"extern {\n"
    ));

    try!(self.generate_rs_methods(namespace, name, out_stream));
    try!(self.generate_rs_ctors(namespace, name, out_stream));

    try!(out_stream.write_all(b"}\n"));

    Ok(())
  }

  fn generate_rs_ctors(&self,
                       namespace: &Vec<&[u8]>,
                       name: &[u8],
                       out_stream: &mut Write) -> Result<()> {
    if self.ctors.len() < 1 {
      return Ok(())
    }

    for (ctor_name, ctor_args) in self.ctors.iter() {
      try!(out_stream.write_all(b"  pub fn new_"));
      if ctor_name.len() > 0 {
        try!(out_stream.write_all(ctor_name));
        try!(out_stream.write_all(b"_"));
      }
      try!(generate_c_path(namespace, name, out_stream));

      try!(out_stream.write_all(b"("));

      if ctor_args.len() > 0 {
        try!(ctor_args.generate_proto_rs(out_stream));
      }

      try!(out_stream.write_all(b")"));
      try!(out_stream.write_all(b" -> *mut c_void"));
      try!(out_stream.write_all(b";\n"));
    }

    try!(out_stream.write_all(b"  pub fn del_"));
    try!(generate_c_path(namespace, name, out_stream));
    try!(out_stream.write_all(b"(ctx: *mut c_void);\n"));

    Ok(())
  }

  fn generate_rs_methods(&self,
                         namespace: &Vec<&[u8]>,
                         name: &[u8],
                         out_stream: &mut Write) -> Result<()> {
    for (method_name, method_desc) in self.methods.iter() {
      let function_desc = &method_desc.func;

      try!(out_stream.write_all(b"  pub fn mth_"));

      try!(generate_c_path(namespace, name, out_stream));
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
        try!(function_desc.args.generate_proto_rs(out_stream));
      }

      try!(out_stream.write_all(b")"));

      if !function_desc.ret.is_void() {
        try!(out_stream.write_all(b" -> "));
        try!(function_desc.ret.generate_rs(out_stream));
      }
      try!(out_stream.write_all(b";\n"));
    }

    Ok(())
  }

  pub fn generate(&self,
              namespace: &Vec<&[u8]>,
              name: &[u8],
              cpp_stream: &mut Write,
              rs_stream: &mut Write) -> Result<()> {
    try!(self.generate_cpp(&namespace, name, cpp_stream));
    self.generate_rs(&namespace, name, rs_stream)
  }
}

#[macro_export]
macro_rules! function_args {
  () => (
    $crate::FunctionArgs::None
  );

  ($arg1:expr) => (
    $crate::FunctionArgs::Args1([$arg1])
  );

  ($arg1:expr, $arg2:expr) => (
    $crate::FunctionArgs::Args2([$arg1, $arg2])
  );
}

#[macro_export]
macro_rules! function {
  ($rtype:expr) => ($crate::Function {
    ret: $rtype,
    args: function_args!()
  });

  ($rtype:expr, $( $args:expr ),+ ) => ($crate::Function {
    ret: $rtype,
    args: function_args!($( $args ),+)
  });
}

#[macro_export]
macro_rules! void_function {
  ( $ ( $ arg : expr ),* ) => (
    function!($crate::proto::void() $ (, $ arg ) * )
  );
}

pub mod proto {
  use std::io;
  use std::io::Write;

  pub enum BasicType {
    Simple(CType),
    MutPointer(CType),
    ConstPointer(CType),
    MutRef(CType),
    ConstRef(CType)
  }

  impl BasicType {

    pub fn is_void(&self) -> bool {
      if let &BasicType::Simple(CType::Void) = self {
        return true;
      }

      false
    }

    pub fn is_ref(&self) -> bool {
      use self::BasicType::*;
      match self {
        &MutRef(_)   |
        &ConstRef(_) => true,
        _            => false
      }
    }

    pub fn generate_cpp(&self, out_stream: &mut Write) -> io::Result<()> {
      use self::BasicType::*;

      match self {
        &Simple(ref t)       => t.generate_cpp(out_stream),
        &MutPointer(ref t)   |
        &MutRef(ref t)       => {
          try!(t.generate_cpp(out_stream));
          out_stream.write_all(b"*")
        },
        &ConstPointer(ref t) |
        &ConstRef(ref t)     => {
          try!(t.generate_cpp(out_stream));
          out_stream.write_all(b" const*")
        }
      } // match self
    } // generate_cpp

    pub fn generate_rs(&self, out_stream: &mut Write) -> io::Result<()> {
      use self::BasicType::*;

      match self {
        &Simple(ref t)       => t.generate_rs(out_stream),
        &MutPointer(ref t)   |
        &MutRef(ref t)       => {
          try!(out_stream.write_all(b"*mut "));
          t.generate_rs(out_stream)
        },
        &ConstPointer(ref t) |
        &ConstRef(ref t)     => {
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
    SizeT,
    Long,
    Custom(&'static [u8])
  }

  pub use self::CType::*;

  impl CType {
    pub fn generate_cpp(&self, out_stream: &mut Write) -> io::Result<()> {
      use self::CType::*;

      out_stream.write_all(match self {
        &Void          => b"void",
        &UChar         => b"unsigned char",
        &SizeT         => b"size_t",
        &UInt          => b"unsigned int",
        &Long          => b"long",
        &Custom(ref s) => s,
      }) // write_all(match...)
    } // generate_cpp

    pub fn generate_rs(&self, out_stream: &mut Write) -> io::Result<()> {
      use self::CType::*;

      out_stream.write_all(match self {
        &Void       |
        &Custom(_)  => b"c_void",
        &UChar      => b"c_uchar",
        &SizeT      => b"size_t",
        &UInt       => b"c_uint",
        &Long       => b"c_long",
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

  pub fn long() -> BasicType {
    BasicType::Simple(CType::Long)
  }

  pub fn mut_ptr(t: CType) -> BasicType {
    BasicType::MutPointer(t)
  }

  pub fn const_ptr(t: CType) -> BasicType {
    BasicType::ConstPointer(t)
  }

  pub fn const_ref(t: CType) -> BasicType {
    BasicType::ConstRef(t)
  }
}
//#define RCPP_NEW(rcpp_t) \
//  extern "C" \
//  rcpp_t * new_ ## rcpp_t () { \
//    return new rcpp_t (); \
//  }
//
