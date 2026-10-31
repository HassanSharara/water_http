use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream as StdTcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use water_http::server::{PeerAction, ServerConfigurations};
use water_http::{InitControllersRoot, WaterController};

InitControllersRoot! {
    name: MAIN_ROOT,
    holder_type: MainHolderType,
}

type MainHolderType = CHolder;

#[derive(Debug, Clone)]
pub struct CHolder;

static CONNECTION_COUNT: AtomicUsize = AtomicUsize::new(0);

/// OnAcceptCallback with the exact signature:
/// for<'a> fn(&'a tokio::net::TcpStream, &'a std::net::SocketAddr) -> PeerAction
fn on_accept_check(stream: &tokio::net::TcpStream, addr: &SocketAddr) -> PeerAction {
    let count = CONNECTION_COUNT.fetch_add(1, Ordering::SeqCst) + 1;
    let peer = stream.peer_addr().ok();
    println!("\n>>> [on_accept_check] Connection #{} from {}", count, addr);
    println!("    TcpStream peer_addr: {:?}", peer);

    // Alternating rule for demonstration:
    // Odd connections: PASS
    // Even connections: BLOCK
    if count % 2 == 0 {
        println!("    Decision: [BLOCK] - Dropping connection immediately!");
        PeerAction::Block
    } else {
        println!("    Decision: [PASS]  - Allowing connection through.");
        PeerAction::Pass
    }
}

fn main() {
    let port = 8085;
    let mut config = ServerConfigurations::bind("127.0.0.1", port);

    // Register callback on ServerConfigurations
    config.set_on_accept_connection(on_accept_check);

    // Spawn a background thread to simulate live clients (PASS and BLOCK)
    std::thread::spawn(move || {
        // Wait for server to start listening
        std::thread::sleep(Duration::from_millis(600));

        println!("\n=======================================================");
        println!("=== STARTING LIVE DEMO CLIENTS ON 127.0.0.1:{} ===", port);
        println!("=======================================================");

        // --- Test 1: First connection (should PASS) ---
        println!("\n[Client 1] Connecting to server (Expect: PASS)...");
        if let Ok(mut stream) = StdTcpStream::connect(("127.0.0.1", port)) {
            let request = "GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
            let _ = stream.write_all(request.as_bytes());
            let mut buf = [0u8; 1024];
            if let Ok(n) = stream.read(&mut buf) {
                let response = String::from_utf8_lossy(&buf[..n]);
                let status_line = response.lines().next().unwrap_or("");
                println!("[Client 1] Result: SUCCESS! Received response: {}", status_line);
            }
        }

        std::thread::sleep(Duration::from_millis(300));

        // --- Test 2: Second connection (should BLOCK) ---
        println!("\n[Client 2] Connecting to server (Expect: BLOCK)...");
        if let Ok(mut stream) = StdTcpStream::connect(("127.0.0.1", port)) {
            let request = "GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
            let _ = stream.write_all(request.as_bytes());
            let mut buffer = [0u8; 128];
            match stream.read(&mut buffer) {
                Ok(0) => {
                    println!("[Client 2] Result: BLOCKED! Connection was closed immediately by server (0 bytes read).");
                }
                Ok(n) => {
                    println!("[Client 2] Read {} bytes (unexpected): {:?}", n, String::from_utf8_lossy(&buffer[..n]));
                }
                Err(e) => {
                    println!("[Client 2] Result: BLOCKED with connection error: {}", e);
                }
            }
        }

        println!("\n=======================================================");
        println!("=== LIVE TEST COMPLETED SUCCESSFULLY! ===");
        println!("Server is still running at http://127.0.0.1:{} for manual testing.", port);
        println!("Try: curl http://127.0.0.1:{}/", port);
        println!("=======================================================\n");
    });

    println!("Starting server on 127.0.0.1:{} with on_accept_connection callback...", port);
    water_http::RunServer!(
        config,
        MAIN_ROOT,
        MainController
    );
}

WaterController! {
    holder -> crate::MainHolderType,
    name -> MainController,
    functions -> {
        GET => / => hello(context) async {
            response!(context -> "Hello from water_http server with on_accept_connection!")
        }
    }
}
