use bytes::{BufMut, BytesMut};

#[allow(dead_code)]
static CRLF: &[u8; 2] = b"\r\n";

///
pub struct Buffer {
    inner: BytesMut, // inner.len() is the write_index, Don't use any "split" method of inner
    read_index: usize,
    reserved_prepend_index: usize, // send write data with this offet for prepend header later
}

impl Buffer {
    ///
    pub fn new(init_size: usize, reserved_prepend_size: usize) -> Self {
        let mut b = Self {
            inner: BytesMut::with_capacity(init_size),
            read_index: reserved_prepend_size,
            reserved_prepend_index: reserved_prepend_size,
        };

        // write prepending zeros( as placeholder )
        b.inner.put_bytes(0, reserved_prepend_size);
        b
    }

    ///
    #[inline(always)]
    pub fn data_mut(&mut self) -> *mut u8 {
        unsafe { self.begin_ptr().offset(self.read_index as isize) }
    }

    ///
    #[inline(always)]
    pub fn write_index(&self) -> usize {
        self.inner.len()
    }

    ///
    #[inline(always)]
    pub fn set_write_index(&mut self, len: usize) {
        unsafe {
            self.inner.set_len(len);
        }
    }

    /// Length returns the number of bytes of the unread portion of the buffer
    #[inline(always)]
    pub fn length(&self) -> usize {
        assert!(self.inner.len() >= self.read_index);
        self.inner.len() - self.read_index
    }

    /// It is the same as length()
    #[inline(always)]
    pub fn size(&self) -> usize {
        self.length()
    }

