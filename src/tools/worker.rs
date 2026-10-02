//! Internal worker control contracts, not backend or execution authority.
//! Framing owns bounded transport records; exchange owns outer-envelope identity
//! and supervisor-side sequencing. OS channels and process lifecycle are separate.
mod exchange;
mod framing;
pub use exchange::{ToolWorkerExchange, ToolWorkerOperation, ToolWorkerOutcome};
use framing::parse;
pub use framing::{
    split_tool_worker_channel, ToolWorkerChannel, ToolWorkerReceiver, ToolWorkerSender,
};
