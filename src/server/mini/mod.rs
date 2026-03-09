#![cfg(not(feature = "use_io_uring"))]
use std::future::Future;
use std::net::{SocketAddr, ToSocketAddrs};
use integer_to_bytes::HumanInt;
use tokio::net::TcpListener;
use water_buffer::WaterBuffer;
use crate::http::request::{FormingRequestResult, IncomingRequest};
use crate::http::WaterBytes;
use crate::http::status_code::HttpStatusCode;
use crate::server::{HttpStream, ServerConfigurations, STATIC_SERVER_CONFIGURATION};
use crate::server::connection::{handle_responding, reserve_buf};
use crate::server::io::buf::{PooledBufferType, PooledWaterBuffer};

type MiniBuffer = WaterBuffer<u8>;

fn create_listener((address, port): (String, u16)) -> TcpListener {
    let conf = super::get_server_config();
    let addr_str = format!("{}:{}", address, port);
    let socket_addr = addr_str.to_socket_addrs().unwrap().next().expect("Invalid address");
    let socket = match &socket_addr {
        SocketAddr::V4(_) => tokio::net::TcpSocket::new_v4(),
        SocketAddr::V6(_) => tokio::net::TcpSocket::new_v6(),
    }.expect("Failed to create socket");

    socket.set_reuseaddr(true).ok();
    #[cfg(all(
        unix,
        not(target_os = "solaris"),
        not(target_os = "illumos"),
        not(target_os = "cygwin"),
    ))]
    socket.set_reuseport(true).ok();
    socket.set_nodelay(true).ok();
    socket.bind(socket_addr).expect("Bind failed");
    socket.listen(conf.backlog).expect("Listen failed")
}

// ── CtxPtr ────────────────────────────────────────────────────────────────────
// A Send-able raw pointer to MiniContext.
// SAFETY: MiniContext is only accessed from one task at a time.

pub struct CtxPtr<const H: usize, const Q: usize>(*mut MiniContext<H, Q>);
unsafe impl<const H: usize, const Q: usize> Send for CtxPtr<H, Q> {}

impl<const H: usize, const Q: usize> CtxPtr<H, Q> {
    /// SAFETY: caller must ensure the pointer is valid and no other
    /// reference to the same MiniContext exists for the lifetime of the
    /// returned reference.
    #[inline(always)]
    pub  fn get(&mut self) -> &mut MiniContext<H, Q> {
        unsafe {&mut *self.0}
    }
}

// ── Trait ─────────────────────────────────────────────────────────────────────

pub trait RequestHandler<const H: usize, const Q: usize>: Send + Clone + 'static {
    type Fut: Future<Output = ()> + Send + 'static;
    fn call(&self, ctx: CtxPtr<H, Q>) -> Self::Fut;
}

// ── HandlerFn wrapper ─────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct HandlerFn<F>(pub F);

impl<const H: usize, const Q: usize, F, Fut> RequestHandler<H, Q> for HandlerFn<F>
    where
        F: Fn(CtxPtr<H, Q>) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = ()> + Send + 'static,
{
    type Fut = Fut;

    fn call(&self, ctx: CtxPtr<H, Q>) -> Self::Fut {
        (self.0)(ctx)
    }
}

// ── serve ─────────────────────────────────────────────────────────────────────

