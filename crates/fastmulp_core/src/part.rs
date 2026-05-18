use core::ops::Range;

use smallvec::SmallVec;

use crate::{Header, TextValue};

#[derive(Debug)]
pub struct Part<'a> {
  headers: SmallVec<[Header<'a>; 4]>,
  body_start: usize,
  body_end: usize,
  name: Option<TextValue<'a>>,
  file_name: Option<TextValue<'a>>,
  content_type: Option<&'a [u8]>,
}

impl<'a> Part<'a> {
  pub(crate) fn new(
    headers: SmallVec<[Header<'a>; 4]>,
    body_start: usize,
    body_end: usize,
    name: Option<TextValue<'a>>,
    file_name: Option<TextValue<'a>>,
    content_type: Option<&'a [u8]>,
  ) -> Self {
    Self {
      headers,
      body_start,
      body_end,
      name,
      file_name,
      content_type,
    }
  }

  pub fn headers(&self) -> &[Header<'a>] {
    &self.headers
  }

  /// Returns the byte range of this part's body within the original multipart
  /// body.
  pub fn body_range(&self) -> Range<usize> {
    self.body_start..self.body_end
  }

  /// Returns this part's body by slicing the original multipart body.
  ///
  /// `source` is expected to be the same multipart body that was parsed to
  /// create this `Part`.
  ///
  /// # Panics
  ///
  /// Panics if `source` does not contain this part's stored body range. Use
  /// [`Part::try_body`] when accepting a defensive `None` is preferable.
  pub fn body<'b>(&self, source: &'b [u8]) -> &'b [u8] {
    &source[self.body_start..self.body_end]
  }

  /// Returns this part's body when `source` contains the stored body range.
  ///
  /// This is the defensive alternative to [`Part::body`]. It returns `None`
  /// instead of panicking when the provided source is too short or otherwise
  /// cannot cover the range recorded for this part. Like [`Part::body`], it is
  /// intended to be called with the original multipart body.
  pub fn try_body<'b>(&self, source: &'b [u8]) -> Option<&'b [u8]> {
    source.get(self.body_start..self.body_end)
  }

  pub fn name(&self) -> Option<&TextValue<'a>> {
    self.name.as_ref()
  }

  pub fn file_name(&self) -> Option<&TextValue<'a>> {
    self.file_name.as_ref()
  }

  pub fn content_type(&self) -> Option<&'a [u8]> {
    self.content_type
  }
}
