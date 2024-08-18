
pub struct HTTPFrameworkConfigs {
    pub addresses:Vec<String>,
    pub public_files_path:String,

}


impl HTTPFrameworkConfigs {
    pub fn default()->Self {
        HTTPFrameworkConfigs{
            addresses:vec!["localhost:8082".to_string()],
            public_files_path:"../public".to_string()
        }
    }
}