
//use libc::{c_void, c_int, c_char, c_ulong, c_long, c_uint, c_uchar, size_t};
use libc::{c_void, c_uint, c_uchar, size_t};

#[link(name = "rustcryptopp")]
extern {
  pub fn SHA3_256_new() -> *mut c_void;
  pub fn SHA3_256_delete(ctx: *const c_void) -> c_uint;
  pub fn HashTransformation_Update(ctx: *mut c_void,
                                   input: *const c_uchar,
                                   len: size_t);
  pub fn HashTransformation_Final(ctx: *mut c_void,
                                  digest: *const c_uchar);
}

