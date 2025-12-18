use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use bytes::{Buf, BufMut, BytesMut};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use crate::http::request::{FormingRequestResult, IncomingRequest};
use crate::server::connection::{BodyReadingBuffer, reserve_buf};
use crate::server::{CapsuleWaterController, EACH_REQUEST_BODY_READING_BUFFER, Http1Context, HttpContext, HttpStream, READING_BUF_LEN, ServingRequestResults, WRITING_BUF_LEN};
use crate::server::matcher::Matcher;

pub struct WaterTcpStream<'a,'b> {
    stream: &'a mut HttpStream,
    read_buf: &'a mut ReadBuf<'b>,
    write_buf: &'a mut BytesMut,
}


#[derive(Debug)]
pub enum PollReadResults {
    ReadSuccess(usize),
    ReadErr,
}

impl<'a,'b> WaterTcpStream<'a,'b> {

    #[allow(unused)]
    #[inline(always)]
    fn is_write_buf_empty(&self)->bool{
        self.write_buf.is_empty()
    }
    #[inline(always)]
    fn poll_write( self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<PollWriteResults> {
        let this = unsafe { self.get_unchecked_mut() };
        let stream_ptr: *mut HttpStream = this.stream as *mut _;

        #[cfg(feature = "debugging")]
        {
            println!("\n Writing Response: \n trying to write response \n {:?} \n end -- ",
                     String::from_utf8_lossy(this.write_buf)
            );
        }
        unsafe {
            match &mut *stream_ptr {
                #[cfg(feature = "support_tls")]
                HttpStream::AsyncSecure(stream) => {
                    match stream.poll_write(cx, this.write_buf) {
                        Poll::Ready(Ok(n)) => {
                            this.write_buf.advance(n);
                            if this.write_buf.is_empty() {
                                this.write_buf.clear();
                            }
                            Poll::Ready(PollWriteResults::WriteSuccess(n))
                        }
                        Poll::Ready(Err(_)) => Poll::Ready(PollWriteResults::WriteErr),
                        Poll::Pending => Poll::Pending,
                    }
                }
                HttpStream::Async(stream) => {
                    match Pin::new_unchecked(stream).poll_write(cx, &this.write_buf) {
                        Poll::Ready(Ok(n)) => {
                            this.write_buf.advance(n);
                            if this.write_buf.is_empty() {
                                this.write_buf.clear();
                            }
                            Poll::Ready(PollWriteResults::WriteSuccess(n))
                        }
                        Poll::Ready(Err(_)) => Poll::Ready(PollWriteResults::WriteErr),
                        Poll::Pending => Poll::Pending,
                    }
                }
            }
        }
    }


    #[inline(always)]
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<PollReadResults> {
        // obtain a mutable reference to the pinned `self`
        let this = unsafe { self.get_unchecked_mut() };
        // raw pointer to the HttpStream field to avoid moving the field out
        let stream_ptr: *mut HttpStream = this.stream as *mut _;

        // match on the inner stream via the raw pointer and call its poll_read without moving
        unsafe {
            match &mut *stream_ptr {
                #[cfg(feature = "support_tls")]
                HttpStream::AsyncSecure(s) => match Pin::new_unchecked(s).poll_read(cx, this.read_buf) {
                    Poll::Ready(Ok(_)) => {
                        let filled = unsafe {(&mut *this).read_buf.filled()};
                        Poll::Ready(PollReadResults::ReadSuccess(filled.len()))
                    },                    Poll::Ready(Err(_)) => Poll::Ready(PollReadResults::ReadErr),
                    Poll::Pending => Poll::Pending,
                },
                HttpStream::Async(s) => match Pin::new_unchecked(s).poll_read(cx, this.read_buf) {
                    Poll::Ready(Ok(_)) => {
                        let filled =  (&mut *this).read_buf.filled();
                        #[cfg(feature = "debugging")]
                        {
                            println!("read {:?} ",String::from_utf8_lossy(filled));
                        }
                        Poll::Ready(PollReadResults::ReadSuccess(filled.len()))
                    },
                    Poll::Ready(Err(_)) => Poll::Ready(PollReadResults::ReadErr),
                    Poll::Pending => Poll::Pending,
                },
            }
        }}


}






impl<'a,'b> WaterTcpStream<'a,'b> {
    pub(crate) fn new(stream: &'a mut HttpStream,
                      read_buf:&'a mut ReadBuf<'b>,
                      write_buf:&'a mut BytesMut,

    ) -> Self {
        Self {
            stream,
            read_buf,
            write_buf,
        }
    }