pub fn serve<const H: usize, const Q: usize, Hnd: RequestHandler<H, Q>,Init,TI>(
    conf: ServerConfigurations,
    handler: Hnd,
    thread_init:Option<Init>
) where
    Init: Fn() -> TI + Send + Sync + Clone + 'static,
    TI:Future<Output=()> + Send + 'static {
    unsafe { STATIC_SERVER_CONFIGURATION = Some(conf); }
    let conf = super::get_server_config();
    let worker_threads = conf.worker_threads_count;
    let address = &conf.addresses;

    #[cfg(not(feature = "use_tokio_send"))]
    {
        let mut ws = vec![];
        #[cfg(feature = "cpu_affinity")]
        let mut cpu_cores = core_affinity::get_core_ids().unwrap_or(vec![]);
        for thread_number in 0..worker_threads {
            let address = address.clone();
            let listeners_count = conf.listeners_count;
            let handler = handler.clone();
            let thread_init_clone = thread_init.clone(); // Clone the factory, not the future

            #[cfg(feature = "cpu_affinity")]
            let core = if cpu_cores.is_empty() {
                cpu_cores = core_affinity::get_core_ids().unwrap_or(vec![]);
                cpu_cores.pop()
            }else {cpu_cores.pop()
            };
            let thread = std::thread::Builder::new()
                .name(format!("water_thread [{}]", thread_number))
                .spawn(move || {
                    #[cfg(feature = "cpu_affinity")]
                    {
                        if let Some(c) = core {
                            core_affinity::set_for_current(c);
                        }
                    }
                    let handler = handler.clone();
                    let fut = async move {
                        if let Some(init_factory) = thread_init_clone {
                            init_factory().await;
                        }
                        for add in address {
                            let handler = handler.clone();
                            for _ in 0..listeners_count {
                                let handler = handler.clone();
                                let listener = create_listener(add.clone());
                                tokio::task::spawn_local(handle_listener(listener, handler));
                            }
                        }
                        std::future::pending::<()>().await
                    };

                    #[cfg(tokio_unstable)]
                    {
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build_local(Default::default())
                            .unwrap();
                        rt.block_on(fut);
                        return;
                    }

                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("cannot create tokio runtime");
                    let local_set = tokio::task::LocalSet::new();
                    local_set.block_on(&rt, fut);
                })
                .unwrap();
            ws.push(thread);
        }
        for t in ws {
            let _ = t.join();
        }
    }
}

// ── internal ──────────────────────────────────────────────────────────────────

async fn handle_listener<const H: usize, const Q: usize, Hnd: RequestHandler<H, Q> + Clone + 'static>(
    listener: TcpListener,
    handler: Hnd,
) {
    loop {
        if let Ok((stream, add)) = listener.accept().await {
            tokio::task::spawn_local(handle_connection(
                HttpStream::Async(stream),
                add,
                handler.clone(),
            ));
        }
    }
}

#[inline(always)]
unsafe fn erase_request_lifetime<const H: usize, const Q: usize>(
    req: IncomingRequest<'_, H, Q>,
) -> IncomingRequest<'static, H, Q> {
    std::mem::transmute(req)
}

async fn handle_connection<const H: usize, const Q: usize, Hnd: RequestHandler<H, Q>>(
    mut stream: HttpStream,
    _add: SocketAddr,
    handler: Hnd,
) {
    let mut reading_buffer = PooledWaterBuffer::new(PooledBufferType::Read).take_inner();
    let mut response_buffer = PooledWaterBuffer::new(PooledBufferType::Write).take_inner();

    'main_loop: loop {
        reserve_buf(&mut reading_buffer);
        if let Ok(r) = stream.read(&mut reading_buffer).await {
            if r == 0 { break 'main_loop; }
            reading_buffer.advance_mut(r);

            loop {
                let bytes = reading_buffer.chunk();
                let req = IncomingRequest::<H, Q>::new(bytes);

                match req {
                    FormingRequestResult::Success(req) => {
                        let total_headers = req.get_total_headers_length();
                        {
                            let req_static = unsafe { erase_request_lifetime(req) };
                            let mut mini_context = MiniContext::<H, Q>::new(
                                unsafe { response_buffer.unsafe_clone() },
                                req_static,
                            );
                            // SAFETY: mini_context lives until after the
                            // future is awaited. CtxPtr does not outlive
                            // this block.
                            let ctx_ptr = CtxPtr(&mut mini_context as *mut _);
                            let _ = handler.call(ctx_ptr).await;
                        }
                        reading_buffer.advance(total_headers);
                        if reading_buffer.is_empty() { break ; }
                    }
                    FormingRequestResult::ReadMore => break,
                    FormingRequestResult::Err(_) => break 'main_loop,
                }
            }

            if !response_buffer.is_empty() {
                if handle_responding(unsafe{response_buffer.unsafe_clone()}, &mut stream).await.is_err() {
                    break 'main_loop;
                }
            }
        } else {

            break 'main_loop;
        }
    }
    if !response_buffer.is_empty() {
        let _ = handle_responding(unsafe{response_buffer.unsafe_clone()}, &mut stream).await;
    }
    PooledWaterBuffer::recycle(reading_buffer, PooledBufferType::Read);
    PooledWaterBuffer::recycle(response_buffer, PooledBufferType::Write);
}

