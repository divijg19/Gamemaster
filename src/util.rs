//! Misc small utilities shared across modules.
use serenity::builder::{CreateActionRow, CreateEmbed};
use std::fmt::{Result as FmtResult, Write};

/// Naive hasher for an embed + components to detect stale cached nav states.
/// This is intentionally lightweight; collisions are acceptable (worst case: unnecessary re-render skipped).
pub fn hash_embed(embed: &CreateEmbed, components: &[CreateActionRow]) -> u64 {
    // serenity's builders don't expose fields directly; rely on Debug formatting.
    // Use a tiny in-place FNV-1a writer to avoid allocating a large intermediate String.
    struct Fnv1a64(u64);
    impl Fnv1a64 {
        #[inline]
        fn new() -> Self {
            Self(0xcbf29ce484222325)
        }
        #[inline]
        fn write_bytes(&mut self, bytes: &[u8]) {
            const FNV_PRIME: u64 = 0x00000100000001B3;
            for &b in bytes {
                self.0 ^= b as u64;
                self.0 = self.0.wrapping_mul(FNV_PRIME);
            }
        }
        #[inline]
        fn finish(self) -> u64 {
            self.0
        }
    }

    struct FnvWrite<'a>(&'a mut Fnv1a64);
    impl<'a> Write for FnvWrite<'a> {
        fn write_str(&mut self, s: &str) -> FmtResult {
            self.0.write_bytes(s.as_bytes());
            Ok(())
        }
    }

    let mut hasher = Fnv1a64::new();
    let mut sink = FnvWrite(&mut hasher);
    let _ = std::fmt::write(&mut sink, format_args!("{:?}{:?}", embed, components));
    hasher.finish()
}
