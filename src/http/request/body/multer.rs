use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;
use bytes::Buf;

// use twoway::find_bytes;
use crate::http::request::MultiPartFormDataField;
use crate::server::connection::BodyReadingBuffer;
use crate::util::{found_boundary_in, PatternExistResult};

use super::{H1StreamHolder, H2StreamHolder};
use super::FormDataAll;

pub (crate) enum MultipartStreamHolder<'a> {
    H1(H1StreamHolder<'a>),
    H2(H2StreamHolder<'a>),
}

/// For handling multipart form data in both HTTP/1 and HTTP/2 protocols
pub struct MultipartData<'a> {
    stream_holder: MultipartStreamHolder<'a>,
    reading_buffer: &'a mut BodyReadingBuffer,
    _boundary: Cow<'a, str>,
    /// Pre-computed delimiter bytes: always `"--" + boundary`, per RFC 2046.
    /// Computed once at construction so we never have to guess at call time
    /// whether `boundary` "already includes" the dashes -- it never does;
    /// the leading `--` is a wire-format requirement independent of whatever
    /// characters the boundary token itself happens to contain (WebKit
    /// tokens, for example, embed their own leading dashes as literal token
    /// content).
    boundary_delimiter: Vec<u8>,
    content_length: usize,
}

pub type FieldCallBackResult = Result<Option<Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>>, ()>;

