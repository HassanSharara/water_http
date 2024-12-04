use std::path::Path;
use tokio_rustls::rustls::pki_types::{CertificateDer,PrivateKeyDer};
use rustls_pemfile::{certs, private_key};
use std::io::{self, BufReader, ErrorKind};
use std::fs::File;
use crate::server::TLSCertificate;


pub fn generate_tls_configurations(config:&TLSCertificate)->Result<rustls::server::ServerConfig,()>{
    let certs = load_certs(Path::new(&config.tls_cert));
    let key = load_key(Path::new(&config.tls_key));
    if let Ok( mut certs ) = certs {
        if let Some(tls_bundle) = &config.tls_ca_bundle {
            let ca_bundle = load_certs(Path::new(&tls_bundle));
            if let Ok(mut ca_bundle) = ca_bundle {
                certs.append(&mut ca_bundle);
            }
        }

       if let Ok(key) = key {
           let config = rustls::ServerConfig::builder()
               .with_no_client_auth()
               .with_single_cert(certs, key);

             if let Ok(mut config) = config {
                 config.alpn_protocols.push(b"h2".to_vec());
                 return Ok(config);
             }
       }
    }
    Err(())
}

fn load_certs(path: &Path) -> io::Result<Vec<CertificateDer<'static>>> {
    certs(&mut BufReader::new(File::open(path)?)).collect()
}

fn load_key(path: &Path) -> io::Result<PrivateKeyDer<'static>> {
    Ok(private_key(&mut BufReader::new(File::open(path)?))
        .unwrap()
        .ok_or(io::Error::new(
            ErrorKind::Other,
            "no private key found".to_string(),
        ))?)
}