use std::cell::RefCell;
use std::io::{Read, Write};
use std::rc::Rc;

#[derive(Clone, Default)]
pub(crate) struct MagicBuffer {
    inner: Rc<RefCell<Vec<u8>>>,
    read_offset: Rc<RefCell<usize>>,
}

impl Write for MagicBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.borrow_mut().extend_from_slice(buf);

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // How does this not have a default implementation?
        Ok(())
    }
}

impl Read for MagicBuffer {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let inner_borrow = self.inner.borrow();
        let read_offset = *self.read_offset.borrow();
        let current_len = inner_borrow.len();

        let read_size = std::cmp::min(buf.len(), current_len.saturating_sub(read_offset));
        buf[..read_size].copy_from_slice(&inner_borrow[read_offset..(read_offset + read_size)]);

        *self.read_offset.borrow_mut() += read_size;

        Ok(read_size)
    }
}
