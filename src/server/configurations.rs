use std::collections::HashMap;
#[cfg(feature = "debugging")]
use tracing::error;
use crate::server::encoding::{EncodingConfigurations};

pub (crate) const EACH_REQUEST_BODY_READING_BUFFER:usize = 1024*4;
// pub (crate) const EACH_REQUEST_BODY_WRITING_BUFFER:usize = 1024*4;
pub (crate) const READING_BUF_LEN:usize = 1024*8;
pub (crate) const WRITING_BUF_LEN:usize = 1024*8;
pub (crate) const WRITING_FILES_BUF_LEN:usize = 1024*80;



/// for saving all  named Routes
pub (crate) static mut ___ALL_ROUTES:Option<HashMap<String,String>> = None;

/// for retrieving named routes like
/// GET_categories_post => / => XX(context) async {
/// }
///
///in this example ("categories_post") is the name of this route , so we could call
/// our function [___get_from_all_routes] and parse "categories_post" as our parameter
#[doc(hidden)]
pub fn ___get_from_all_routes(key:&str,mut params:Option<HashMap<&str,&str>>)->Option<String>{
   unsafe {
       match ___ALL_ROUTES.as_ref() {
           None => {}
           Some(___a) => {
               if let Some(route) = ___a.get(key) {
                   let mut route = route.to_string();
                   loop {
                       if let Some(f_index) = route.find("{") {
                           if let Some(s_index) = (&route[f_index..]).find("}") {
                               let param = &route[f_index+1..s_index];
                               match params {
                                   None => {
                                       #[cfg(feature = "debugging")]
                                       error!("you should provide {param} with your given route {key}");
                                       return None
                                   }
                                   Some(ref mut all_params) => {
                                       if let Some(k) = all_params.get(param) {
                                           let param = param.to_string();
                                           route = route.replace(&format!("{}{}{}","{",param,"}"),*k);
                                           all_params.remove(param.as_str());
                                           continue;
                                       } else {
                                           return None;
                                       }
                                   }
                               }
                           } else {break;}
                       } else {break;}

                   }
                   match params {
                       None => {}
                       Some( p) => {
                           for (index,(key,value)) in p.iter().enumerate() {
                               if index==0 {route.push_str("?");}
                               else { route.push_str("&"); }
                               route.push_str(&format!("{key}="));
                               route.push_str(value);
                           }
                       }
                   }
                   return Some(route)
               }
           }
       }

   }
    None
}

pub (crate) fn push_named_route(name:String,route:String){
    unsafe  {
        match ___ALL_ROUTES.as_mut() {
            None => {
                let mut map = HashMap::new();
                map.insert(name,route);
                ___ALL_ROUTES = Some(map);
            }
            Some(map ) => {
                map.insert(name,route);
            }
        }
    }
}

/// specify the strict role for the server
/// to detect who is connecting our server and who is not
pub enum RestrictionRule {

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
pub struct ServerConfigurations {
    ///
    ///
    /// - set the address that your server need to bind
    /// and also the ports with them for examples [0.0.0.0:80]
    /// or [0.0.0.0:443] as tls port
    ///
    ///
    pub addresses:Vec<(String,u16)>,

    /// - specifying which ip to accept connection and which not
    pub restricted_ips:Option<RestrictionRule>,

    /// http encoding configurations ,
    /// which takes [EncodingConfigurations] struct
    /// # Note :
    /// the default value for encoding logic [`EncodingLogic::None`]
    pub (crate) responding_encoding_configurations:EncodingConfigurations,

    /// - if you need your server to support tls or ssl encryption
    /// just provide the path of your [private.key] and [certificate.cer]
    /// and also you can provide [ca_bundle.cert]
    /// and then let the framework do the rest implementations with
    /// the fastest results
    pub tls_certificate:Option<TLSCertificate>,

    /// - specify where should the system apply tls protocol on which ports
    /// the default value is ['443']
    pub tls_ports:Vec<u16>,


    ///backlog defines the maximum number of pending connections are queued by the operating system at any given time. Connection are removed from the queue with accepting connection from tcp listener When the queue is full, the operating-system will start rejecting connections.
    pub backlog:u32,
    /// defining the max size for handling single request
    pub max_request_size:usize,
    // /// setting the max headers length will specify how many headers we would read from incoming request
    // /// and the reason why we use fixed length because we need to make the allocation in the Stack
    // /// so that would give us a very fast request header reading and also for safety
    // /// as some malicious requests could have very long headers count which helping them to distract your web server
    // pub max_http1_headers_length:usize,
    // /// defining how many queries could we may serve in incoming request
    // /// and those just the queries subjected by incoming request path
    // /// # For Example :
    // /// https://wwww.example.com/post?id=1&name=2
    // /// as you see in these examples we have just two queries count
    // pub max_http1_query_length:usize,
}


/// - struct for parsing tls certificates resources files paths
///  to  [ServerConfigurations]
pub struct TLSCertificate {
    pub tls_cert:String,
    pub tls_key:String,
    pub tls_ca_bundle:Option<String>,
}



/// - configurations methods
impl ServerConfigurations {

    ///  returning default server configurations
    ///  - default port = 80
    ///  - threshold_for_encoding_response = 4000000 -> for detecting when to call encoding large data when responding to clients
    ///  - tls_ports = vec![443]
    /// # return [ ServerConfigurations]
    ///
    pub fn default()->Self {
        ServerConfigurations{
            addresses:vec![("0.0.0.0".to_string(),80),],
            tls_certificate:None,
            restricted_ips:None,
            responding_encoding_configurations:EncodingConfigurations::default(),
            tls_ports:vec![443],
            backlog:1028,
            max_request_size:10000,
        }
    }



    /// set config encoding configurations
    /// when framework responding to client
    ///
    /// if the client sent that`s he is accepting some encoding algorithms like  gzip,deflate,brotli,zstd
    /// example : inside headers "Accept-Encoding": deflate,brotli
    ///
    /// the server then may would encode the response
    /// and this decision depends on your configuration
    /// ,so you need to set [EncodingConfigurations]
    pub fn set_response_encoding_configuration(&mut self,conf:EncodingConfigurations){
        self.responding_encoding_configurations = conf;
    }


    ///
    /// # setting role to connect the server
    /// this role would be a type of `WaterIpAddressesRestriction`
    ///
    pub fn set_restriction_to_ips(&mut self,roll:RestrictionRule){
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
        ServerConfigurations {
            addresses:vec![(link.to_string(),port)],
            ..Self::default()
        }
    }


    /// # when you want to bind to multiple custom address with multiple custom ports
    /// it used when you have multiple networks,and you want to listen to custom set of them
    pub fn bind_multi_addresses(links:Vec<(String,u16)>)->Self{
        ServerConfigurations {
            addresses:links,
            ..Self::default()
        }
    }
}





