
pub struct HTTPFrameworkConfigs {
    pub addresses:Vec<String>,
    pub public_files_path:String,
    pub tls_certificate:Option<TLSCertificate>
}

pub struct TLSCertificate {
    pub tls_cert:String,
    pub tls_key:String,
}
impl HTTPFrameworkConfigs {
    pub fn default()->Self {
        HTTPFrameworkConfigs{
            addresses:vec!["0.0.0.0:80".to_string()],
            public_files_path:"../public".to_string(),
            tls_certificate:None
        }
    }

    pub fn set_tls_certificate(&mut self,cert_path:&str,key_path:&str){
        self.tls_certificate = Some(
            TLSCertificate {
                tls_cert:cert_path.to_string(),
                tls_key:key_path.to_string()
            }
        );
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