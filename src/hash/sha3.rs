use hash::{Transformation,
           Function32};

macro_rules! hash_impl_body {
  ($tname:ident, $( $trt:ident ),+) => (
    use hash::{ $( $trt ),+ };

    $(
      impl $trt for $tname {}
    )+

    #[cfg(test)]
    mod digest_tests {
      use hash::Transformation;
      use hash::{ $( $trt ),+ };
      define_Digest_tests!($tname);
    }
  )
}

macro_rules! hash_impls {
  (28, $tname:ident) => (
    hash_impl_body!($tname, Function28, Digest28);
  );

  (32, $tname:ident) => (
    hash_impl_body!($tname, Function32, Digest32);
  );

  (48, $tname:ident) => (
    use hash::{Function48, Digest48};
    impl Function48 for $tname {}
    impl Digest48 for $tname {}
  );

  (64, $tname:ident) => (
    use hash::{Function64, Digest64};
    impl Function64 for $tname {}
    impl Digest64 for $tname {}
  );
}

macro_rules! token_to_expr {
  ($tok:expr) => ($tok)
}

macro_rules! define_sha3 {
  ($file:expr, $modname:ident, $tname:ident, $hsize:tt) => (
    pub mod $modname {
      use cpp;
      use libc::{c_void};
      use hash::Transformation;

      include!(concat!(env!("OUT_DIR"), $file));

      impl Transformation for $tname {}

      hash_impls!($hsize, $tname);

      pub fn new() -> $tname {
        $tname::new()
      }

      pub fn digest(msg: &[u8]) -> [u8; token_to_expr!($hsize)] {
        $tname::digest(msg)
      }

      pub fn empty_digest() -> [u8; token_to_expr!($hsize)] {
        $tname::empty_digest()
      }

    }
  )
}

define_sha3!("/SHA3_224.rs", h224, H224, 28);
define_sha3!("/SHA3_256.rs", h256, H256, 32);
define_sha3!("/SHA3_384.rs", h384, H384, 48);
define_sha3!("/SHA3_512.rs", h512, H512, 64);

/// produce a keccak hmac; that is, an hmac done insecurely but its
/// ok because keccak is crazy like that.
/// http://en.wikipedia.org/wiki/Hash-based_message_authentication_code
/// the hmac is: `keccak(secret || msg)`
/// maybe dangerous?
///
/// # Panics
/// - when secret.len() < 16
/// - when msg is empty
pub fn keccak_mac(secret: &[u8], msg: &[u8]) -> [u8; 32] {
  assert!(secret.len() >= 16, "secret is dangerously small");
  assert!(msg.len() > 0, "msg is empty");

  let mut keccak = h256::new();

  keccak.update(secret);
  keccak.update(msg);
  keccak.final_digest()
}

#[cfg(test)]
mod test {
  use hash::DigestSize;
  use hash::Transformation;
  use hash::Function32;

  #[test]
  fn sanity() {
    let mut h256 = super::h256::new();
    let msg      = b"abc";
    let expected = [0x4e, 0x03, 0x65, 0x7a, 0xea, 0x45, 0xa9, 0x4f,
                    0xc7, 0xd4, 0x7b, 0xa8, 0x26, 0xc8, 0xd6, 0x67,
                    0xc0, 0xd1, 0xe6, 0xe3, 0x3a, 0x64, 0xa0, 0x36,
                    0xec, 0x44, 0xf5, 0x8f, 0xa1, 0x2d, 0x6c, 0x45];

    assert_eq!(h256.size(), DigestSize::Bits256);

    h256.update(msg);
    assert_eq!(h256.final_digest(), expected);

    assert_eq!(super::h256::digest(msg), expected);

    let msg2 = b"uchk uchk chk uchk ucka chka chuk";
    let expected2 = [0xc5, 0x1a, 0x46, 0x40, 0x69, 0x4d, 0x14, 0x91,
                     0x6a, 0x82, 0xdd, 0xd6, 0x66, 0xd4, 0xea, 0x63,
                     0x15, 0x87, 0x45, 0xed, 0x99, 0xe6, 0xca, 0xd1,
                     0x33, 0x1f, 0x39, 0xc5, 0x7e, 0x3a, 0xbe, 0x37];

    assert_eq!(super::h256::digest(msg2), expected2);

    assert_eq!(super::keccak_mac(&msg2[..16], &msg2[16..]), expected2);
  }


  #[test]
  fn digest_empty_digest() {
    let empty_hash = [0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c,
                      0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7, 0x03, 0xc0,
                      0xe5, 0x00, 0xb6, 0x53, 0xca, 0x82, 0x27, 0x3b,
                      0x7b, 0xfa, 0xd8, 0x04, 0x5d, 0x85, 0xa4, 0x70];
    assert_eq!(super::h256::empty_digest(), empty_hash);
  }

  mod keccak_mac {
    #[test]
    #[should_panic]
    fn panic_secret() {
      super::super::keccak_mac(b"too short", b"msg");
    }

    #[test]
    #[should_panic]
    fn panic_msg() {
      super::super::keccak_mac(b"0123456789012345", b"");
    }
  }
}
