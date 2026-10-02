//! Sanitized transport outcomes. Never retain URLs, response bodies or secrets
//! in errors crossing from the host broker to package-manager runtimes.
use std::fmt;

#[derive(Clone, Copy, Debug)]
pub(super) enum Failure {
    InvalidRequest,
    SourceDenied,
    InvalidRedirect,
    RedirectLimit,
    RequestLimit,
    ByteLimit,
    Transport,
    Timeout,
}

impl Failure {
    pub(super) fn status(self) -> u16 {
        match self {
            Self::InvalidRequest => 400,
            Self::SourceDenied => 403,
            Self::ByteLimit => 413,
            Self::RequestLimit => 429,
            Self::Timeout => 504,
            Self::InvalidRedirect | Self::RedirectLimit | Self::Transport => 502,
        }
    }

    pub(super) fn code(self) -> &'static str {
        match self {
            Self::InvalidRequest => "REQUEST_INVALID",
            Self::SourceDenied => "SOURCE_DENIED",
            Self::InvalidRedirect => "REDIRECT_INVALID",
            Self::RedirectLimit => "REDIRECT_LIMIT",
            Self::RequestLimit => "REQUEST_LIMIT",
            Self::ByteLimit => "BYTE_LIMIT",
            Self::Transport => "SOURCE_UNAVAILABLE",
            Self::Timeout => "SOURCE_TIMEOUT",
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: scoped acquisition failed", self.code())
    }
}

impl std::error::Error for Failure {}
