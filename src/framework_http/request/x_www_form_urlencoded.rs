use std::collections::HashMap;

pub struct XWWWFormUrlEncoded {
    pub data:HashMap<String,String>,
}


impl  XWWWFormUrlEncoded {
    pub fn from_str(needle:&str)->Result<Self,String>{
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

    pub fn get(&self,key:&str)->Option<&String>{
        self.data.get(key)
    }

    pub fn insert(&mut self,key:String,value:String)->Option<String>{
        self.data.insert(key,value)
    }
}