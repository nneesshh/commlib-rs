use std::io::{self, Cursor, Write};

use bytes::Buf;

const CRLF: &[u8; 2] = b"\r\n";

///
pub struct Buffer {
    storage: Cursor<Vec<u8>>, // storage.position() is the read_pos( cusor is for read only )
    write_pos: u64,
    header_reserve_size: usize, // send write data with this offset for prepend header later
}

impl Buffer {
    ///
    pub fn new(init_size: usize, header_reserve_size: usize) -> Self {
        // at least hold more than 1 CRLF
        assert!(init_size > header_reserve_size as usize + CRLF.len());

        let mut b = Self {
            storage: Cursor::new(vec![0_u8; init_size]),
            write_pos: header_reserve_size as u64,
            header_reserve_size: header_reserve_size,
        };

        // pad prepending zeros with cursor( as placeholder )
        for _ in 0..header_reserve_size {
            b.storage.write(&[0]).unwrap();
        }
        b
    }

    /// Get cursor pos
    #[inline(always)]
    pub fn read_pos(&self) -> u64 {
        self.storage.position()
    }

    /// Set cursor pos
    #[inline(always)]
    pub fn set_read_pos(&mut self, pos: u64) {
        self.storage.set_position(pos);
    }

    ///
    #[inline(always)]
    pub fn write_pos(&self) -> u64 {
        self.write_pos
    }

    ///
    #[inline(always)]
    pub fn set_write_pos(&mut self, pos: u64) {
        self.write_pos = pos;
    }

    /// It means readable bytes in buffer
    #[inline(always)]
    pub fn size(&self) -> usize {
        self.readable_bytes()
    }

    /// It means bytes between body_pos and write_pos (for write only)
    #[inline(always)]
    pub fn wrote_body_len(&self) -> usize {
        let w_pos = self.write_pos();
        let b_pos = self.header_reserve_size as u64;
        assert!(w_pos >= b_pos);
        (w_pos - b_pos) as usize
    }

    /// It means writable bytes in buffer
    #[inline(always)]
    pub fn free_space(&self) -> usize {
        self.writable_bytes()
    }

