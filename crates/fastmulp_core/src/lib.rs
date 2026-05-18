//! `fastmulp-core` is a `multipart/form-data` parser tuned for HTTP form uploads.
//!
//! Relevant specifications:
//! - <https://www.rfc-editor.org/rfc/rfc7578#section-4.1>
//! - <https://www.rfc-editor.org/rfc/rfc7578#section-4.2>
//! - <https://www.rfc-editor.org/rfc/rfc2046#section-5.1.1>
//! - <https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#multipart-form-data>
//!
//! For deployed compatibility, the parser accepts `filename*=` parameters even though
//! RFC 7578 says senders `MUST NOT` generate them.
//!
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![deny(clippy::undocumented_unsafe_blocks, unsafe_op_in_unsafe_fn)]

mod boundary;
mod boundary_scan;
mod content_disposition;
mod error;
mod header;
mod parser;
mod part;
mod text;
mod util;

pub use boundary::{Boundary, boundary_from_content_type};
pub use error::{Error, Result};
pub use header::Header;
pub use parser::{Multipart, MultipartParser, ParseLimits, parse, parse_with_limits};
pub use part::Part;
pub use text::TextValue;

#[cfg(test)]
mod tests;