    /// It is the same as 0 == length()
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        0_usize == self.length()
    }

    /// Capacity returns the capacity of the buffer's underlying byte slice, that is, the
    /// total space allocated for the buffer's data
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    ///
    #[inline(always)]
    pub fn writable_bytes(&self) -> usize {
        assert!(self.capacity() >= self.write_index());
        self.capacity() - self.write_index()
    }

    ///
    #[inline(always)]
    pub fn prependable_bytes(&self) -> usize {
        self.read_index
    }

    /// Ensure buffer capacity
    #[inline(always)]
    pub fn ensure(&mut self, len: usize) {
        if self.capacity() < self.reserved_prepend_index + len {
            self.grow(self.reserved_prepend_index + len - self.capacity());
        }
    }

    /// Ensure writable bytes
    #[inline(always)]
    pub fn ensure_writable_bytes(&mut self, len: usize) {
        let writable_bytes = self.writable_bytes();
        if writable_bytes < len {
            self.grow(len - writable_bytes);
        }
    }

    /// Extend more space for write
    #[inline(always)]
    pub fn extend(&mut self, len: usize) -> &mut [u8] {
        self.ensure_writable_bytes(len);
        let slice = unsafe { std::slice::from_raw_parts_mut(self.write_ptr(), len) };
        self.set_write_index(self.write_index() + len);
        slice
    }

    /// Truncate discards all but the first n unread bytes from the buffer, and
    /// continues to use the same allocated storage.
    /// It does nothing if n is greater than the length of the buffer.
    #[inline(always)]
    pub fn truncate_to(&mut self, remain: usize) {
        if 0 == remain {
            // "write_index == read_index" means no data in buffer now, so reset it
            self.read_index = self.reserved_prepend_index;
            self.set_write_index(self.reserved_prepend_index);
        } else if self.write_index() > self.read_index + remain {
            // retains n bytes in the buffer for user read
            self.set_write_index(self.read_index + remain);
        } else {
            // truncate nothing
        }
    }

    /// Discard returns a slice containing the tail n bytes from the buffer, rollback the
    /// buffer as if the bytes had never been write.
    /// If there are fewer than n bytes in the buffer, Next() returns then entire buff.
    /// The slice is only valid until the next call to read or write method.
    pub fn discard(&mut self, len: usize) -> &mut [u8] {
        if len < self.length() {
            let write_index = self.write_index() - len;
            let write_ptr = unsafe { self.begin_ptr().offset(write_index as isize) };
            let slice = unsafe { std::slice::from_raw_parts_mut(write_ptr, len) };
            self.set_write_index(write_index);
            slice
        } else {
            self.next_all()
        }
    }

    /// Reset resets the buffer to be empty, but it retains the underlying storage
    /// for use by future writes.
    /// It is the same as truncate_to(0)
    #[inline(always)]
    pub fn reset(&mut self) {
        self.truncate_to(0);
    }

    /// Skip advance the reading index of the buffer
    #[inline(always)]
    pub fn skip(&mut self, len: usize) {
        if len < self.length() {
            self.read_index += len;
        } else {
            self.reset();
        }
    }

    /*================================ write ================================*/

    /// Write with raw ptr and len
    #[inline(always)]
    pub fn write(&mut self, d: *const u8, len: usize) {
        let slice = unsafe { std::slice::from_raw_parts(d, len) };
        self.write_slice(slice);
    }

    /// Write with raw ptr and len
    #[inline(always)]
    pub fn write_slice(&mut self, slice: &[u8]) {
        self.inner.put_slice(slice);
    }

    /// Append u128/u64/732/u16 with network endian (big endian)
    #[inline(always)]
    pub fn append_u128(&mut self, n: u128) {
        let be = n.to_be();
        self.write(&be as *const u128 as *const u8, std::mem::size_of::<u128>());
    }

    #[inline(always)]
    pub fn append_u64(&mut self, n: u64) {
        let be = n.to_be();
        self.write(&be as *const u64 as *const u8, std::mem::size_of::<u64>());
    }

    #[inline(always)]
    pub fn append_u32(&mut self, n: u32) {
        let be = n.to_be();
        self.write(&be as *const u32 as *const u8, std::mem::size_of::<u32>());
    }

    #[inline(always)]
    pub fn append_u16(&mut self, n: u16) {
        let be = n.to_be();
        self.write(&be as *const u16 as *const u8, std::mem::size_of::<u16>());
    }

    #[inline(always)]
    pub fn append_u8(&mut self, n: u8) {
        let write_index = self.write_index();
        (self.inner)[write_index] = n;
        self.set_write_index(write_index + 1);
    }

    /*================================ prepend ================================*/

    /// Prepend u128
    #[inline(always)]
    pub fn prepend_u128(&mut self, n: u128) {
        let be = n.to_be();
        self.prepend(&be as *const u128 as *const u8, std::mem::size_of::<u128>());
    }

    /// Prepend u64
    #[inline(always)]
    pub fn prepend_u64(&mut self, n: u64) {
        let be = n.to_be();
        self.prepend(&be as *const u64 as *const u8, std::mem::size_of::<u64>());
    }

    /// Prepend u32
    #[inline(always)]
    pub fn prepend_u32(&mut self, n: u32) {
        let be = n.to_be();
        self.prepend(&be as *const u32 as *const u8, std::mem::size_of::<u32>());
    }

    /// Prepend u16
    #[inline(always)]
    pub fn prepend_u16(&mut self, n: u16) {
        let be = n.to_be();
        self.prepend(&be as *const u16 as *const u8, std::mem::size_of::<u16>());
    }

    /// Prepend u8
    #[inline(always)]
    pub fn prepend_u8(&mut self, n: u8) {
        self.read_index -= 1;
        (self.inner)[self.read_index] = n;
    }

    /// Prepend insert content specified by the parameter, into the front of reading index
    #[inline(always)]
    pub fn prepend(&mut self, d: *const u8, len: usize) {
        assert!(len <= self.prependable_bytes());
        self.read_index -= len;

        let src = d;
        let dst = self.data_mut();
        unsafe {
            std::ptr::copy_nonoverlapping(src, dst, len);
        }
    }

    /*================================ next ================================*/

    /// Next returns a slice containing the next n bytes from the buffer, advancing the
    /// buffer as if the bytes had been retured by Read.
    /// If there are fewer than n bytes in the buffer, Next() returns then entire buff.
    /// The slice is only valid until the next call to read or write method.
    pub fn next(&mut self, len: usize) -> &mut [u8] {
        if len < self.length() {
            let slice = unsafe { std::slice::from_raw_parts_mut(self.data_mut(), len) };
            self.read_index += len;
            slice
        } else {
            self.next_all()
        }
    }

    /// NextAll returns a slice containing all the unread portion of the buffer, advancing
    /// the buffer as if the bytes had been returned by read.
    pub fn next_all(&mut self) -> &mut [u8] {
        let slice = unsafe { std::slice::from_raw_parts_mut(self.data_mut(), self.length()) };
        self.reset();
        slice
    }

    /*================================ peek and read ================================*/

    /// Read u128
    #[inline(always)]
    pub fn read_u128(&mut self) -> u128 {
        let n = self.peek_u128();
        self.read_index += std::mem::size_of::<u128>();
        n
    }

    /// Read u64
    #[inline(always)]
    pub fn read_u64(&mut self) -> u64 {
        let n = self.peek_u64();
        self.read_index += std::mem::size_of::<u64>();
        n
    }

    /// Read u32
    #[inline(always)]
    pub fn read_u32(&mut self) -> u32 {
        let n = self.peek_u32();
        self.read_index += std::mem::size_of::<u32>();
        n
    }

    /// Read u16
    #[inline(always)]
    pub fn read_u16(&mut self) -> u16 {
        let n = self.peek_u16();
        self.read_index += std::mem::size_of::<u16>();
        n
    }

    /// Read u8
    #[inline(always)]
    pub fn read_u8(&mut self) -> u8 {
        let n = self.peek_u8();
        self.read_index += 1;
        n
    }

    /// Peek u128
    #[inline(always)]
    pub fn peek_u128(&self) -> u128 {
        let len = std::mem::size_of::<u128>();
        let n = 0_u128;
        assert!(self.length() >= len);
        unsafe {
            let src = std::slice::from_raw_parts(
                self.begin_ptr_const().offset(self.read_index as isize),
                len,
            );
            let dst = std::slice::from_raw_parts_mut(&n as *const u128 as *mut u8, len);

            //le do nothing, we should use swap_bytes()
            dst[0] = src[15];
            dst[1] = src[14];
            dst[2] = src[13];
            dst[3] = src[12];
            dst[4] = src[11];
            dst[5] = src[10];
            dst[6] = src[9];
            dst[7] = src[8];

            dst[8] = src[7];
            dst[9] = src[6];
            dst[10] = src[5];
            dst[11] = src[4];
            dst[12] = src[3];
            dst[13] = src[2];
            dst[14] = src[1];
            dst[15] = src[0];
        }
        n
    }

    /// Peek u64
    #[inline(always)]
    pub fn peek_u64(&self) -> u64 {
        let len = std::mem::size_of::<u64>();
        let n = 0_u64;
        assert!(self.length() >= len);
        unsafe {
            let src = std::slice::from_raw_parts(
                self.begin_ptr_const().offset(self.read_index as isize),
                len,
            );
            let dst = std::slice::from_raw_parts_mut(&n as *const u64 as *mut u8, len);

            //le do nothing, we should use swap_bytes()
            dst[0] = src[7];
            dst[1] = src[6];
            dst[2] = src[5];
            dst[3] = src[4];
            dst[4] = src[3];
            dst[5] = src[2];
            dst[6] = src[1];
            dst[7] = src[0];
        }
        n
    }

    /// Peek u32
    #[inline(always)]
    pub fn peek_u32(&self) -> u32 {
        let len = std::mem::size_of::<u32>();
        let n = 0_u32;
        assert!(self.length() >= len);
        unsafe {
            let src = std::slice::from_raw_parts(
                self.begin_ptr_const().offset(self.read_index as isize),
                len,
            );
            let dst = std::slice::from_raw_parts_mut(&n as *const u32 as *mut u8, len);

            //le do nothing, we should use swap_bytes()
            dst[0] = src[3];
            dst[1] = src[2];
            dst[2] = src[1];
            dst[3] = src[0];
        }
        n
    }

    /// Peek u16
    #[inline(always)]
    pub fn peek_u16(&self) -> u16 {
        let len = std::mem::size_of::<u16>();
        let n = 0_u16;
        assert!(self.length() >= len);
        unsafe {
            let src = std::slice::from_raw_parts(
                self.begin_ptr_const().offset(self.read_index as isize),
                len,
            );
            let dst = std::slice::from_raw_parts_mut(&n as *const u16 as *mut u8, len);

            //le do nothing, we should use swap_bytes()
            dst[0] = src[1];
            dst[1] = src[0];
        }
        n
    }

    /// Peek u8
    #[inline(always)]
    pub fn peek_u8(&self) -> u8 {
        assert!(self.length() >= 1);
        (self.inner)[self.read_index]
    }

    /// Peek all readable data
    #[inline(always)]
    pub fn peek(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.read_ptr(), self.length()) }
    }

    /*================================ private ================================*/

    #[inline(always)]
    fn begin_ptr_const(&self) -> *const u8 {
        (*self.inner).as_ptr()
    }

    #[inline(always)]
    fn begin_ptr(&mut self) -> *mut u8 {
        self.inner.as_mut().as_mut_ptr()
    }

    #[inline(always)]
    fn reserve_prepend_ptr(&mut self) -> *mut u8 {
        unsafe { self.begin_ptr().offset(self.reserved_prepend_index as isize) }
    }

    #[inline(always)]
    fn read_ptr(&self) -> *const u8 {
        unsafe { self.begin_ptr_const().offset(self.read_index as isize) }
    }

    #[inline(always)]
    fn write_ptr(&mut self) -> *mut u8 {
        unsafe { self.begin_ptr().offset(self.write_index() as isize) }
    }

    #[inline(always)]
    fn grow(&mut self, additional: usize) {
        // if we can make space inside buffer
        assert!(self.reserved_prepend_index <= self.read_index);
        let old_writable_bytes = self.writable_bytes();
        let already_read_bytes = self.read_index - self.reserved_prepend_index;
        if already_read_bytes < additional {
            // grow the capacity
            self.inner.reserve(std::cmp::max(self.capacity() << 1, additional));
        } else {
            // already_read_bytes space can be reused:
            // move readable data to the front, make space inside buffer
            let readable = self.length();

            let src = self.read_ptr();
            let dst = self.reserve_prepend_ptr();
            unsafe {
                std::ptr::copy(src, dst, readable);
            }
            self.read_index = self.reserved_prepend_index;
            self.set_write_index(self.read_index + readable);

            assert_eq!(self.length(), readable);
            assert!(self.writable_bytes() >= old_writable_bytes + additional);
        }
    }
}

impl std::fmt::Write for Buffer {
    #[inline(always)]
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        let slice = s.as_bytes();
        self.write_slice(slice);
        Ok(())
    }

    #[inline(always)]
    fn write_char(&mut self, c: char) -> std::fmt::Result {
        let slice =
            unsafe { std::slice::from_raw_parts_mut(self.write_ptr(), self.writable_bytes()) };
        c.encode_utf8(slice);
        Ok(())
    }
}
