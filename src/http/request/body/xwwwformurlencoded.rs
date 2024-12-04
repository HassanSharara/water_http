use std::borrow::Cow;
use std::collections::HashMap;


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



/// crate self using implementations for framework
impl <'a> XWWWFormUrlEncoded<'a> {

    /// for getting value based on its given key
    pub fn get_as_bytes (&self,key:&str)->Option<&'a[u8]>{
        if let Some(data) = self.data.get(key.as_bytes()) {
            return  Some(*data)
        }
        None
    }

    /// for getting value based on its given key as [&str]
    pub fn get(&self,key:&str)->Option<Cow<'a,str>>{
        if let Some(data) = self.get_as_bytes(key) {
            return String::from_utf8_lossy(data).into();
        }
        None
    }
    /// for getting all incoming data as HashMap of bytes
    pub fn all(&self)->&HashMap<&'a[u8],&'a[u8]>{
        &self.data
    }
    pub (crate) fn new(payload:&'a[u8])->XWWWFormUrlEncoded<'a>{
        let mut map = HashMap::new();
        let mut key : Option<&'a [u8]> = None;
        let mut cursor = 0_usize;
        loop {
            match key {
                None =>{
                    if let Some(index) = twoway::find_bytes(payload,b"=") {
                        cursor = index;
                        key = Some(&payload[..index]);
                    }else { break; }
                }
                Some(k)=>{
                    if let Some(index) = twoway::find_bytes(payload,b",") {
                        let n_index = cursor+1;
                        if n_index > index {
                            break;
                        }
                        map.insert(k,&payload[n_index..index]);
                        key = None;
                        continue;
                    }else {
                        let n_index = cursor+1;
                        if n_index > payload.len() {
                            break;
                        }
                        map.insert(k,&payload[n_index..]);
                        break;
                    }
                }
            };

        }
        return XWWWFormUrlEncoded{ data:map}
    }


    pub (crate) fn from_multiple_payloads(payloads:(&'a[u8],&'a[u8]))->XWWWFormUrlEncoded<'a>{
        let mut map = HashMap::new();
        let mut key : Option<&'a [u8]> = None;
        let mut cursor = 0_usize;
        let mut payload = payloads.0;
        let  mut is_first_end = false;
        loop {
            match key {
                None =>{
                    if let Some(index) = twoway::find_bytes(payload,b"=") {
                        cursor = index;
                        key = Some(&payload[..index]);
                    }else {
                        if !is_first_end {
                            is_first_end = true;
                            cursor = 0;
                            payload = payloads.1;
                            continue;
                        }
                        break; }
                }
                Some(k)=>{
                    if let Some(index) = twoway::find_bytes(payload,b",") {
                        let n_index = cursor+1;
                        if n_index > index {
                            break;
                        }
                        map.insert(k,&payload[n_index..index]);
                        key = None;
                        continue;
                    }else {
                        let n_index = cursor+1;
                        if n_index > payload.len() {
                            if !is_first_end {
                                is_first_end = true;
                                cursor = 0;
                                payload = payloads.1;
                                continue;
                            }
                            break;
                        }
                        map.insert(k,&payload[n_index..]);
                        if !is_first_end {
                            is_first_end = true;
                            cursor = 0;
                            payload = payloads.1;
                            continue;
                        }
                        break;
                    }
                }
            };

        }
        return XWWWFormUrlEncoded{ data:map}
    }

}