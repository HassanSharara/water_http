use std::collections::HashMap;


/// a struct for handling Content or the body of request
/// when the requested body was a type of x-www-form data
/// then it would be handled using [XWWWFormUrlEncoded] struct
/// which hold data
pub struct XWWWFormUrlEncoded {
    /// data that have been requested with x-www-form
    pub data:HashMap<String,String>,
}


impl  XWWWFormUrlEncoded {
    pub  fn from_str(needle:&str)->Result<Self,String>{
        let mut data :HashMap<String,String> = HashMap::new();
        let str = needle.split(",");
        for child in str {
            let mut key_v_splitter = child.split("=");
            let key = key_v_splitter.next();
            let value = key_v_splitter.next();
           if let Some(key) = key {
               if let Some(value) = value {
                   data.insert(key.to_string(),value.to_string());
               }
           }
        }

        if !data.is_empty() {
            return  Ok(XWWWFormUrlEncoded{
                data
            });
        }
        Err("can not create x-www-form-urlencoded".to_string())
    }

    /// for getting values from requested data by using their keys
    pub fn get(&self,key:&str)->Option<&String>{
        self.data.get(key)
    }

    /// if you want to insert custom key value data to your body to be handled and carried by
    /// this struct you could use insert function
    pub fn insert(&mut self,key:String,value:String)->Option<String>{
        self.data.insert(key,value)
    }
}