use std::io::{self, BufRead, BufReader, Read};

/// [`BufReader`] with the ability to peek two bytes. This is
/// necessary until <https://github.com/rust-lang/rust/issues/128405> is merged.
#[derive(Debug)]
pub struct PeekReader<R: Read> {
    buf_reader: BufReader<R>,
    /// A secondary buffer in case `peek2` is called,
    /// but `buf_reader.buffer().len() == 1`. It is not (yet)
    /// possible to fill the buffer before it is empty, so in this
    /// case, we must put the one byte here, empty the buffer, and then
    /// fill it again.
    buffer2: Option<u8>,
}

impl<R: Read> PeekReader<R> {
    /// Creates a new `PeekReader<R>` with a default buffer capacity. The default is currently 8 KiB,
    /// but may change in the future.
    pub fn new(inner: R) -> Self {
        Self {
            buf_reader: BufReader::new(inner),
            buffer2: None,
        }
    }

    /// Creates a new `PeekReader<R>` with the specified buffer capacity.
    #[cfg(test)]
    pub fn with_capacity(capacity: usize, inner: R) -> Self {
        Self {
            buf_reader: BufReader::with_capacity(capacity, inner),
            buffer2: None,
        }
    }

    /// Read one value without discarding it.  Returns None if EOF is reached.
    pub fn peek(&mut self) -> io::Result<Option<u8>> {
        if let Some(byte) = self.buffer2 {
            Ok(Some(byte))
        } else if let Some(byte) = self.buf_reader.fill_buf()?.first() {
            Ok(Some(*byte))
        } else {
            Ok(None)
        }
    }

    /// Read two values without discarding them.  Returns None if EOF is reached.
    pub fn peek2(&mut self) -> io::Result<Option<[u8; 2]>> {
        let current_buf = self.buf_reader.fill_buf()?;
        let Some(current_buf_first) = current_buf.first() else {
            return Ok(None);
        };

        if let Some(byte1) = self.buffer2 {
            Ok(Some([byte1, *current_buf_first]))
        } else if let Some(current_buf_second) = current_buf.get(1) {
            Ok(Some([*current_buf_first, *current_buf_second]))
        } else {
            // buf_reader buffer is only one byte long! Put current first into
            // buffer 2, consume buffer, and try to fetch new values
            let current_buf_first = self.buffer2.insert(*current_buf_first);
            self.buf_reader.consume(1);

            if let Some(new_buf_first) = self.buf_reader.fill_buf()?.first() {
                Ok(Some([*current_buf_first, *new_buf_first]))
            } else {
                Ok(None)
            }
        }
    }

    /// Read a single byte. Returns None if EOF is reached.
    pub fn read_byte(&mut self) -> io::Result<Option<u8>> {
        let mut buff = [0];
        match self.read_exact(&mut buff) {
            Ok(()) => Ok(Some(buff[0])),
            Err(err) => {
                if err.kind() == io::ErrorKind::UnexpectedEof {
                    Ok(None)
                } else {
                    Err(err)
                }
            }
        }
    }
}

impl<R: Read> Read for PeekReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        if let Some(byte) = self.buffer2.take() {
            buf[0] = byte;
            let read = 1 + self.buf_reader.read(&mut buf[1..])?;
            Ok(read)
        } else {
            self.buf_reader.read(buf)
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        if buf.is_empty() {
            return Ok(());
        }

        if let Some(byte) = self.buffer2.take() {
            buf[0] = byte;
            self.buf_reader.read_exact(&mut buf[1..])
        } else {
            self.buf_reader.read_exact(buf)
        }
    }
}

impl<R: Read> BufRead for PeekReader<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.buf_reader.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        if amt == 0 {
            return;
        }
        if self.buffer2.take().is_some() {
            self.buf_reader.consume(amt - 1)
        } else {
            self.buf_reader.consume(amt)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peek_reader() {
        let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut reader = PeekReader::new(data.as_slice());
        assert_eq!(reader.peek().unwrap(), Some(0));
        assert_eq!(reader.peek2().unwrap(), Some([0, 1]));

        assert_eq!(reader.read_byte().unwrap(), Some(0));
        assert_eq!(reader.peek2().unwrap(), Some([1, 2]));

        let mut buf = [0, 0, 0];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [1, 2, 3]);
        assert_eq!(reader.peek().unwrap(), Some(4));
        assert_eq!(reader.peek2().unwrap(), Some([4, 5]));

        let mut buf = [0, 0, 0, 0, 0];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [4, 5, 6, 7, 8]);
        assert_eq!(reader.peek().unwrap(), Some(9));
        assert_eq!(reader.peek2().unwrap(), None);

        assert_eq!(reader.read_byte().unwrap(), Some(9));
        assert_eq!(reader.peek().unwrap(), None);
        assert_eq!(reader.read_byte().unwrap(), None);
    }

    #[test]
    fn test_small_buf() {
        let data = vec![9, 8, 7, 6, 5, 4, 3, 2, 1, 0];
        let mut reader = PeekReader::with_capacity(3, data.as_slice());
        assert_eq!(reader.peek().unwrap(), Some(9));
        assert_eq!(reader.peek2().unwrap(), Some([9, 8]));
        assert_eq!(reader.buf_reader.buffer().len(), 3);

        let mut buf = [0, 0];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [9, 8]);
        assert_eq!(reader.buf_reader.buffer().len(), 1);

        assert_eq!(reader.peek().unwrap(), Some(7));
        assert!(reader.buffer2.is_none());
        assert_eq!(reader.peek2().unwrap(), Some([7, 6]));
        assert_eq!(reader.buffer2, Some(7));
        assert_eq!(reader.buf_reader.buffer().len(), 3);

        assert_eq!(reader.peek().unwrap(), Some(7));
        assert_eq!(reader.peek2().unwrap(), Some([7, 6]));
        assert_eq!(reader.buffer2, Some(7));

        assert_eq!(reader.read_byte().unwrap(), Some(7));
        assert!(reader.buffer2.is_none());
        assert_eq!(reader.peek2().unwrap(), Some([6, 5]));
        assert!(reader.buffer2.is_none());

        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [6, 5]);
    }
}