    #[inline(always)]
    pub (crate) async fn serve<
        #[cfg(feature = "use_tokio_send")]
        Holder:Send + 'static,
        #[cfg(not(feature = "use_tokio_send"))]
        Holder,
        #[cfg(all(feature = "thread_shared_struct",not(feature = "use_tokio_send")))]
        SHARED:Clone,


        #[cfg(all(feature = "thread_shared_struct",feature = "use_tokio_send"))]
        SHARED:Clone + Send + 'static,
        const HS:usize, const QS:usize>(
        hs: &mut HttpStream,
        peer: &SocketAddr,
        #[cfg(feature = "thread_shared_struct")]
        _controller: &'static CapsuleWaterController<Holder,SHARED, HS, QS>,
        #[cfg(not(feature = "thread_shared_struct"))]
        _controller: &'static CapsuleWaterController<Holder, HS, QS>,
        #[cfg(feature = "thread_shared_struct")]
        shared_factory:SHARED,
        #[cfg(feature = "thread_shared_struct")]
        matcher:Matcher<Holder,SHARED,HS,QS>,
        #[cfg(not(feature = "thread_shared_struct"))]
        matcher:Matcher<Holder,HS,QS>
    ) {
        let mut read_buf = BytesMut::with_capacity(READING_BUF_LEN);
        let mut write_buf = BytesMut::with_capacity(WRITING_BUF_LEN);
        let mut body_buf = BodyReadingBuffer::with_capacity(EACH_REQUEST_BODY_READING_BUFFER);
        let mut water_stream;
        let mut rem = 0usize;
        'main_loop: loop {

            // Only read if we need more data
            // Ensure we have space to read into
            reserve_buf(&mut read_buf);
            // Convert UninitSlice to [MaybeUninit<u8>]
            let uninit_slice = read_buf.chunk_mut();
            let uninit_buf: &mut [std::mem::MaybeUninit<u8>] = unsafe {
                std::slice::from_raw_parts_mut(
                    uninit_slice.as_mut_ptr() as *mut std::mem::MaybeUninit<u8>,
                    uninit_slice.len()
                )
            };

            let mut rdr = ReadBuf::uninit(uninit_buf);



            water_stream = WaterTcpStream::new(
                hs, &mut rdr,
                &mut write_buf,
            );

            let stream_ptr: *mut WaterTcpStream<'_, '_> = &mut water_stream;

            unsafe {
                let exec = WaterTcpReader { stream: &mut *stream_ptr }.await;
                match exec {
                    PollReadResults::ReadSuccess(u) => {
                        if u > 0 {
                            read_buf.advance_mut(u);
                            if rem >  0 {
                                if rem >= u {
                                    read_buf.clear();
                                    rem -= u;
                                    continue;
                                }
                                else {
                                    read_buf.advance(rem);
                                    rem = 0;
                                }
                            }
                        }
                    }
                    PollReadResults::ReadErr => break,
                }


            }
            loop {
                let req = IncomingRequest::<HS, QS>::new(&read_buf);
                match req {
                    FormingRequestResult::ReadMore => {
                        break
                    },
                    FormingRequestResult::Err(_) => return,
                    FormingRequestResult::Success(request) => {

                        let total_req_size = request.get_total_headers_length();
                        let left_bytes = &read_buf[total_req_size..];

                        #[cfg(feature = "thread_shared_struct")]
                            let mut context = HttpContext::<Holder,SHARED, HS, QS>::new(
                            crate::server::sr_context::Protocol::Http1(Http1Context::new(hs, &mut write_buf, &mut body_buf, left_bytes, request)),
                            peer
                        );
                        #[cfg(feature = "thread_shared_struct")]
                        {
                            context.thread_shared_struct = Some(shared_factory.clone());
                        }

                        #[cfg(not(feature = "thread_shared_struct"))]
                            let mut context = HttpContext::<Holder, HS, QS>::new(
                            crate::server::sr_context::Protocol::Http1(Http1Context::new(hs, &mut write_buf, &mut body_buf, left_bytes, request)),
                            peer
                        );


                        match context.serve_ef(matcher.clone()).await {
                            ServingRequestResults::Stop => return,
                            ServingRequestResults::Done => {
                                let content_length = context.content_length();
                                match content_length {
                                    None => {
                                        if total_req_size >= read_buf.chunk().len() {
                                            read_buf.clear();
                                        }
                                        else {
                                            #[cfg(feature = "accept_transfer_chunked")]
                                            {
                                                if let Some(h) = context.get_from_headers_as_bytes("Transfer-Encoding") {
                                                    if h == b"chunked" {
                                                        match &context.protocol {
                                                            #[cfg(not(feature = "use_only_http1"))]
                                                            crate::server::sr_context::Protocol::Http2(_) => {}
                                                            crate::server::sr_context::Protocol:: Http1(h1) => {
                                                                if let Some(to_advance ) = h1.to_advance {
                                                                    read_buf.advance(total_req_size + to_advance);
                                                                    continue;
                                                                }
                                                                else if !body_buf.is_empty() {

                                                                    read_buf.advance(total_req_size);
                                                                    read_buf.extend_from_slice(body_buf.chunk());
                                                                    body_buf.clear();
                                                                    continue;
                                                                }
                                                            }
                                                        }
                                                        read_buf.clear();
                                                        break;
                                                    }
                                                }
                                            }
                                            read_buf.advance(total_req_size);
                                        }
                                    }
                                    Some(c) => {
                                        let c = c.clone();

                                        read_buf.advance(total_req_size );
                                        if read_buf.len() >= c {
                                            read_buf.advance(c);
                                            if read_buf.is_empty() {continue 'main_loop}
                                            continue;
                                        }
                                        rem = c;
                                        if body_buf.bytes_consumed > 0 {
                                            rem-=read_buf.len();
                                            if body_buf.extended_bytes > 0 {
                                                body_buf.advance(body_buf.extended_bytes);
                                                body_buf.extended_bytes = 0;
                                            }

                                            if rem > 0  {
                                                let to_advance_body_buf = body_buf.len().min(rem);
                                                body_buf.advance(to_advance_body_buf);
                                            }
                                            rem -= body_buf.bytes_consumed.min(rem);

                                            read_buf.clear();
                                            if  !body_buf.is_empty() {
                                                read_buf.extend_from_slice(body_buf.chunk())
                                            }
                                            body_buf.reset();
                                        }
                                        if read_buf.is_empty() {continue 'main_loop;}
                                    }
                                }

                            }
                        }
                    }
                }
            }
        }

        // println!("system calls counter {:?} for address {:?}",system_calls_counter,peer);
    }


}


pub (crate) struct  WaterTcpReader<'a,'b>{
    stream:&'a mut WaterTcpStream<'a,'b>,
}

impl Future for WaterTcpReader<'_, '_> {
    type Output = PollReadResults;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let stream_ptr: *mut WaterTcpStream<'_, '_> = this.stream as *mut _;

