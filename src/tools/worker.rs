//! Internal worker control contracts, not backend or execution authority.
//! Framing owns bounded transport records; exchange owns outer-envelope identity
//! and supervisor-side sequencing. OS channels and process lifecycle are separate.
mod channel;
mod exchange;
mod framing;
mod io;
mod session;
#[cfg(windows)]
mod windows_job;
pub use channel::{
    native_tool_worker_channel, NativeToolWorkerEndpoint, NativeToolWorkerReader,
    NativeToolWorkerWriter,
};
pub use exchange::{
    ToolWorkerExchange, ToolWorkerOperation, ToolWorkerOutcome, ToolWorkerRequestContext,
};
use framing::parse;
pub use framing::{
    split_tool_worker_channel, ToolWorkerChannel, ToolWorkerReceiver, ToolWorkerSender,
};
pub use io::NativeToolWorkerIo;
pub use session::ToolWorkerSession;
#[cfg(windows)]
pub use windows_job::WindowsToolWorkerJob;
