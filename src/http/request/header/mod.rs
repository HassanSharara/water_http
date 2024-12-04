use std::borrow::Cow;
use std::fmt::{Display, Formatter};


pub (crate) struct KeyValueList<'a,const length:usize> {
    data:[KeyValuePair<'a>;length],
    cursor:usize
}

impl <'a,const data_length:usize> KeyValueList<'a,data_length> {
    pub (crate) fn empty()->KeyValueList<'a,data_length> {
        KeyValueList {
            data:[KeyValuePair::empty();data_length],
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
    pub fn get_as_bytes(&self,key:&[u8])->Option<&'a [u8]>{
        for ref i in self.data {
            if i.key == key {
                return Some(i.value);
            }
        }
        None
    }

    /// getting value as [`Cow<str>`] from pair using key
    pub fn get_as_str(&self,key:&str)->Option<Cow<str>>{
        if let Some(value) = self.get_as_bytes(key.as_bytes()){
            return Some(String::from_utf8_lossy(value));
        }
        None
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