// ── MiniContext ───────────────────────────────────────────────────────────────

pub struct MiniContext<const H: usize, const Q: usize> {
    response_buffer: MiniBuffer,
    request: IncomingRequest<'static, H, Q>,
    headers_respond_status: ResponseStatus,
}

impl<const H: usize, const Q: usize> MiniContext<H, Q> {
    fn new(response_buffer: MiniBuffer, request: IncomingRequest<'static, H, Q>) -> Self {
        MiniContext {
            response_buffer,
            request,
            headers_respond_status: ResponseStatus::None,
        }
    }

    pub fn set_status_code(&mut self, status: HttpStatusCode) {
        let buf = &mut self.response_buffer;
        buf.extend_from_slice(b"HTTP/1.1 ");
        status.status.get().put_into(buf);
        buf.extend_from_slice(b" ");
        buf.extend_from_slice(status.label.as_bytes());
        buf.extend_from_slice(b"\r\n");
        self.headers_respond_status = ResponseStatus::JustFirstLine;
    }

    #[inline]
    fn write_header<'b>(&mut self, key: impl Into<WaterBytes<'b>>, value: impl Into<WaterBytes<'b>>) {
        let buf = &mut self.response_buffer;
        key.into().push_to(buf);
        buf.extend_from_slice(b": ");
        value.into().push_to(buf);
        buf.extend_from_slice(b"\r\n");
    }

    #[inline]
    pub fn set_header<'b>(&mut self, key: impl Into<WaterBytes<'b>>, value: impl Into<WaterBytes<'b>>) {
        match self.headers_respond_status {
            ResponseStatus::JustFirstLine => {
                self.headers_respond_status = ResponseStatus::JustHeadersSet;
                self.write_header(key, value)
            },
            ResponseStatus::JustHeadersSet => self.write_header(key,value),
            ResponseStatus::None => {
                self.set_status_code(HttpStatusCode::OK);
                self.headers_respond_status = ResponseStatus::JustHeadersSet;
                self.write_header(key, value);
            }
            _ => panic!("you cant write headers after responding"),
        }
    }

    #[inline]
    pub fn write_body_bytes(&mut self, bytes: &[u8]) {
        match self.headers_respond_status {
            ResponseStatus::JustFirstLine => panic!("you should write headers first"),
            ResponseStatus::JustHeadersSet => {
                self.headers_respond_status = ResponseStatus::WhileSendingResponse;
                self.response_buffer.extend_from_slice(b"\r\n");
                self.response_buffer.extend_from_slice(bytes);
            }
            ResponseStatus::WhileSendingResponse => {
                self.response_buffer.extend_from_slice(bytes);
            }
            ResponseStatus::Done => panic!("you cant write response twice"),
            ResponseStatus::None => unreachable!(),
        }
    }




    #[inline(always)]
    pub fn done(&mut self) {
        self.headers_respond_status = ResponseStatus::Done;
    }

    pub fn request(&self) -> &IncomingRequest<'static, H, Q> {
        &self.request
    }
}

enum ResponseStatus {
    JustFirstLine,
    JustHeadersSet,
    WhileSendingResponse,
    Done,
    None,
}