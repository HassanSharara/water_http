use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use crate::inc_start_pointer;


/// organizing keys and values with simple and fast structure
#[derive(Debug)]
pub  struct KeyValueList<'a,const LENGTH:usize> {
    data:[KeyValuePair<'a>;LENGTH],
    cursor:usize
}

impl <'a,const DATA_LENGTH:usize> KeyValueList<'a,DATA_LENGTH> {
    pub (crate) fn empty()->KeyValueList<'a,DATA_LENGTH> {
        KeyValueList {
            data:[KeyValuePair::empty();DATA_LENGTH],
            cursor:0
        }
    }

    pub (crate) fn is_fully_filled(&self)->bool{
        self.cursor >= self.data.len()
    }
    pub (crate) fn push(&mut self,key:&'a[u8],value:&'a [u8])->Result<(),&str>{
        if self.is_fully_filled() { return Err("the stack of key value pair is fully filled")}
        let child = self.data.get_mut(self.cursor);
        if let Some(data) = child {
            data.change_to(key,value);
            self.cursor+=1;
        }
        Ok(())
    }

    /// returning all data that hold valid key-value pair
    pub fn all_pairs(&self)->&'a [KeyValuePair]{
        &self.data[..self.cursor]
    }


    /// getting value as bytes from pair using key
    pub fn get_as_bytes(&self,key:&str)->Option<&'a [u8]>{
        for ref i in self.data {
            if i.key == key.as_bytes() {
                return Some(i.value);
            }
        }
        None
    }


    /// getting value as HeaderValue from pair using key
    pub fn get_as_header_value(&self,key:&str)->Option<HeaderValue<'a>>{
        for ref i in self.data {
            if i.key == key.as_bytes() {
                return Some(HeaderValue::new(i.value));
            }
        }
        None
    }

    /// getting value as [`Cow<str>`] from pair using key
    pub fn get_as_str(&self,key:&str)->Option<Cow<str>>{
        if let Some(value) = self.get_as_bytes(key){
            return Some(String::from_utf8_lossy(value));
        }
        None
    }

    /// for try parsing bytes into headers key and value
    pub  fn try_parse<const L: usize>(bytes: &[u8]) ->Option<KeyValueList<L>> {

        let mut key_list = KeyValueList::empty();
        let mut end_indicators = 0_u16;
        let mut last_index_used = 0_usize;
        let mut key =None;
        for (index,byte) in bytes.iter().enumerate() {
            match byte {
                &b':'=>{
                    key = Some(&bytes[last_index_used..index]);
                    last_index_used = index ;
                }

                &b' '=>{
                    if last_index_used+1 == index && key.is_some()
                    && index +1 < bytes.len(){
                        last_index_used=index+1;
                    }
                }

                &b'\r'=>{
                    end_indicators+=1;
                    if let Some(key) = key {
                        _=key_list.push(key,&bytes[last_index_used..index]);
                    }
                }
                &b'\n'=> {
                    end_indicators+=1;
                    if end_indicators >= 4 {
                        key_list.cursor = end_indicators as usize;
                        return Some(key_list);}
                }
                _=>{
                    end_indicators = 0;
                }
            }
        }
        None
    }
}



/// forming header value
#[derive(Debug)]
pub struct HeaderValue <'a>{
    bytes:&'a [u8],
    /// all header value map
    pub map:Option<HashMap<&'a [u8],&'a[u8]>>
}

impl<'a> HeaderValue<'a> {

    #[allow(unused_assignments)]
    pub (crate) fn new(bytes:&'a [u8])->HeaderValue<'a>{
        let mut map = HashMap::new();
        let mut last_index = 0_usize;
        let mut key = None;

        let payload_length = bytes.len();
        for (index,byte) in bytes.iter().enumerate() {

            match byte {
                &b'='=>{
                    key = Some(&bytes[last_index..index]);
                    inc_start_pointer!(last_index,index,payload_length);
                }
                &b' '=>{
                    if index == 1 {
                        last_index = index;
                        continue;
                    }
                    else if let Some(k) = key {
                        map.insert(k,&bytes[last_index..index]);
                        inc_start_pointer!(last_index,index,payload_length);
                        key = None;
                    } else {
                        if &bytes[last_index] == &b';' {
                            last_index = index;
                            inc_start_pointer!(last_index,index,payload_length);
                        }
                    }
                }
                &b';' | &b'\r' => {
                    if let Some(k) = key {
                        map.insert(k,&bytes[last_index..index]);
                        inc_start_pointer!(last_index,index,payload_length);
                        key = None;
                    } else {
                        last_index = index;
                    }
                }
                _ => {}
            }
        }
        if let Some(key) = key {
            map.insert(key,&bytes[last_index..]);
        }

        HeaderValue{
            bytes,
            map:map.into()
        }

    }

    /// get as string
    pub fn get_from_values_as_string(&self,key:&'static str)->Option<String>{
        if let Some(v) = self.get_from_values_as_str(key){
            return Some(v.replace("\"",""));
        }
        None
    }

    /// get value as [`Cow<str>`]
    pub fn get_from_values_as_str(&self,key:&'static str)->Option<Cow<str>>{
        if let Some(v) = self.get_from_values_as_bytes(key){
            return Some(String::from_utf8_lossy(v));
        }
        None
    }

    /// get value as [`&'a [u8]`]
    pub fn get_from_values_as_bytes(&self,key:&'static str)->Option<&'a[u8]>{
        if let Some(v) = self.map.as_ref() {
            if let Some(value) = v.get(key.as_bytes()) {
                return Some(*value);
            }
        }
        None
    }

    /// converting the total value to str
    pub fn to_str(&self)->Cow<str>{
        String::from_utf8_lossy(self.bytes)
    }


    /// to bytes
    pub fn to_bytes(&self)->&'a [u8]{
        self.bytes
    }
}

/// wrap struct for incoming header key => value pair
#[derive(Debug)]
pub struct KeyValuePair<'a> {
     pub key:&'a [u8],
     pub value:&'a [u8]
 }
impl<'a> Clone for KeyValuePair<'a> {
    fn clone(&self) -> Self {
        Self {
            key:self.key,
            value:self.value,
        }
    }
}
impl <'a> Copy for  KeyValuePair<'a> {}
impl <'a> KeyValuePair <'a> {

    pub fn change_to(&mut self,key:&'a [u8],value:&'a [u8]){
        self.key = key;
        self.value = value;
    }
    pub fn empty()->Self {
        KeyValuePair {
            key:b"",
            value:b""
        }
    }
}
impl <'a> Display for  KeyValuePair<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut to_show = String::new();
        to_show.push_str(String::from_utf8_lossy(self.key).to_string().as_str());
        to_show.push_str(" : ");
        let value = String::from_utf8_lossy(self.value);
        to_show.push_str(&value);
        std::fmt::Display::fmt(
            &to_show,
            f
        )
    }
}




#[cfg(test)]
mod test_key_list {
    use crate::http::request::KeyValueList;

    #[test]
    fn check_header_parsing(){
        let bytes = b"Content-Disposition: attachment; filename=\"example.txt\" name=\"file1\"\r\nContent-Type: Image/png\r\n\r\n";

        let v = KeyValueList::<12>::try_parse::<12>(bytes);
        if let Some(ref d) = v {
            let content_disposition = d.get_as_header_value("Content-Disposition");
            if let Some(content_disposition) = content_disposition {
                println!("{:?}",content_disposition.get_from_values_as_str("name"));
            }
        }
        assert!(v.is_some());
    }
}


