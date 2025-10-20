use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use bytes::{Buf, BufMut, BytesMut};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadBuf};
use crate::http::request::{FormingRequestResult, IncomingRequest};
use crate::server::connection::{BodyReadingBuffer, reserve_buf};
use crate::server::{CapsuleWaterController, EACH_REQUEST_BODY_READING_BUFFER, Http1Context, HttpContext, HttpStream, Protocol, READING_BUF_LEN, ServingRequestResults, WRITING_BUF_LEN};
use crate::server::Protocol::Http1;

pub struct WaterTcpStream<'a,'b> {
    stream: &'a mut HttpStream,
    read_buf: &'a mut ReadBuf<'b>,
    write_buf: &'a mut BytesMut,
    body_reading_buffer: &'a mut BodyReadingBuffer,
    peer:&'a SocketAddr,
}


#[derive(Debug)]
pub enum PollResults {
    ReadSuccess(usize),
    ReadErr,
    WriteSuccess(usize),
    WriteErr,
}

impl<'a,'b> WaterTcpStream<'a,'b> {
    #[inline(always)]
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<PollResults> {
        // get mutable access to the pinned `self` without creating multiple &mut borrows
        let this = unsafe { self.get_unchecked_mut() };
        // raw pointer to the `HttpStream` field to avoid simultaneous &mut borrows of `this`
        let stream_ptr: *mut HttpStream = this.stream as *mut _;

        // perform variant dispatch and calls inside an unsafe block using the raw pointer
        unsafe {
            match &mut *stream_ptr {
                #[cfg(feature = "support_tls")]
                HttpStream::AsyncSecure(stream) => {
                    match stream.poll_write(cx, this.write_buf) {
                        Poll::Ready(r) => match r {
                            Ok(n) => {
                                if n >= this.write_buf.len() {
                                    this.write_buf.clear();
                                } else {
                                    this.write_buf.advance(n);
                                }
                                return Poll::Ready(PollResults::WriteSuccess(n));
                            }
                            Err(_) => {}
                        },
                        Poll::Pending => {}
                    }
                }
                HttpStream::Async(stream) => {
                    match Pin::new_unchecked(stream).poll_write(cx, &mut this.write_buf) {
                        Poll::Ready(r) => match r {
                            Ok(n) => {
                                if n >= this.write_buf.len() {
                                    this.write_buf.clear();
                                } else {
                                    this.write_buf.advance(n);
                                }
                                return Poll::Ready(PollResults::WriteSuccess(n));
                            }
                            Err(_) => {}
                        },
                        _ => {}
                    }
                    return Poll::Pending;
                }
            }
        }

        Poll::Pending
    }
    #[inline(always)]
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<PollResults> {
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
                        Poll::Ready(PollResults::ReadSuccess(filled.len()))
                    },                    Poll::Ready(Err(_)) => Poll::Ready(PollResults::ReadErr),
                    Poll::Pending => Poll::Pending,
                },
                HttpStream::Async(s) => match Pin::new_unchecked(s).poll_read(cx, this.read_buf) {
                    Poll::Ready(Ok(_)) => {
                        let filled = unsafe {(&mut *this).read_buf.filled()};
                        Poll::Ready(PollResults::ReadSuccess(filled.len()))
                    },
                    Poll::Ready(Err(_)) => Poll::Ready(PollResults::ReadErr),
                    Poll::Pending => Poll::Pending,
                },
            }
        }}
    #[inline(always)]
    fn poll_read_ready(mut self:Pin<&mut Self>,cx:&mut Context<'_>)->Poll<PollResults>{
        return match &mut self.stream {
            #[cfg(feature = "support_tls")]
            HttpStream::AsyncSecure(s) => {
                _= s.get_ref().0.poll_read_ready(cx);
                Poll::Pending
            }
            HttpStream::Async(s) => {
                _= s.poll_read_ready(cx);
                Poll::Pending
            }
        }
    }

}






impl<'a,'b> WaterTcpStream<'a,'b> {
    pub(crate) fn new(stream: &'a mut HttpStream,
                      read_buf:&'a mut ReadBuf<'b>,
                      write_buf:&'a mut BytesMut,
                      body_reading_buffer:&'a mut BodyReadingBuffer,
                      peer:&'a SocketAddr
    ) -> Self {
        Self {
            stream,
            read_buf,
            write_buf,
            body_reading_buffer,
            peer,
        }
    }



