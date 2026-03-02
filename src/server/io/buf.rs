use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use water_buffer::WaterBuffer as BM;

type WaterBuffer = BM<u8>;

/// Configuration constants for the pool
const MAX_CACHED_BUFFERS: usize = 512 * 3;    // Max buffers stored per thread
const DEFAULT_CAPACITY: usize = 16384 * 4;   // 16KB initial size
const MAX_RECYCLABLE_SIZE: usize = 65536 * 2;

thread_local! {
    /// The actual storage for recycled buffers
    static BUFFER_CACHE: RefCell<Vec<WaterBuffer>> = RefCell::new(Vec::with_capacity(MAX_CACHED_BUFFERS));
    static ALC:RefCell<usize> = RefCell::new(0);
}

/// A "Smart Pointer" that wraps WaterBuffer. 
/// When this struct is dropped, the inner buffer is returned to the pool.
pub struct PooledWaterBuffer {
    inner: Option<WaterBuffer>,
}

impl PooledWaterBuffer {
    /// Acquires a buffer from the thread-local pool or allocates a new one if empty.
    pub fn new() -> Self {
        let buf = BUFFER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            if let Some(mut existing_buf) = cache.pop() {
                existing_buf.clear(); // Clear pointers/length but keep heap allocation
                existing_buf
            } else {
              ALC.with(|b|{
                   let mut a =  b.borrow_mut();
                  *a = *a +1;
                  println!("allocating buffer count {:?}",*a);
              });

                WaterBuffer::with_capacity(DEFAULT_CAPACITY)
            }
        });

        Self { inner: Some(buf) }
    }

    /// Explicitly take the inner WaterBuffer. 
    /// Useful for tokio-uring which requires moving ownership into the Kernel.
    pub fn take_inner(&mut self) -> WaterBuffer {
        self.inner.take().expect("Buffer already taken or dropped")
    }

    /// Manually put a buffer back into the pool (e.g., after a uring operation completes).
    pub fn recycle(mut buf: WaterBuffer) {
        // Safety check: Don't cache massive buffers to prevent memory bloat
        if buf.cap() <= MAX_RECYCLABLE_SIZE {
            BUFFER_CACHE.with(|cache| {
                let mut cache = cache.borrow_mut();
                if cache.len() < MAX_CACHED_BUFFERS {
                    cache.push(buf);
                }
            });
        }
        // If not added to cache, 'buf' drops naturally and calls dealloc
    }
}

/// Auto-recycling logic when the PooledWaterBuffer handle goes out of scope
impl Drop for PooledWaterBuffer {
    fn drop(&mut self) {
        if let Some(buf) = self.inner.take() {
            Self::recycle(buf);
        }
    }
}

// --- Traits to make PooledWaterBuffer act like a normal WaterBuffer ---

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