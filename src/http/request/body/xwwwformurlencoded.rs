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
    data:HashMap<&'a[u8],&'a[u8]>
}

/// heap implementation data for x-www form url encoded data
#[derive(Debug)]
pub struct HeapXWWWFormUrlEncoded{
    data:HashMap<String,Bytes>
}

impl HeapXWWWFormUrlEncoded {
    /// creating new heap x-www-form-urlencoded data
    pub fn new(data:&XWWWFormUrlEncoded)->Self{
        let all = data.all();
        Self {
            data:all
        }
    }

    pub fn all_ref(&self)->&HashMap<String,Bytes>{
        &self.data
    }
}
impl  DynamicBodyMapTrait for HeapXWWWFormUrlEncoded {
    fn get_as_bytes(&self, key: &str) -> Option<&[u8]> {
        if let Some(data) = self.data.get(key){
            return Some(data.as_ref())
        }
        None
    }

    fn get(&self, key: &str) -> Option<Cow<'_,str>> {
        if let Some(data) = self.data.get(key){
            return Some(String::from_utf8_lossy(data.as_ref()))
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

    fn get(&self, key: &str) -> Option<Cow<'_,str>> {
        self.get_as_str(key)
    }

    fn all(&self) -> HashMap<String, Bytes> {
        let mut map = HashMap::new();
        for (key,value) in &self.data {
            map.insert(String::from_utf8_lossy(key).to_string(),Bytes::copy_from_slice(value));
        }
        map
    }


}
/// crate self using implementations for framework
impl <'a> XWWWFormUrlEncoded<'a> {

    /// for getting value based on its given key
     fn get_as_bytes_ref(&self,key:&str)->Option<&'a[u8]>{
        if let Some(data) = self.data.get(key.as_bytes()) {
            return  Some(*data)
        }
        None
    }

    /// for getting value based on its given key as [&str]
     fn get_as_str(&self,key:&str)->Option<Cow<'_,str>>{
        if let Some(data) = self.get_as_bytes(key) {
            return String::from_utf8_lossy(data).into();
        }
        None
    }
    /// for getting all incoming data as HashMap of bytes
    pub fn all_ref(&self)->&HashMap<&'a[u8],&'a[u8]>{
        &self.data
    }
    pub (crate) fn new(payload:&'a[u8])->XWWWFormUrlEncoded<'a>{
        let mut map = HashMap::new();
        let mut key : Option<&'a [u8]> = None;
        let mut cursor = 0_usize;

        for (index,byte) in payload.iter().enumerate() {
            match key {
                None => {

                    match byte {
                        b'=' => {
                            key = Some(&payload[cursor..index]);
                            cursor = index +1;
                        }
                        _=>{}
                    }
                }
                Some(k) => {

                    match byte {
                        b'&' | b'\r' => {
                            map.insert(k,&payload[cursor..index]);
                            cursor=index+1;
                            key = None;
                        }
                        _=>{}
                    }
                }
            }
        }
        if let Some(k) = key {
            map.insert(k,&payload[cursor..]);
        }
        return XWWWFormUrlEncoded{ data:map}
    }




    /*

    pub fn from_multiple_payloads(payloads: (&'a [u8], &'a [u8])) -> XWWWFormUrlEncoded<'a> {
        let mut map = HashMap::new();
        let mut key: Option<&'a [u8]> = None;
        let mut part = 0;
        let mut payload = payloads.0;
        use twoway::find_bytes;
        loop {
            if payload.is_empty() {
                // move to next payload if available
                if part == 0 {
                    part = 1;
                    payload = payloads.1;
                    continue;
                } else {
                    break;
                }
            }

            if let Some(k) = key {
                // find '&' for value end
                if let Some(idx) = find_bytes(payload, b"&") {
                    let value = &payload[..idx];
                    map.insert(k, value);
                    payload = &payload[idx + 1..];
                    key = None;
                    continue;
                } else {
                    // last pair in this payload
                    map.insert(k, payload);
                    if part == 0 {
                        part = 1;
                        payload = payloads.1;
                        continue;
                    } else {
                        break;
                    }
                }
            } else {
                // find '=' for key
                if let Some(idx) = find_bytes(payload, b"=") {
                    key = Some(&payload[..idx]);
                    payload = &payload[idx + 1..];
                } else {
                    if part == 0 {
                        part = 1;
                        payload = payloads.1;
                        continue;
                    } else {
                        break;
                    }
                }
            }
        }

        XWWWFormUrlEncoded { data: map }
    }
    */
}



#[cfg(test)]
mod test {
    use super::XWWWFormUrlEncoded; // import XWWWFormUrlEncoded into this module




    #[test]
    fn test_xxx_form_url_encoded() {
        let p1 = b"username=john&age=25&note=ok";
        let parsed = XWWWFormUrlEncoded::new(p1);
        assert_eq!(parsed.data.get(b"username".as_ref()), Some(&b"john".as_ref()));
        assert_eq!(parsed.data.get(b"age".as_ref()), Some(&b"25".as_ref()));
        assert_eq!(parsed.data.get(b"note".as_ref()), Some(&b"ok".as_ref()));
    }

}