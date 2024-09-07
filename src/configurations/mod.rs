

/// specify the strict role for the server
/// to detect who is connecting our server and who is not
pub enum WaterIpAddressesRestriction {

    ///
    /// when you specify  restricted ips
    /// the server will only allow these ip address to connect
    /// and abort others
    ///

     OnlyAllowedIps(Vec<String>),

    ///
    /// when you specify  blacklist ips
    /// the server will reject serving them
    /// notice that when you specify
    ///
     BlacklistIps(Vec<String>),
}

/// a struct for configurations of the server
/// we could specify which port we are listening to
/// and who is going to connect to your server and who is restricted
/// , where the public files exist
/// , do you need to even check on these public files or not
/// , setting tls connection configuration
/// specify the tls ports and threshold of encoding data algorithm
/// and also specify the important headers to retrieve for optimizing requests while
/// handling a lot of them
pub struct WaterServerConfigurations {
    ///
    ///
    /// - set the address that your server need to bind
    /// and also the ports with them for example [0.0.0.0:80]
    /// or [0.0.0.0:443] as tls port
    ///
    ///
    pub addresses:Vec<(String,u16)>,

    /// - specifying which ip to accept connection and which not
    pub restricted_ips:Option<WaterIpAddressesRestriction>,

    ///
    /// - specify where the public resources exist so that you could
    /// use [context.send_file_from_public_resources( --the path of your file inside the public resources-- )]
    /// and then the file will be detected and sent automatically
    ///
    pub public_files_path:String,
    /// - if you do not want your server to return files from public directory
    /// just set this property to true
    pub do_not_even_check_public_resources:bool,
    /// - if you need your server to support tls or ssl encryption
    /// just provide the path of your [private.key] and [certificate.cer]
    /// and also you can provide [ca_bundle.cert]
    /// and then let the framework do the rest implementations with
    /// the fastest results
    pub tls_certificate:Option<TLSCertificate>,

    /// - specify where should the system apply tls protocol on which ports
    /// the default value is ['443']
    pub tls_ports:Vec<u16>,
    /// - if you want to read only specific headers in during application life
    /// to optimize performance you can use this option
    /// if provide [Some(vec![])] which means empty vector
    /// the framework will automatically read the most 7 important headers
    /// if you want to read all the incoming headers leave it as default [None]
    pub headers_for_reading:Option<Vec<String>>,
    /// - this framework support encoding with all encoding algorithms
    /// ['zstd,Gzip,Deflate,Brotli'] so the response will be compressed with one of these
    /// algorithms depending on the threshold of the data you need to send
    /// so the default value is 4000000 which is approximately [4 MB ]
    /// so if your server is very close to your clients leave this value as default but
    /// if your server is a little far from your client then try to decrease this threshold
    /// to get the best response latency
    /// also notice that when you send a custom headers you should implement this encoding manually
    pub threshold_for_encoding_response:u64
}


/// - struct for parsing tls certificates resources files paths
///  to  [WaterServerConfigurations]
pub struct TLSCertificate {
    pub tls_cert:String,
    pub tls_key:String,
    pub tls_ca_bundle:Option<String>,
}


/// - configurations methods
impl WaterServerConfigurations {

    ///  returning default server configurations
    ///  - default port = 80
    ///  - public_files_path = "./public"
    ///  - threshold_for_encoding_response = 4000000 -> for detecting when to call encoding large data when responding to clients
    ///  - tls_ports = vec![443]
    /// # return [ WaterServerConfigurations]
    ///
    pub fn default()->Self {
        WaterServerConfigurations{
            addresses:vec![("0.0.0.0".to_string(),80),],
            public_files_path:"./public".to_string(),
            tls_certificate:None,
            restricted_ips:None,
            headers_for_reading:None,
            do_not_even_check_public_resources:false,
            threshold_for_encoding_response:4000000,
            tls_ports:vec![443]
        }
    }

    ///
    /// # setting role to connect the server
    /// this role would be a type of [WaterIpAddressesRestriction]
    ///
    pub fn set_restriction_to_ips(&mut self,roll:WaterIpAddressesRestriction){
        self.restricted_ips = Some(roll);
    }


    /// # creating [TLSCertificate] from certificate path and private key path
    /// - (optional) also you could provide bundle path
    pub fn set_tls_certificate(&mut self,cert_path:&str,key_path:&str,bundle:Option<String>){
        self.tls_certificate = Some(
            TLSCertificate {
                tls_cert:cert_path.to_string(),
                tls_key:key_path.to_string(),
                tls_ca_bundle:bundle
            }
        );
    }

    /// # when you want to listen to multiple ports at the default ip
    /// it would be a good option also for ssl or tls cause all of them are listening at port 443
    pub fn bind_multi_ports(ports:Vec<u16>)->Self {
        let mut  addresses = Vec::<(String,u16)>::new();
        for port in ports {
            addresses.push(("0.0.0.0".to_string(),port));
        }
        Self::bind_multi_addresses(addresses)
    }

    /// # when you want to bind to single port
    /// when your server need to listen to one single port
    /// at the default ip address which it 0.0.0.0
    pub fn bind_port(port:u16)->Self {
        Self::bind("0.0.0.0",port)
    }

    /// # when you want to bind to single ip address with a single port
    /// it used to bind to custom ip address like 127.0.0.1 with custom single port like 8888
    pub fn bind(link:&str,port:u16)->Self{
        WaterServerConfigurations {
            addresses:vec![(link.to_string(),port)],
            ..Self::default()
        }
    }


    /// # when you want to bind to multiple custom address with multiple custom ports
    /// it used when you have multiple networks,and you want to listen to custom set of them
    pub fn bind_multi_addresses(links:Vec<(String,u16)>)->Self{
        WaterServerConfigurations {
            addresses:links,
            ..Self::default()
        }
    }
}