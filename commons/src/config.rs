//! Configuration lookup, parsing and validation.
//!
//! This module provides helpers for sourcing configuration options from
//! multiple inputs, merging, and validating them.

#[macro_export]
macro_rules! assign_if_some {
    ( $dst:expr, $src:expr ) => {{
        if let Some(x) = $src {
            $dst = x.into();
        };
    }};
}

/// Merge configuration options into runtime settings.
///
/// This consumes a generic configuration object, merging its options
/// into runtime settings. It only overlays populated values from config,
/// leaving unset ones preserved as-is from existing settings.
pub trait MergeOptions<T> {
    /// MergeOptions values from `options` into current settings.
    fn merge(&mut self, options: T);
}
