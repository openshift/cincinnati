//! Tracing stub — distributed tracing has been removed.
//!
//! The --service.tracing_endpoint flag is accepted but ignored for
//! backward compatibility with existing deployments.

use crate::prelude_errors::*;

/// Initialize tracing. The endpoint argument is accepted for backward compatibility but ignored.
pub fn init_tracer(_name: &'static str, _maybe_endpoint: Option<String>) -> Fallible<()> {
    Ok(())
}