    ///
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        0_usize == self.size()
    }

    /// Capacity returns the capacity of the buffer's underlying byte slice, that is, the
    /// total space allocated for the buffer's data
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.storage.get_ref().len()
    }

    /// Reset resets the buffer to be empty, but it retains the underlying storage
    /// for use by future writes.
    /// It is the same as truncate_to(0)
    #[inline(always)]
    pub fn reset(&mut self) {
        let b_pos = self.header_reserve_size as u64;
        self.set_read_pos(b_pos);
        self.set_write_pos(b_pos);
    }

    /*================================ write ================================*/

    /// Ensure buffer capacity
    #[inline(always)]
    pub fn ensure(&mut self, cnt: usize) {
        let p_size = self.header_reserve_size;
        let capacity = self.capacity();
        if capacity < p_size + cnt {
            self.grow(p_size + cnt - capacity);
        }
    }

    /// Ensure free space for write
    #[inline(always)]
    pub fn ensure_free_space(&mut self, cnt: usize) {
        let b_pos = self.header_reserve_size as u64;
        assert!(b_pos <= self.write_pos);
        let used_bytes = self.write_pos - b_pos;
        self.ensure(used_bytes as usize + cnt);
    }

    /// Extend more space for write
    #[inline(always)]
    pub fn extend(&mut self, cnt: usize) -> &mut [u8] {
        self.ensure_free_space(cnt);

        let (ptr, w_pos) = self.write_raw_parts();
        let slice_mut = unsafe { std::slice::from_raw_parts_mut(ptr, cnt) };
        self.set_write_pos(w_pos + cnt as u64);
        slice_mut
    }

    /// Truncate discards all but the first n unread bytes from the buffer, and
    /// continues to use the same allocated storage.
    /// It does nothing if n is greater than the length of the buffer.
    #[inline(always)]
    pub fn truncate_to(&mut self, remain: usize) {
        if 0 == remain {
            // "write_pos == read_pos" means no data in buffer now, so reset it
            self.reset();
        } else if self.size() > remain {
            // retains n bytes in the buffer for user read
            let w_pos = self.read_pos() + remain as u64;
            self.set_write_pos(w_pos);
        } else {
            // truncate nothing
        }
    }

    /// Discard returns a slice containing the tail n bytes from the buffer, rollback the
    /// buffer as if the bytes had never been write.
    /// If there are fewer than n bytes in the buffer, Next() returns then entire buff.
    /// The slice is only valid until the next call to read or write method.
    pub fn discard(&mut self, len: usize) -> &mut [u8] {
        if len < self.size() {
            let (ptr, w_pos) = self.write_raw_parts();
            let w_pos = w_pos - len as u64;
            let slice = unsafe { std::slice::from_raw_parts_mut(ptr, len) };
            self.set_write_pos(w_pos);
            slice
        } else {
            self.advance_all()
        }
    }

    /// Write with raw ptr and len
    #[inline(always)]
    pub fn write(&mut self, d: *const u8, len: usize) {
        self.ensure_free_space(len);

        let src = unsafe { std::slice::from_raw_parts(d, len) };
        let w_pos = self.write_pos();
        let w_tail = w_pos + len as u64;
        let dst_mut = &mut self.storage.get_mut()[(w_pos as usize)..(w_tail as usize)];
        dst_mut.copy_from_slice(src);
        self.set_write_pos(w_tail);
    }

    /// Write with raw ptr and len
    #[inline(always)]
    pub fn write_slice(&mut self, slice: &[u8]) {
        self.write(slice.as_ptr(), slice.len());
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
        let w_pos = self.write_pos();
        self.storage.get_mut()[w_pos as usize] = n;
        self.set_write_pos(w_pos + 1);
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
        let r_pos = self.backward(1);
        self.storage.get_mut()[r_pos as usize] = n;
    }

    /// Prepend insert content specified by the parameter, into the front of read_pos
    #[inline(always)]
    pub fn prepend(&mut self, d: *const u8, len: usize) {
        self.backward(len);

        let src = unsafe { std::slice::from_raw_parts(d, len) };
        let r_pos = self.read_pos();
        let r_tail = r_pos + len as u64;
        let dst_mut = &mut self.storage.get_mut()[(r_pos as usize)..(r_tail as usize)];
        dst_mut.copy_from_slice(src);
    }

    /*================================ peek and read ================================*/

    /// Advance the read_pos(cursor) of the buffer
    #[inline(always)]
    pub fn advance(&mut self, cnt: usize) -> &mut [u8] {
        let (ptr, r_pos) = self.read_raw_parts();
        let readable = self.size();
        if cnt < readable {
            let slice_mut = unsafe { std::slice::from_raw_parts_mut(ptr, cnt) };
            self.set_read_pos(r_pos + cnt as u64);
            slice_mut
        } else {
            let slice_mut = unsafe { std::slice::from_raw_parts_mut(ptr, readable) };
            self.reset();
            slice_mut
        }
    }

    /// Advance the read_pos(cursor) of the buffer
    #[inline(always)]
    pub fn advance_all(&mut self) -> &mut [u8] {
        let (ptr, _r_pos) = self.read_raw_parts();
        let readable = self.size();
        let slice_mut = unsafe { std::slice::from_raw_parts_mut(ptr, readable) };
        self.reset();
        slice_mut
    }

    /// Backward the read_pos(cursor) of the buffer
    //#[inline(always)]
    pub fn backward(&mut self, cnt: usize) -> u64 {
        let r_pos = self.read_pos();
        assert!(cnt as u64 <= r_pos);
        let new_pos = r_pos - cnt as u64;
        self.set_read_pos(new_pos);
        new_pos
    }

    /// Read u128
    #[inline(always)]
    pub fn read_u128(&mut self) -> u128 {
        let n = self.peek_u128();
        self.advance(std::mem::size_of::<u128>());
        n
    }

    /// Read u64
    #[inline(always)]
    pub fn read_u64(&mut self) -> u64 {
        let n = self.peek_u64();
        self.advance(std::mem::size_of::<u64>());
        n
    }

    /// Read u32
    #[inline(always)]
    pub fn read_u32(&mut self) -> u32 {
        let n = self.peek_u32();
        self.advance(std::mem::size_of::<u32>());
        n
    }

    /// Read u16
    #[inline(always)]
    pub fn read_u16(&mut self) -> u16 {
        let n = self.peek_u16();
        self.advance(std::mem::size_of::<u16>());
        n
    }

    /// Read u8
    #[inline(always)]
    pub fn read_u8(&mut self) -> u8 {
        let n = self.peek_u8();
        self.advance(1);
        n
    }

    /// Peek u128
    #[inline(always)]
    pub fn peek_u128(&self) -> u128 {
        let len = std::mem::size_of::<u128>();
        assert!(self.size() >= len);

        let src = self.peek();

        let n = 0_u128;
        unsafe {
            //le do nothing, we should use swap_bytes()
            let dst = std::slice::from_raw_parts_mut(&n as *const u128 as *mut u8, len);

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
        assert!(self.size() >= len);

        let src = self.peek();

        let n = 0_u64;
        unsafe {
            //le do nothing, we should use swap_bytes()
            let dst = std::slice::from_raw_parts_mut(&n as *const u64 as *mut u8, len);

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
        assert!(self.size() >= len);

        let src = self.peek();

        let n = 0_u32;
        unsafe {
            //le do nothing, we should use swap_bytes()
            let dst = std::slice::from_raw_parts_mut(&n as *const u32 as *mut u8, len);

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
        assert!(self.size() >= len);

        let src = self.peek();

        let n = 0_u16;
        unsafe {
            //le do nothing, we should use swap_bytes()
            let dst = std::slice::from_raw_parts_mut(&n as *const u16 as *mut u8, len);

            dst[0] = src[1];
            dst[1] = src[0];
        }
        n
    }

    /// Peek u8
    #[inline(always)]
    pub fn peek_u8(&self) -> u8 {
        assert!(self.size() >= 1);
        let src = self.peek();
        src[0]
    }

    /// Peek all readable data
    #[inline(always)]
    pub fn peek(&self) -> &[u8] {
        let offset = self.read_pos() as usize;
        let readable = self.size();
        &self.storage.get_ref()[offset..offset + readable]
    }

    ///
    #[inline(always)]
    pub fn chunk(&self) -> &[u8] {
        self.peek()
    }

    ///
    #[inline(always)]
    pub fn remaining(&self) -> usize {
        Buf::remaining(self.as_cursor())
    }

    /*================================ view( as ) ================================*/

    /// For write
    #[inline(always)]
    pub fn as_write_mut(&mut self) -> &mut [u8] {
        let w_pos = self.write_pos();
        let slice_mut = self.storage.get_mut();
        &mut slice_mut[w_pos as usize..]
    }

    /// For read
    #[inline(always)]
    pub fn as_read_mut(&mut self) -> &mut [u8] {
        let r_pos = self.read_pos();
        let w_pos = self.write_pos();
        let slice_mut = self.storage.get_mut();
        &mut slice_mut[r_pos as usize..w_pos as usize]
    }

    /// For read -- get a cursor to the data storage.
    #[inline(always)]
    pub fn as_cursor(&self) -> &Cursor<Vec<u8>> {
        &self.storage
    }

    /// For read -- get a cursor to the mutable data storage.
    #[inline(always)]
    pub fn as_cursor_mut(&mut self) -> &mut Cursor<Vec<u8>> {
        &mut self.storage
    }

    /*================================ stream ================================*/

    /// Read next portion of data from the given input stream.
    #[inline(always)]
    pub fn read_from<S: io::Read>(&mut self, stream: &mut S) -> io::Result<usize> {
        let slice_mut = self.as_write_mut();
        let size = stream.read(slice_mut)?;
        let w_pos = self.write_pos();
        self.set_write_pos(w_pos + size as u64);
        Ok(size)
    }

    /*================================ private ================================*/

    #[inline(always)]
    fn begin_ptr(&mut self) -> *mut u8 {
        self.storage.get_mut().as_mut_ptr()
    }

    #[inline(always)]
    fn body_raw_parts(&mut self) -> (*mut u8, u64) {
        let b_pos = self.header_reserve_size as u64;
        unsafe {
            (
                self.begin_ptr().offset(self.header_reserve_size as isize),
                b_pos,
            )
        }
    }

    #[inline(always)]
    fn read_raw_parts(&mut self) -> (*mut u8, u64) {
        let r_pos = self.read_pos();
        unsafe { (self.begin_ptr().offset(r_pos as isize), r_pos) }
    }

    #[inline(always)]
    fn write_raw_parts(&mut self) -> (*mut u8, u64) {
        let w_pos = self.write_pos();
        unsafe { (self.begin_ptr().offset(w_pos as isize), w_pos) }
    }

    #[inline(always)]
    fn writable_bytes(&self) -> usize {
        let w_pos = self.write_pos();
        let capacity = self.capacity();
        assert!(capacity >= w_pos as usize);
        capacity - w_pos as usize
    }

    #[inline(always)]
    fn readable_bytes(&self) -> usize {
        let r_pos = self.read_pos();
        let w_pos = self.write_pos();
        assert!(w_pos >= r_pos);
        (w_pos - r_pos) as usize
    }

    #[inline(always)]
    fn grow(&mut self, additional: usize) {
        let b_pos = self.header_reserve_size as u64;
        let r_pos = self.read_pos();
        let w_pos = self.write_pos();
        let capacity = self.capacity();

        // if we can make space inside buffer
        assert!(b_pos <= self.read_pos());
        let old_writable_bytes = capacity - w_pos as usize;
        let already_read_bytes = (r_pos - b_pos) as usize;
        if already_read_bytes < additional {
            // grow the capacity with resize
            self.storage.get_mut().resize(capacity + additional, 0_u8);
        } else {
            // already_read_bytes space can be reused:
            // move readable data to the front, make space inside buffer
            let readable = self.size();

            let (src, _r_pos) = self.read_raw_parts();
            let (dst, _b_pos) = self.body_raw_parts();
            unsafe {
                std::ptr::copy(src, dst, readable);
            }
            self.set_read_pos(b_pos);
            self.set_write_pos(b_pos + readable as u64);

            assert_eq!(self.size(), readable);
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
        let slice = self.as_write_mut();
        c.encode_utf8(slice);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_write_number_and_slice() {
        let mut buffer = Buffer::new(4096, 8);
        let n128 = 12345678901234567_u128;
        let n64 = -1234567_i64;
        let n32 = 4012345678_u32;
        let n16 = -32767_i16;
        let n8 = 123_u8;
        let hello = b"Hello World!";

        buffer.append_u128(n128);
        buffer.append_u64(n64 as u64);
        buffer.append_u32(n32);
        buffer.append_u16(n16 as u16);
        buffer.append_u8(n8);

        buffer.write_slice(hello);

        buffer.prepend_u16(n16 as u16);
        buffer.prepend_u8(n8);

        assert_eq!(n8, buffer.read_u8());
        assert_eq!(n16, buffer.read_u16() as i16);

        assert_eq!(n128, buffer.read_u128());
        assert_eq!(n64, buffer.read_u64() as i64);
        assert_eq!(n32, buffer.read_u32());
        assert_eq!(n16, buffer.read_u16() as i16);
        assert_eq!(n8, buffer.read_u8());

        assert_eq!(hello, buffer.advance(hello.len()));
    }

    #[test]
    fn simple_reading() {
        let mut input = Cursor::new(b"Hello World!".to_vec());
        let mut buffer = Buffer::new(4096, 8);
        let size = buffer.read_from(&mut input).unwrap();
        assert_eq!(size, 12);
        assert_eq!(buffer.peek(), b"Hello World!");
    }

    #[test]
    fn reading_in_chunks() {
        let mut inp = Cursor::new(b"Hello World!".to_vec());
        let mut buf = Buffer::new(6, 2);

        let size = buf.read_from(&mut inp).unwrap();
        assert_eq!(size, 4);
        assert_eq!(buf.chunk(), b"Hell");

        buf.advance(2);
        assert_eq!(buf.chunk(), b"ll");
        let w_pos = buf.write_pos();
        assert_eq!(
            &buf.storage.get_mut()[buf.header_reserve_size..(w_pos as usize)],
            b"Hell"
        );

        buf.ensure_free_space(4);
        let size = buf.read_from(&mut inp).unwrap();
        assert_eq!(size, 4);
        assert_eq!(buf.chunk(), b"llo Wo");
        let w_pos = buf.write_pos();
        assert_eq!(
            &buf.storage.get_mut()[buf.header_reserve_size..(w_pos as usize)],
            b"Hello Wo"
        );

        buf.ensure_free_space(4);
        let size = buf.read_from(&mut inp).unwrap();
        assert_eq!(size, 4);
        assert_eq!(buf.chunk(), b"llo World!");
    }
}
