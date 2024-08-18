
pub struct HTTPFrameworkConfigs {
    pub addresses:Vec<String>,
    pub public_files_path:String,

}


impl HTTPFrameworkConfigs {
    pub fn default()->Self {
        HTTPFrameworkConfigs{
            addresses:vec!["0.0.0.0:8082".to_string()],
            public_files_path:"../public".to_string()
        }
    }

    pub fn bind_multi_ports(ports:Vec<u16>)->Self {
        let mut  addresses = Vec::<String>::new();
        for port in ports {
            let address = format!("0.0.0.0:{}",port);
            addresses.push(address);
        }
        Self::bind_multi_addresses(addresses)
    }
    pub fn bind_port(port:u16)->Self {
        Self::bind(&format!("0.0.0.0:{}",port))
    }
    pub fn bind(link:&str)->Self{
        HTTPFrameworkConfigs {
            addresses:vec![link.to_string()],
            ..Self::default()
        }
    }

    pub fn bind_multi_addresses(links:Vec<String>)->Self{
        HTTPFrameworkConfigs {
            addresses:links,
            ..Self::default()
        }
    }
}