impl<'a> MultipartData<'a> {
    /// For creating new Multipart parser
    pub (crate) fn new(
        stream_holder: MultipartStreamHolder<'a>,
        reading_buffer: &'a mut BodyReadingBuffer,
        boundary: Cow<'a, str>,
        content_length: usize,
    ) -> MultipartData<'a> {
        let boundary_delimiter = Self::compute_delimiter(&boundary);
        MultipartData {
            stream_holder,
            reading_buffer,
            _boundary:boundary
            ,
            boundary_delimiter,
            content_length,
        }
    }

    /// The wire delimiter is always `--` followed by the boundary token,
    /// verbatim. Do NOT special-case tokens that already start with `-`:
    /// those dashes belong to the token itself, not to the delimiter
    /// prefix, and skipping the prepend causes the search pattern to be
    /// shorter than the real delimiter (mismatched length/offset), which
    /// makes `found_boundary_in` fail to locate it at all -- the entire
    /// remainder of the body then gets handed to the current field's
    /// callback as if it were unterminated data.
    #[inline]
    fn compute_delimiter(boundary: &str) -> Vec<u8> {
        let mut v = Vec::with_capacity(boundary.len() + 2);
        v.extend_from_slice(b"--");
        v.extend_from_slice(boundary.as_bytes());
        v
    }

    pub async fn on_field_detected(
        &mut self,
        mut callback: impl FnMut(&MultiPartFormDataField, &[u8]) -> FieldCallBackResult,
    ) -> Result<(), ()> {
        let mut field: Option<MultiPartFormDataField<'_>> = None;
        let boundary: &[u8] = &self.boundary_delimiter;

        match &mut self.stream_holder {
            MultipartStreamHolder::H1(h1) => {
                let left_bytes_len = h1.left_bytes.len();

                // Strip initial boundary on request start if present
                if h1.left_bytes.starts_with(boundary) {
                    h1.left_bytes = &h1.left_bytes[boundary.len()..];
                    if h1.left_bytes.starts_with(b"\r\n") {
                        h1.left_bytes = &h1.left_bytes[2..];
                    }
                }

                loop {
                    // Check for terminal boundary (`--{boundary}--`) or clean end.
                    // Exact equality only -- this must not match merely because
                    // some field's payload happens to *start with* these bytes.
                    if h1.left_bytes == b"--\r\n" || h1.left_bytes == b"--" {
                        return Ok(());
                    }

                    // Buffer exhausted — transition to reading buffer stream
                    if h1.left_bytes.is_empty() {
                        if left_bytes_len + self.reading_buffer.bytes_red_by_buffer >= self.content_length {
                            return Ok(());
                        }
                        return self.read_using_local_buffer(field, callback, left_bytes_len).await;
                    }

                    match &field {
                        None => {
                            // Strip leading \r\n before parsing new headers if present
                            if h1.left_bytes.starts_with(b"\r\n") {
                                h1.left_bytes = &h1.left_bytes[2..];
                            }

                            if let Some(f_field) = MultiPartFormDataField::new(h1.left_bytes) {
                                h1.left_bytes = &h1.left_bytes[f_field.field_header_length..];
                                field = Some(f_field);
                            } else {
                                self.reading_buffer.extend_from_slice(h1.left_bytes);
                                h1.left_bytes = &[];
                                continue;
                            }
                        }
                        Some(f_field) => {
                            match found_boundary_in(h1.left_bytes, boundary) {
                                PatternExistResult::Some(index) => {
                                    let data = &h1.left_bytes[..index];
                                    // Final segment before a confirmed boundary:
                                    // safe to strip the trailing CRLF that
                                    // precedes the delimiter.
                                    if Self::handle_callback(f_field, data, &mut callback, true).await.is_err() {
                                        return Err(());
                                    }

                                    // Consume payload + boundary marker
                                    let mut consumed = index + boundary.len();
                                    if h1.left_bytes[consumed..].starts_with(b"\r\n") {
                                        consumed += 2;
                                    } else if h1.left_bytes[consumed..].starts_with(b"--") {
                                        return Ok(());
                                    }

                                    h1.left_bytes = &h1.left_bytes[consumed..];
                                    field = None; // Reset field state to parse next header
                                    continue;
                                }
                                PatternExistResult::MaybeExistOnLastBytesFromLen(index) => {
                                    // Not a confirmed boundary yet -- just an
                                    // in-flight chunk. Do NOT shave CRLF here.
                                    let to_send = &h1.left_bytes[..index];
                                    if Self::handle_callback(f_field, to_send, &mut callback, false).await.is_err() {
                                        return Err(());
                                    }

                                    self.reading_buffer.clear();
                                    self.reading_buffer.extend_from_slice(&h1.left_bytes[index..]);
                                    h1.left_bytes = &[];

                                    if self.reading_buffer.read_buf(h1.stream).await.is_err() {
                                        return Err(());
                                    }
                                    continue;
                                }
                                PatternExistResult::None => {
                                    // Pure streaming chunk, no boundary in sight.
                                    // Must not be shaved -- it is not adjacent
                                    // to a delimiter.
                                    if Self::handle_callback(f_field, h1.left_bytes, &mut callback, false).await.is_err() {
                                        return Err(());
                                    }
                                    h1.left_bytes = &[];
                                    continue;
                                }
                            }
                        }
                    }
                }
            }
            MultipartStreamHolder::H2(_) => {
                self.read_using_local_buffer_for_h2(field, callback).await
            }
        }
    }

    #[inline(always)]
    fn shave_data(data: &[u8]) -> &[u8] {
        if data.ends_with(b"\r\n") {
            &data[..data.len() - 2]
        } else {
            data
        }
    }

    /// `is_final_segment` must be `true` only when `data` is the last chunk
    /// of a field, immediately followed by a confirmed boundary delimiter.
    /// Any other call site is relaying an in-flight streaming chunk and must
    /// pass `false`, otherwise a chunk that merely happens to end on `\r\n`
    /// (due to network fragmentation) gets 2 real bytes truncated from the
    /// field's actual content.
    #[inline(always)]
    async fn handle_callback(
        field: &'_ MultiPartFormDataField<'_>,
        data: &[u8],
        callback: &mut impl FnMut(&'_ MultiPartFormDataField<'_>, &'_ [u8]) -> FieldCallBackResult,
        is_final_segment: bool,
    ) -> Result<(), ()> {
        let payload = if is_final_segment { Self::shave_data(data) } else { data };
        match callback(field, payload) {
            Ok(Some(future)) => {
                if future.await.is_err() {
                    return Err(());
                }
            }
            Ok(None) => {}
            Err(_) => return Err(()),
        };
        Ok(())
    }

    #[inline]
    async fn read_using_local_buffer(
        &mut self,
        mut field: Option<MultiPartFormDataField<'_>>,
        mut callback: impl FnMut(&'_ MultiPartFormDataField<'_>, &'_ [u8]) -> FieldCallBackResult,
        left_bytes_len: usize,
    ) -> Result<(), ()> {
        let boundary: &[u8] = &self.boundary_delimiter;
        let boundary_length = boundary.len();
        let mut field_bytes = Vec::<u8>::with_capacity(2500);

        let h1 = match &mut self.stream_holder {
            MultipartStreamHolder::H1(h1) => h1,
            MultipartStreamHolder::H2(_) => return Err(()),
        };

        loop {
            let have_enough = self.reading_buffer.bytes_red_by_buffer + left_bytes_len >= self.content_length;

            if !have_enough {
                if self.reading_buffer.read_buf(h1.stream).await.is_err() {
                    return Err(());
                }
            }
            // If we already believe we've read the full content length,
            // don't spin waiting for more bytes that will never arrive --
            // just fall through and let the normal field/boundary matching
            // below consume whatever is already buffered. The terminal
            // boundary is already detected correctly inside the
            // `PatternExistResult::Some` arm (its `starts_with(b"--")`
            // check), so no separate end-of-stream heuristic is needed.

            match &field {
                None => {
                    let mut chunk = self.reading_buffer.chunk();

                    if chunk.is_empty() {
                        // Nothing left to parse and no more bytes coming.
                        return Ok(());
                    }

                    // Consume initial boundary/CRLF if standing on one
                    if chunk.starts_with(boundary) {
                        self.reading_buffer.advance(boundary_length);
                        chunk = self.reading_buffer.chunk();
                    }
                    if chunk.starts_with(b"\r\n") {
                        self.reading_buffer.advance(2);
                        chunk = self.reading_buffer.chunk();
                    }

                    // Terminal boundary (exact match only, see H1 loop note above)
                    if chunk == b"--\r\n" || chunk == b"--" {
                        self.reading_buffer.advance(chunk.len());
                        return Ok(());
                    }

                    if let Some(r_field) = MultiPartFormDataField::new(chunk) {
                        field_bytes.clear();
                        field_bytes.extend_from_slice(&chunk[..r_field.field_header_length]);
                        field = Some(MultiPartFormDataField::new(&field_bytes).unwrap());
                        self.reading_buffer.advance(r_field.field_header_length);
                        continue;
                    } else if have_enough {
                        // No full header available and nothing more will
                        // arrive -- avoid looping forever.
                        return Ok(());
                    }
                }
                Some(r_field) => {
                    let chunk = self.reading_buffer.chunk();

                    match found_boundary_in(chunk, boundary) {
                        PatternExistResult::Some(index) => {
                            let data = &chunk[..index];
                            if Self::handle_callback(r_field, data, &mut callback, true).await.is_err() {
                                return Err(());
                            }

                            let mut consumed = index + boundary_length;
                            if chunk[consumed..].starts_with(b"\r\n") {
                                consumed += 2;
                            } else if chunk[consumed..].starts_with(b"--") {
                                self.reading_buffer.advance(consumed + 2);
                                return Ok(());
                            }

                            self.reading_buffer.advance(consumed);
                            field = None;
                            continue;
                        }
                        PatternExistResult::MaybeExistOnLastBytesFromLen(index) => {
                            let data = &chunk[..index];
                            if Self::handle_callback(r_field, data, &mut callback, false).await.is_err() {
                                return Err(());
                            }
                            let consumed = data.len();
                            self.reading_buffer.advance(consumed);
                            if have_enough {
                                // Nothing more will arrive to resolve the
                                // ambiguous tail -- stop rather than spin.
                                return Ok(());
                            }
                            continue;
                        }
                        PatternExistResult::None => {
                            if chunk.is_empty() {
                                if have_enough {
                                    return Ok(());
                                }
                            } else if Self::handle_callback(r_field, chunk, &mut callback, false).await.is_err() {
                                return Err(());
                            }
                            let consumed = chunk.len();
                            self.reading_buffer.advance(consumed);
                            continue;
                        }
                    }
                }
            }
        }
    }

    #[inline]
    async fn read_using_local_buffer_for_h2(
        &mut self,
        mut field: Option<MultiPartFormDataField<'_>>,
        mut callback: impl FnMut(&'_ MultiPartFormDataField<'_>, &'_ [u8]) -> FieldCallBackResult,
    ) -> Result<(), ()> {
        let boundary: &[u8] = &self.boundary_delimiter;
        let boundary_length = boundary.len();
        let mut field_bytes = Vec::<u8>::with_capacity(2500);

        let h2 = match &mut self.stream_holder {
            MultipartStreamHolder::H1(_) => return Err(()),
            MultipartStreamHolder::H2(h2) => h2,
        };

        let body_mut = h2.batch.body_mut();

        loop {
            let chunk = self.reading_buffer.chunk();

            // Drive the state machine forward
            if !chunk.is_empty() {
                match &field {
                    None => {
                        let mut chunk = chunk;

                        if chunk.starts_with(boundary) {
                            self.reading_buffer.advance(boundary_length);
                            self.reading_buffer.bytes_red_by_buffer += boundary_length;
                            chunk = self.reading_buffer.chunk();
                        }
                        if chunk.starts_with(b"\r\n") {
                            self.reading_buffer.advance(2);
                            self.reading_buffer.bytes_red_by_buffer += 2;
                            chunk = self.reading_buffer.chunk();
                        }

                        // Terminal boundary check -- exact equality only,
                        // and only reachable here (no active field), unlike
                        // the previous `starts_with` check that ran
                        // unconditionally every iteration and could
                        // misfire on binary field payloads that happened
                        // to begin with the same bytes.
                        if chunk == b"--\r\n" || chunk == b"--" {
                            let len = chunk.len();
                            self.reading_buffer.advance(len);
                            self.reading_buffer.bytes_red_by_buffer += len;
                            return Ok(());
                        }

                        if let Some(r_field) = MultiPartFormDataField::new(chunk) {
                            let header_length = r_field.field_header_length;

                            field_bytes.clear();
                            field_bytes.extend_from_slice(&chunk[..header_length]);
                            field = Some(MultiPartFormDataField::new(&field_bytes).unwrap());

                            self.reading_buffer.advance(header_length);
                            self.reading_buffer.bytes_red_by_buffer += header_length;
                            continue;
                        }
                    }
                    Some(r_field) => {
                        match found_boundary_in(chunk, boundary) {
                            PatternExistResult::Some(index) => {
                                let data = &chunk[..index];
                                if Self::handle_callback(r_field, data, &mut callback, true).await.is_err() {
                                    return Err(());
                                }

                                let mut consumed = index + boundary_length;
                                if chunk[consumed..].starts_with(b"\r\n") {
                                    consumed += 2;
                                } else if chunk[consumed..].starts_with(b"--") {
                                    self.reading_buffer.advance(consumed + 2);
                                    return Ok(());
                                }

                                self.reading_buffer.advance(consumed);
                                self.reading_buffer.bytes_red_by_buffer += consumed;
                                field = None;
                                continue;
                            }
                            PatternExistResult::MaybeExistOnLastBytesFromLen(index) => {
                                let data = &chunk[..index];
                                if Self::handle_callback(r_field, data, &mut callback, false).await.is_err() {
                                    return Err(());
                                }
                                let consumed = data.len();
                                self.reading_buffer.advance(consumed);
                                self.reading_buffer.bytes_red_by_buffer += consumed;
                                continue;
                            }
                            PatternExistResult::None => {
                                if Self::handle_callback(r_field, chunk, &mut callback, false).await.is_err() {
                                    return Err(());
                                }
                                let consumed = chunk.len();
                                self.reading_buffer.advance(consumed);
                                self.reading_buffer.bytes_red_by_buffer += consumed;
                                continue;
                            }
                        }
                    }
                }
            }

            // Fetch next frame if buffer exhausted
            if self.reading_buffer.bytes_red_by_buffer < self.content_length {
                if let Some(data_result) = body_mut.data().await {
                    match data_result {
                        Ok(data) => {
                            if data.is_empty() {
                                tokio::task::yield_now().await;
                                continue;
                            }
                            self.reading_buffer.extend_from_slice(data.as_ref());
                        }
                        Err(_) => return Err(()),
                    }
                } else {
                    if field.is_none() {
                        return Ok(());
                    }
                    return Err(());
                }
            } else {
                return Ok(());
            }
        }
    }

    /// Converting buffer bytes into [FormDataAll]
    pub async fn take(mut self) -> Result<FormDataAll, ()> {
        let mut data = FormDataAll::new();
        if (&mut self)
            .on_field_detected(|field, parsed_data| {
                data.push(field, parsed_data);
                Ok(None)
            })
            .await
            .is_err()
        {
            return Err(());
        }
        Ok(data)
    }

    /// Shortcut for [self.take]
    pub async fn to_form_data_all(self) -> Result<FormDataAll, ()> {
        self.take().await
    }
}