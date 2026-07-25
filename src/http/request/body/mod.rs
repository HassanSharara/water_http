mod bytes_puller;
mod chunked;
mod multipartformdata;
mod multer;
mod stream_holders;
mod xwwwformurlencoded;

use std::borrow::Cow;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use bytes::{Bytes, BytesMut};
pub use bytes_puller::*;
pub use chunked::*;
pub use multipartformdata::MultiPartFormDataField;
pub use multer::*;
pub (crate) use stream_holders::*;

pub use xwwwformurlencoded::*;
use crate::http::request::header::KeyValueMap;
use crate::server::errors::WaterErrors;

/// Indicates the incoming body state for a request.
#[derive(Debug)]
pub enum IBody<'a> {
    /// When body bytes are handled as a continuous zero-copy slice or processed manually.
    Bytes(&'a [u8]),
    /// Handling parsed multipart/form-data.
    MultiPartFormData(FormDataAll),
    /// Handling parsed x-www-form-urlencoded data.
    XWWWFormUrlEncoded(XWWWFormUrlEncoded<'a>),
}

/// Incoming body stream when handled as chunks.
pub enum IBodyChunks<'a> {
    /// Handling incoming body bytes via puller.
    Bytes(BytesPuller<'a>),
    /// Parsing incoming bytes into [MultiPartFormDataField] stream handlers.
    FormData(MultipartData<'a>),
    #[cfg(feature = "accept_transfer_chunked")]
    /// Reading incoming body with chunked transfer-encoding.
    Chunked(BodyChunkedReader<'a>),
}

/// Informs the context how incoming request body bytes should be parsed.
pub enum ParsingBodyMechanism {
    /// Let the context determine the parsing strategy based on Content-Type header.
    Default,
    /// Keep incoming bytes raw without automatic parsing.
    JustBytes,
    /// Parse incoming bytes into multipart form fields.
    FormData,
    /// Parse incoming bytes into x-www-form-urlencoded structure.
    XWWWFormData,
    #[cfg(feature = "accept_transfer_chunked")]
    /// Parse transfer-encoding: chunked body.
    ChunkedTransferEncoding,
}

/// Parsing body mechanism execution results.
pub enum ParsingBodyResults<'a> {
    /// Incoming body is processed as chunks to protect RAM from oversized payloads.
    Chunked(IBodyChunks<'a>),
    /// Full body loaded into memory (common case for smaller payloads).
    FullBody(IBody<'a>),
    /// Request has no body (e.g., standard GET request).
    None,
    /// Parsing error encountered.
    Err(WaterErrors<'a>),
}

impl<'a> ParsingBodyResults<'a> {
    pub async fn on_multipart_form_data_detect(
        payload: &'a [u8],
        mut on_detect: impl FnMut(
            Result<MultiPartFormDataField<'a>, &str>,
        ) -> Pin<Box<dyn Future<Output = HandlingFormDataResult> + Send>>,
    ) -> Result<(), &'a str> {
        let mut index: usize = 0;
        while index < payload.len() {
            match MultiPartFormDataField::new(&payload[index..]) {
                None => break,
                Some(data) => {
                    index += data.field_header_length;
                    match on_detect(Ok(data)).await {
                        HandlingFormDataResult::Pass => continue,
                        HandlingFormDataResult::Stop => break,
                        HandlingFormDataResult::Shutdown => return Err("shutdown connection"),
                    }
                }
            }
        }
        Ok(())
    }

    /// Returns `true` if body parsing resulted in an error.
    #[inline]
    pub fn is_err(&self) -> bool {
        matches!(self, ParsingBodyResults::Err(_))
    }

    /// Returns `true` if no body payload was present.
    #[inline]
    pub fn is_none(&self) -> bool {
        matches!(self, ParsingBodyResults::None)
    }
}

/// Controls stream execution during form-data parsing.
pub enum HandlingFormDataResult {
    /// Continue reading and parsing next field.
    Pass,
    /// Stop parsing form-data fields but keep connection open.
    Stop,
    /// Terminate reading and close the connection immediately.
    Shutdown,
}

/// Represents a single parsed multipart form field using zero-copy byte buffers.
#[derive(Debug, Clone)]
pub struct HeapFormField {
    pub multipart: KeyValueMap,
    pub data: Bytes,
}

impl HeapFormField {
    fn from(value: &MultiPartFormDataField, data: &[u8]) -> Self {
        let map = KeyValueMap::from(&value.headers);
        HeapFormField {
            multipart: map,
            data: Bytes::copy_from_slice(data),
        }
    }

    /// Returns the body payload bytes of this field.
    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Returns the zero-copy `Bytes` handle.
    #[inline]
    pub fn bytes(&self) -> Bytes {
        self.data.clone()
    }

    /// Checks if this field is an uploaded file.
    #[inline]
    pub fn is_file(&self) -> bool {
        self.multipart.get_as_bytes("filename").is_some()
    }

    /// Returns the content-type of this field, if specified.
    #[inline]
    pub fn content_type(&self) -> Option<&[u8]> {
        self.multipart.get_as_bytes("Content-Type").map(|v| v.as_ref())
    }
}

/// Aggregated container for parsed multipart form data fields.
#[derive(Debug, Clone, Default)]
pub struct FormDataAll {
    pub fields: Vec<HeapFormField>,
}

impl DynamicBodyMapTrait for FormDataAll {
    fn get_as_bytes(&self, key: &str) -> Option<&[u8]> {
        self.get_field(key).map(|field| field.data())
    }

    fn get(&self, key: &str) -> Option<Cow<'_, str>> {
        self.get_as_bytes(key).map(String::from_utf8_lossy)
    }

    fn all(&self) -> HashMap<String, Bytes> {
        let mut map = HashMap::with_capacity(self.fields.len());
        for field in &self.fields {
            if let Some(name) = field.multipart.get_as_str("name") {
                let clean_name = name.trim_matches('"');
                map.insert(clean_name.to_string(), field.data.clone());
            }
        }
        map
    }
}

impl FormDataAll {
    pub fn new() -> Self {
        Self { fields: Vec::new() }
    }

    /// Retrieves a field reference by key without heap allocations.
    pub fn get_field(&self, key: &str) -> Option<&HeapFormField> {
        let target_key = key.trim_matches('"');
        self.fields.iter().find(|field| {
            field
                .multipart
                .get_as_str("name")
                .map_or(false, |name| name.trim_matches('"') == target_key)
        })
    }

    /// Retrieves a mutable field reference by key without heap allocations.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut HeapFormField> {
        let target_key = key.trim_matches('"');
        self.fields.iter_mut().find(|field| {
            field
                .multipart
                .get_as_str("name")
                .map_or(false, |name| name.trim_matches('"') == target_key)
        })
    }

    /// Appends incoming field data chunks efficiently using zero-copy byte buffering.
    pub(crate) fn push(&mut self, field: &MultiPartFormDataField, data: &[u8]) {
        if let Some(name) = field.content_disposition_name() {
            if let Some(target) = self.get_mut(name.as_ref()) {
                // Efficiently extend using BytesMut to avoid redundant Vec reallocation
                let mut buffer = BytesMut::with_capacity(target.data.len() + data.len());
                buffer.extend_from_slice(&target.data);
                buffer.extend_from_slice(data);
                target.data = buffer.freeze();
                return;
            }
        }
        self.fields.push(HeapFormField::from(field, data));
    }
}

/// Chunk processing state indicator.
pub enum HandlingChunkResult<'a> {
    Consumed,
    Err(&'a str),
}

/// Unified abstraction wrapper for dynamic body types.
#[derive(Debug, Clone)]
pub enum DynamicBodyMap {
    FormField(FormDataAll),
    Xww(HeapXWWWFormUrlEncoded),
}

impl DynamicBodyMapTrait for DynamicBodyMap {
    fn get_as_bytes(&self, key: &str) -> Option<&[u8]> {
        match self {
            DynamicBodyMap::FormField(data) => data.get_as_bytes(key),
            DynamicBodyMap::Xww(data) => data.get_as_bytes(key),
        }
    }

    fn get(&self, key: &str) -> Option<Cow<'_, str>> {
        match self {
            DynamicBodyMap::FormField(data) => data.get(key),
            DynamicBodyMap::Xww(data) => data.get(key),
        }
    }

    fn all(&self) -> HashMap<String, Bytes> {
        match self {
            DynamicBodyMap::FormField(data) => data.all(),
            DynamicBodyMap::Xww(data) => data.all(),
        }
    }
}

/// Trait providing unified querying functionality across dynamic body abstractions.
pub trait DynamicBodyMapTrait {
    fn get_as_bytes(&self, key: &str) -> Option<&[u8]>;

    fn get(&self, key: &str) -> Option<Cow<'_, str>>;

    fn all(&self) -> HashMap<String, Bytes>;

    fn get_as_encoded_string(&self, key: &str) -> Option<String> {
        self.get(key).map(|s| {
            urlencoding::decode(s.as_ref())
                .map(|c| c.into_owned())
                .unwrap_or_else(|_| s.into_owned())
        })
    }
}