        let mut_stream = unsafe {&mut *stream_ptr};
        // Try read first
        let read_results = unsafe { Pin::new_unchecked(mut_stream) }.poll_read(cx);
        if let Poll::Ready(r) = read_results {
             match r {
                PollReadResults::ReadSuccess(n) if n > 0 => {return Poll::Ready(PollReadResults::ReadSuccess(n))},
                PollReadResults::ReadErr => {return Poll::Ready(PollReadResults::ReadErr)},
                _ => {  },
            };
        }
        let st = unsafe {&mut *stream_ptr};
        if  !st.write_buf.is_empty() {
            match unsafe {Pin::new_unchecked(&mut *stream_ptr)}.poll_write(cx) {
                Poll::Ready(r) => {
                    if let PollWriteResults::WriteErr = r {
                        return Poll::Ready(PollReadResults::ReadErr)
                    }
                }
                Poll::Pending => {}
            }
        }
        return Poll::Pending
    }
}


#[allow(unused)]
pub (crate) struct  WaterTcpWriter<'a,'b>{
    stream:&'a mut WaterTcpStream<'a,'b>,
}

#[allow(unused)]
pub (crate) enum PollWriteResults {
    WriteSuccess(usize),
    WriteErr,
}

impl<'a,'b> Future for WaterTcpWriter<'a,'b> {

    type Output = PollWriteResults;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let stream_ptr: *mut WaterTcpStream<'_, '_> = this.stream as *mut _;
        match unsafe { Pin::new_unchecked(&mut *stream_ptr).poll_write(cx) } {
            Poll::Ready(r)=> {
                return match r {
                    PollWriteResults::WriteSuccess(n) => {
                        Poll::Ready(PollWriteResults::WriteSuccess(n))
                    },
                    PollWriteResults::WriteErr => {
                        Poll::Ready(PollWriteResults::WriteErr)
                    },
                }
            }
            Poll::Pending=> {}
        }
        return  Poll::Pending
    }
}

