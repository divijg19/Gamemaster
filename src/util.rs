//! Misc small utilities shared across modules.
use serenity::builder::{CreateActionRow, CreateEmbed};
use std::hash::Hasher;

/// Naive hasher for an embed + components to detect stale cached nav states.
/// This is intentionally lightweight; collisions are acceptable (worst case: unnecessary re-render skipped).
pub fn hash_embed(embed: &CreateEmbed, components: &Vec<CreateActionRow>) -> u64 {
    // serenity's builders don't expose fields directly; rely on Debug formatting.
    let mut hasher = ahash::AHasher::default();
    std::fmt::write(
        &mut hasher_adapter::FmtWriteHasher(&mut hasher),
        format_args!("{:?}{:?}", embed, components),
    )
    .ok();
    hasher.finish()
}

mod hasher_adapter {
    use std::fmt::{Result, Write};
    use std::hash::Hasher;
    pub struct FmtWriteHasher<'a, H: Hasher>(pub &'a mut H);
    impl<'a, H: Hasher> Write for FmtWriteHasher<'a, H> {
        fn write_str(&mut self, s: &str) -> Result {
            self.0.write(s.as_bytes());
            Ok(())
        }
    }
}
