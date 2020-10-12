pub mod event_out_last;
pub mod state_in;
pub mod state_out;

use crate::web::{sse_aggregated, uri_cursor};

pub trait Base: uri_cursor::Handler + sse_aggregated::NodeProvider {}
