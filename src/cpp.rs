
//use libc::{c_void, c_int, c_char, c_ulong, c_long, c_uint, c_uchar, size_t};
use libc::{c_void, c_uint, c_uchar, c_long, size_t};

#[link(name = "rustcryptopp")]
extern {
  pub fn new_SHA3_256() -> *mut c_void;
  pub fn delete_SHA3_256(ctx: *mut c_void) -> c_uint;
  pub fn HashTransformation_Update(ctx: *mut c_void,
                                   input: *const c_uchar,
                                   len: size_t);
  pub fn HashTransformation_Final(ctx: *mut c_void,
                                  digest: *const c_uchar);

  pub fn new_Integer() -> *mut c_void;
  pub fn delete_Integer(ctx: *mut c_void) -> *mut c_void;
  pub fn copy_Integer(ctx: *const c_void) -> *mut c_void;
  pub fn new_from_long_Integer(val: c_long) -> *mut c_void;
}

