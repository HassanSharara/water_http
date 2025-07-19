use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use bytes::Bytes;
use zstd::zstd_safe::WriteBuf;
use crate::inc_start_pointer;


/// organizing keys and values with simple and fast structure
#[derive(Debug)]
pub  struct KeyValueList<'a,const LENGTH:usize> {
    pub(crate) data:[KeyValuePair<'a>;LENGTH],
    pub(crate) cursor:usize
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
        for ref i in self.data {
            if String::from_utf8_lossy(i.key).to_lowercase() == key.to_lowercase() {
               return Some(i.value);
            }
        }
        None
    }


    /// getting value as HeaderValue from pair using key
    pub fn get_as_header_value(&self,key:&str)->Option<HeaderValue<'a>>{
        for ref i in self.data {
            if i.key == key.as_bytes() {
                return Some(HeaderValue::new(i.value,Some(i.key)));
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

    /// getting value as [`&str`] from pair using key
    pub fn get_as_str_ref(&self,key:&str)->Option<&'a str>{
        if let Some(value) = self.get_as_bytes(key){
            return Some(unsafe{std::str::from_utf8_unchecked(value)});
        }
        None
    }

    /// for try parsing bytes into headers key and value
    pub  fn try_parse<const L: usize>(bytes: &[u8]) ->Option<(KeyValueList<L>,usize)> {

        let mut key_list = KeyValueList::empty();
        let mut end_indicators = 0_u16;
        let mut last_index_used = 0_usize;
        let mut key =None;
        for (index,byte) in bytes.iter().enumerate() {
            match byte {
                b':'=>{
                    if (&bytes[last_index_used] == &b'\n' ||  &bytes[last_index_used] == &b' ' )&&
                        last_index_used+1 < index {
                        last_index_used+=1;
                    }
                    key = Some(&bytes[last_index_used..index]);
                    last_index_used = index ;
                }

                b' '=>{
                    if last_index_used+1 == index && key.is_some()
                    && index +1 < bytes.len(){
                        last_index_used=index+1;
                    }
                }

                b'\r'=>{

                    end_indicators+=1;
                    if let Some(key) = key {
                        _=key_list.push(key,&bytes[last_index_used..index]);
                    }
                    last_index_used = index;
                }
                b'\n'=> {
                    end_indicators+=1;
                    if end_indicators >= 4 {
                        return Some((key_list,
                                     index+ 1 .min(bytes.len() -1 )
                        ));}
                    last_index_used = index;
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
    pub (crate) fn new(bytes:&'a [u8],mut key:Option<&'a [u8]>)->HeaderValue<'a>{
        let mut map = HashMap::new();
        let mut last_index = 0_usize;
        let payload_length = bytes.len();
        for (index,byte) in bytes.iter().enumerate() {

            match byte {
                &b'='=>{
                    if &bytes[last_index] == &b' ' && last_index+1 <index {
                        last_index+=1;
                    }
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
        if let Some(v) = self.get_from_values_as_bytes(key){
            return Some(String::from_utf8_lossy(v).replace("\"",""));
        }
        None
    }


    /// get as string
    pub fn get_from_values_as_cow(&self,key:&'static str)->Option<Cow<'a,str>>{
        if let Some(mut v) = self.get_from_values_as_bytes(key){
            if v.starts_with(b"\""){
                v = &v[1..];
            }
            if v.ends_with(b"\""){
                v=&v[..v.len()-1]
            }
            return Some(String::from_utf8_lossy(v));
        }
        None
    }
    /// get value as [`&str`]
    ///
    /// notice: that this method is not safe because its converting unknown bytes from headers to std::str
    /// which is not safe so if you want to convert these bytes manually you could use either
    /// [self.get_from_values_as_string] or [self.get_from_values_as_bytes] and then convert these bytes
     pub  fn get_from_values_as_str(&self,key:&'static str)->Option<&'a str>{
        if let Some(v) = self.get_from_values_as_bytes(key){
            let mut v = unsafe {std::str::from_utf8_unchecked(v)};
            if v.starts_with("\"") {
                v =&v[1..];
            }
            if v.ends_with("\"") {
                v = &v[..v.len() - 1 ];
            }
            return Some(v);
        }
        None
    }

    /// get value as [`&'a [u8]`]
    pub fn get_from_values_as_bytes(&self,key:&'static str)->Option<&'a[u8]>{
        if let Some(v) = self.map.as_ref() {
            if let Some(value) = v.get(key.as_bytes()) {
                return Some(*value);
            }
            for (k,v) in v{
                let k = String::from_utf8_lossy(*k).to_lowercase() ;
                if k == key.to_lowercase() {
                    return Some(*v)
                }
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

    /// changing key and value pair
    pub fn change_to(&mut self,key:&'a [u8],value:&'a [u8]){
        self.key = key;
        self.value = value;
    }

    /// converting value pair to header value
    pub fn to_header_value(&self)->HeaderValue{
        HeaderValue::new(self.value,Some(self.key))
    }
    /// get empty key value pair
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



/// Key value  map
#[derive(Debug,Clone)]
pub struct KeyValueMap {
    map:HashMap<String,Bytes>,
}

impl KeyValueMap {
    /// returning all data that hold valid key-value pair
    pub fn all_pairs(&self)->&HashMap<String,Bytes>{
        &self.map
    }


    /// getting value as bytes from pair using key
    pub fn get_as_bytes(&self,key:&str)->Option<&Bytes>{
        self.map.get(key)
    }


    /// getting value as HeaderValue from pair using key
    pub fn get_as_header_value(&self,key:&str)->Option<HeaderValue>{
        if let Some(bytes) = self.get_as_bytes(key){
            HeaderValue::new(bytes.as_slice(),Some(key.as_bytes()));
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

}


impl<'a,const T:usize> From<&'_ KeyValueList<'a,T>> for  KeyValueMap {
    fn from(value: &KeyValueList<'a, T>) -> Self {
        let mut map = HashMap::new();
        for ref value in value.data {
            if value.key.is_empty() {continue;}
            let vv = value.to_header_value();
            if let Some(m) = vv.map.as_ref() {
                for (key,value) in m {
                    map.insert(String::from_utf8_lossy(*key).replace("\"",""),Bytes::copy_from_slice(*value));
                }
            }else {
                map.insert(String::from_utf8_lossy(value.key).to_string().replace("\"",""),Bytes::copy_from_slice(value.value));
            }
        }
        KeyValueMap {
            map
        }
    }
}

#[cfg(test)]
mod test_key_list {
    use crate::http::request::KeyValueList;

    #[test]
    fn check_header_parsing(){
        let bytes = b"Content-Disposition: attachment; filename=\"example.txt\" name=\"file1\"\r\nContent-Type: Image/png\r\n\r\n";

        let v= KeyValueList::<12>::try_parse::<12>(bytes);
        if let Some( (d,_kl)) = &v {
            let content_disposition = d.get_as_header_value("Content-Disposition");
            if let Some(content_disposition) = content_disposition {
                let cd = content_disposition.get_from_values_as_str("Content-Disposition").unwrap();
                let name = content_disposition.get_from_values_as_str("name").unwrap();
                let filename = content_disposition.get_from_values_as_str("filename").unwrap();
                println!("content-disposition = {}",cd);
                println!("content-disposition:name = {}",name);
                println!("content-disposition:filename = {}",filename);

                assert_eq!(cd,"attachment");
                assert_eq!(name,"\"file1\"");
                assert_eq!(filename,"\"example.txt\"");
            }
        }
        assert!(v.is_some());
    }
}


