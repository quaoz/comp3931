//! FNV-1a hasher for integer-keyed hash sets.
use std::hash::{BuildHasherDefault, Hasher};

const OFFSET: u64 = 0xcbf29ce484222325;
const PRIME: u64 = 0x00000100000001B3;

/// Convenience type alias for use with `HashSet` and `HashMap`.
pub type FnvBuildHasher = BuildHasherDefault<FnvHasher>;

/// FNV-1a hasher state. Initialised with the FNV offset basis.
#[derive(Clone)]
pub struct FnvHasher(u64);

impl Default for FnvHasher {
    fn default() -> Self {
        Self(OFFSET)
    }
}

impl Hasher for FnvHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(PRIME);
        }
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}
