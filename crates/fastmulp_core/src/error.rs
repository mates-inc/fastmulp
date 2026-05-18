use core::fmt;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    EmptyBoundary,
    BoundaryTooLong { len: usize },
    InvalidBoundaryByte { offset: usize, byte: u8 },
    InvalidStartingBoundary,
    InvalidBoundaryTerminator { offset: usize },
    UnexpectedEnd { offset: usize },
    InvalidHeaderLineEnding { offset: usize },
    InvalidHeaderContinuation { offset: usize },
    MissingHeaderSeparator { offset: usize },
    InvalidHeaderName { offset: usize },
    MissingContentDisposition { offset: usize },
    MissingPartName { offset: usize },
    InvalidContentDisposition { offset: usize },
    MissingClosingBoundary { offset: usize },
    TrailingData { offset: usize },
    PartLimitExceeded { limit: usize },
    HeaderCountLimitExceeded { limit: usize, offset: usize },
    HeaderBytesLimitExceeded { limit: usize, offset: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBoundary => f.write_str("multipart boundary must not be empty"),
            Self::BoundaryTooLong { len } => write!(f, "multipart boundary is too long: {len}"),
            Self::InvalidBoundaryByte { offset, byte } => {
                write!(
                    f,
                    "multipart boundary contains an invalid byte at {offset}: 0x{byte:02x}"
                )
            }
            Self::InvalidStartingBoundary => {
                f.write_str("multipart body does not start with the expected boundary")
            }
            Self::InvalidBoundaryTerminator { offset } => {
                write!(
                    f,
                    "multipart boundary terminator is invalid at byte offset {offset}"
                )
            }
            Self::UnexpectedEnd { offset } => write!(
                f,
                "multipart body ended unexpectedly at byte offset {offset}"
            ),
            Self::InvalidHeaderLineEnding { offset } => {
                write!(
                    f,
                    "multipart header line must end with CRLF at byte offset {offset}"
                )
            }
            Self::InvalidHeaderContinuation { offset } => {
                write!(
                    f,
                    "multipart header continuation is not supported at byte offset {offset}"
                )
            }
            Self::MissingHeaderSeparator { offset } => {
                write!(f, "multipart header is missing ':' at byte offset {offset}")
            }
            Self::InvalidHeaderName { offset } => write!(
                f,
                "multipart header name is invalid at byte offset {offset}"
            ),
            Self::MissingContentDisposition { offset } => {
                write!(
                    f,
                    "multipart part is missing Content-Disposition at byte offset {offset}"
                )
            }
            Self::MissingPartName { offset } => {
                write!(
                    f,
                    "multipart part is missing the Content-Disposition name parameter at byte offset {offset}"
                )
            }
            Self::InvalidContentDisposition { offset } => {
                write!(f, "Content-Disposition is invalid at byte offset {offset}")
            }
            Self::MissingClosingBoundary { offset } => {
                write!(
                    f,
                    "multipart closing boundary was not found after byte offset {offset}"
                )
            }
            Self::TrailingData { offset } => write!(
                f,
                "multipart body has trailing data after byte offset {offset}"
            ),
            Self::PartLimitExceeded { limit } => {
                write!(f, "multipart part count exceeded configured limit {limit}")
            }
            Self::HeaderCountLimitExceeded { limit, offset } => write!(
                f,
                "multipart header count exceeded configured limit {limit} at byte offset {offset}"
            ),
            Self::HeaderBytesLimitExceeded { limit, offset } => write!(
                f,
                "multipart header bytes exceeded configured limit {limit} at byte offset {offset}"
            ),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = core::result::Result<T, Error>;