    pub async fn serve<Holder:Send + 'static, const HS:usize, const QS:usize>(
        hs: &mut HttpStream,
        peer: &SocketAddr,
        controller: &'static CapsuleWaterController<Holder, HS, QS>
    ) {
        let mut read_buf = BytesMut::with_capacity(READING_BUF_LEN);
        let mut write_buf = BytesMut::with_capacity(WRITING_BUF_LEN);
        let mut body_buf = BodyReadingBuffer::with_capacity(EACH_REQUEST_BODY_READING_BUFFER);

        loop {
            // Only read if we need more data
            if read_buf.len() < 16 || matches!(
            IncomingRequest::<HS, QS>::new(&read_buf),
            FormingRequestResult::ReadMore
        ) {
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

                let mut water_stream = WaterTcpStream::new(
                    hs, &mut rdr, &mut write_buf, &mut body_buf, peer
                );

                let exec = WaterTcpReader { stream: &mut water_stream };
                match exec.await {
                    PollResults::ReadSuccess(_) => {
                        let filled_len = rdr.filled().len();
                        if filled_len > 0 {
                            // Mark the bytes as initialized in BytesMut
                            unsafe { read_buf.advance_mut(filled_len); }
                        }
                    }
                    PollResults::ReadErr => return,
                    PollResults::WriteSuccess(_) => continue,
                    PollResults::WriteErr => return,
                    _ => continue,
                }
            }

            if read_buf.is_empty() {
                continue;
            }

            let req = IncomingRequest::<HS, QS>::new(&read_buf);

            match req {
                FormingRequestResult::ReadMore => continue,
                FormingRequestResult::Err(_) => return,
                FormingRequestResult::Success(request) => {
                    let total_req_size = request.get_total_headers_length();
                    let left_bytes = &read_buf[total_req_size..];

                    let mut context = HttpContext::<Holder, HS, QS>::new(
                        Http1(Http1Context::new(hs, &mut write_buf, &mut body_buf, left_bytes, request)),
                        peer
                    );

                    match context.serve(controller).await {
                        ServingRequestResults::Stop => return,
                        ServingRequestResults::Done => {
                            read_buf.advance(total_req_size);
                        }
                    }
                }
            }
        }
    }



}


pub (crate) struct  WaterTcpReader<'a,'b>{
    stream:&'a mut WaterTcpStream<'a,'b>,
}

impl Future for WaterTcpReader<'_, '_> {
    type Output = PollResults;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // get mutable access to the pinned `self`
        let this = unsafe { self.get_unchecked_mut() };
        // obtain a raw pointer to the inner `WaterTcpStream` (does not move the field)
        let stream_ptr: *mut WaterTcpStream<'_, '_> = this.stream as *mut _;
        // create a Pin<&mut WaterTcpStream> from the raw pointer and call the inherent poll methods.
        // Using `new_unchecked` is required because we pinned `WaterTcpStream` by contract outside.
        let read_results = unsafe { Pin::new_unchecked(&mut *stream_ptr) }.poll_read(cx);
        match read_results  {
            Poll::Ready(r) => match r {
                PollResults::ReadSuccess(n) => {
                    if  n > 0 { return  Poll::Ready(PollResults::ReadSuccess(n)) }
                },
                PollResults::ReadErr => {return  Poll::Ready(PollResults::ReadErr)},
                _ => unreachable!(),
            },
            _ => {
                let write_length = (unsafe{&*stream_ptr}).write_buf.len();
                let read_filled_len = unsafe { &*stream_ptr }.read_buf.filled().is_empty();
                if  write_length > 0 && read_filled_len  {
                    // println!("writing {:?}",(unsafe{&*stream_ptr}).write_buf.len());
                    if let Poll::Ready(PollResults::WriteErr) =
                        unsafe { Pin::new_unchecked(&mut *stream_ptr) }.poll_write(cx)
                    {
                        return Poll::Ready(PollResults::WriteErr);
                    }
                }
            }
        }


        Poll::Pending
    }
}
























