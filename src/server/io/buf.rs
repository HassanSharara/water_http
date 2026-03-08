use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
// Assuming water_buffer crate exists as per your snippet
use water_buffer::WaterBuffer as BM;

type WaterBuffer = BM<u8>;

/// We keep this as a reasonable "soft" limit for the initial Vec allocation,
/// but the actual recycling limit is now handled by the config.
const INITIAL_VEC_CAPACITY: usize = 128;

thread_local! {
    static BODY_BUFFER_CACHE: RefCell<Vec<WaterBuffer>> = RefCell::new(Vec::with_capacity(INITIAL_VEC_CAPACITY));
    static READ_BUFFER_CACHE: RefCell<Vec<WaterBuffer>> = RefCell::new(Vec::with_capacity(INITIAL_VEC_CAPACITY));
    static WRITE_BUFFER_CACHE: RefCell<Vec<WaterBuffer>> = RefCell::new(Vec::with_capacity(INITIAL_VEC_CAPACITY));
    // static ALC:RefCell<usize> = RefCell::new(0);
}

#[derive(Clone, Copy,Debug)] // Added Copy for easier handling
pub enum PooledBufferType {
    Read,
    Write,
    Body,
}

impl PooledBufferType {
    /// Helper to get the correct thread-local storage for a type
    fn with_cache<F, R>(self, f: F) -> R
        where F: FnOnce(&RefCell<Vec<WaterBuffer>>) -> R
    {
        match self {
            PooledBufferType::Read => READ_BUFFER_CACHE.with(f),
            PooledBufferType::Write => WRITE_BUFFER_CACHE.with(f),
            PooledBufferType::Body => BODY_BUFFER_CACHE.with(f),
        }
    }
}

pub struct PooledWaterBuffer {
    inner: Option<WaterBuffer>,
    ty: PooledBufferType,
}

impl PooledWaterBuffer {
    /// Acquires a buffer using global config defaults
    pub fn new(ty: PooledBufferType) -> Self {
        let conf  = crate::server::get_server_config();
        let s = match ty {
            PooledBufferType::Read => {conf.default_read_buffer_size}
            PooledBufferType::Write => {conf.default_write_buffer_size}
            PooledBufferType::Body => {conf.default_body_buffer_size}
        };
        Self::with_capacity(s, ty)
    }

    /// Creating new buffer with specific capacity
    pub fn with_capacity(cap: usize, ty: PooledBufferType) -> Self {
        let buf = ty.with_cache(|cache| {
            let mut cache = cache.borrow_mut();
            if let Some(mut existing_buf) = cache.pop() {
                existing_buf.reset();
                existing_buf
            } else {
               // if let PooledBufferType::Read = ty {
               //      ALC.with(|c| {
               //          let mut alc = c.borrow_mut();
               //          *alc = *alc + 1;
               //          println!("allocating new {:?} buffer  {:?}", ty, *alc);
               //      });
               //  }
                WaterBuffer::with_capacity(cap)
            }
        });

        Self { inner: Some(buf), ty }
    }

    pub fn take_inner(&mut self) -> WaterBuffer {
        self.inner.take().expect("Buffer already taken or dropped")
    }

    pub fn recycle(buf: WaterBuffer, ty: PooledBufferType) {
        let conf = crate::server::get_server_config();

        // 1. Check if the buffer is too large to keep in memory (Prevent bloat)
        // 2. Check if the cache is already full (Prevent leak)
        if buf.cap() <= conf.max_buffer_size_for_cache {
            ty.with_cache(|cache| {
                let mut cache = cache.borrow_mut();
                // We use the config value here instead of a CONST
                if cache.len() < conf.max_cached_buffers_count {
                    cache.push(buf);
                }
            });
        }

    }
}

impl Drop for PooledWaterBuffer {
    fn drop(&mut self) {
        if let Some(buf) = self.inner.take() {
            Self::recycle(buf, self.ty);
        }
    }
}

// --- Standard Traits ---

impl Deref for PooledWaterBuffer {
    type Target = WaterBuffer;
    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().expect("Buffer used after take")
    }
}

impl DerefMut for PooledWaterBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().expect("Buffer used after take")
    }
}