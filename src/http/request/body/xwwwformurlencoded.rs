use std::borrow::Cow;
use std::collections::HashMap;
use bytes::Bytes;
use crate::http::request::DynamicBodyMapTrait;


/// when ever your incoming request have x-www-form-urlencoded data body type
/// it would be serialized to [XWWWFormUrlEncoded]
#[derive(Debug)]
pub struct XWWWFormUrlEncoded<'a> {

    /// this form of data just holding references to memory addresses where
    /// incoming data is
    /// and that`s a better approach than allocating new memory address
    /// to structr new data form
    data: Vec<(&'a [u8], &'a [u8])>,
}

/// heap implementation data for x-www form url encoded data
#[derive(Debug, Clone)]
pub struct HeapXWWWFormUrlEncoded {
    data: HashMap<String, Bytes>,
}

impl HeapXWWWFormUrlEncoded {
    /// creating new heap x-www-form-urlencoded data
    pub fn new(data: &XWWWFormUrlEncoded) -> Self {
        let all = data.all();
        Self {
            data: all,
        }
    }

    pub fn all_ref(&self) -> &HashMap<String, Bytes> {
        &self.data
    }
}

impl DynamicBodyMapTrait for HeapXWWWFormUrlEncoded {
    fn get_as_bytes(&self, key: &str) -> Option<&[u8]> {
        if let Some(data) = self.data.get(key) {
            return Some(data.as_ref());
        }
        None
    }

    fn get(&self, key: &str) -> Option<Cow<'_, str>> {
        if let Some(data) = self.data.get(key) {
            return Some(String::from_utf8_lossy(data.as_ref()));
        }
        None
    }

    fn all(&self) -> HashMap<String, Bytes> {
        self.data.clone()
    }
}

impl<'a> DynamicBodyMapTrait for XWWWFormUrlEncoded<'a> {
    fn get_as_bytes(&self, key: &str) -> Option<&[u8]> {
        self.get_as_bytes_ref(key)
    }

    fn get(&self, key: &str) -> Option<Cow<'_, str>> {
        self.get_as_str(key)
    }

    fn all(&self) -> HashMap<String, Bytes> {
        let mut map = HashMap::new();
        for &(key, value) in &self.data {
            map.insert(String::from_utf8_lossy(key).to_string(), Bytes::copy_from_slice(value));
        }
        map
    }
}

/// crate self using implementations for framework
impl<'a> XWWWFormUrlEncoded<'a> {

    /// for getting value based on its given key
    pub fn get_as_bytes_ref(&self, key: &str) -> Option<&'a [u8]> {
        let key_bytes = key.as_bytes();
        for &(k, v) in &self.data {
            if k == key_bytes {
                return Some(v);
            }
        }
        None
    }

    /// for getting value based on its given key as [&str]
    pub fn get_as_str(&self, key: &str) -> Option<Cow<'_, str>> {
        if let Some(data) = self.get_as_bytes_ref(key) {
            return Some(String::from_utf8_lossy(data));
        }
        None
    }

    /// for getting all incoming data as slice of byte pairs
    pub fn all_ref(&self) -> &[(&'a [u8], &'a [u8])] {
        &self.data
    }

    pub (crate) fn new(payload: &'a [u8]) -> XWWWFormUrlEncoded<'a> {
        let mut data = Vec::with_capacity(8);
        let mut key: Option<&'a [u8]> = None;
        let mut cursor = 0_usize;

        for (index, &byte) in payload.iter().enumerate() {
            match key {
                None => {
                    if byte == b'=' {
                        key = Some(&payload[cursor..index]);
                        cursor = index + 1;
                    }
                }
                Some(k) => {
                    if byte == b'&' || byte == b'\r' {
                        data.push((k, &payload[cursor..index]));
                        cursor = index + 1;
                        key = None;
                    }
                }
            }
        }
        if let Some(k) = key {
            data.push((k, &payload[cursor..]));
        }
        XWWWFormUrlEncoded { data }
    }
}

#[cfg(test)]
mod test {
    use super::XWWWFormUrlEncoded;

    #[test]
    fn test_xxx_form_url_encoded() {
        let p1 = b"username=john&age=25&note=ok";
        let parsed = XWWWFormUrlEncoded::new(p1);
        assert_eq!(parsed.get_as_bytes_ref("username"), Some(b"john".as_ref()));
        assert_eq!(parsed.get_as_bytes_ref("age"), Some(b"25".as_ref()));
        assert_eq!(parsed.get_as_bytes_ref("note"), Some(b"ok".as_ref()));
    }
}
