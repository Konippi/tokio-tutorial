use bytes::{Buf, BytesMut};
use mini_redis::Frame;
use std::io::Cursor;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
    net::TcpStream,
};

pub struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

impl Connection {
    /// Create a new connection.
    pub fn new(socket: TcpStream) -> Self {
        Self {
            stream: BufWriter::new(socket),
            buffer: BytesMut::with_capacity(4 * 1024), // Capacity: 4KB
        }
    }

    /// Read a frame from the connection.
    pub async fn read_frame(&mut self) -> mini_redis::Result<Option<Frame>> {
        loop {
            // Attempt to parse a frame from the buffered data.
            // If enough data has been buffered, the frame is returned.
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }

            // There is not enough buffered data to read a frame
            // `0` is end of stream.
            if 0 == self.stream.read_buf(&mut self.buffer).await? {
                // The connection is closed.
                if self.buffer.is_empty() {
                    return Ok(None);
                }
                // The connection is reset by peer.
                else {
                    return Err("connection reset by peer".into());
                }
            }
        }
    }

    /// Parse a frame from the buffered data.
    /// 1. Make sure the entire frame is in the buffer.
    /// 2. Parse the frame from the buffer.
    fn parse_frame(&mut self) -> mini_redis::Result<Option<Frame>> {
        let mut cursor = Cursor::new(&self.buffer[..]);

        match Frame::check(&mut cursor) {
            // The frame is complete.
            Ok(_) => {
                // Get the current position of the cursor (the end of frame).
                let position = cursor.position() as usize;
                // Reset the buffer to read the frame.
                cursor.set_position(0);
                // Parse the frame from the buffer.
                let frame = Frame::parse(&mut cursor)?;
                // Advance the buffer to remove the frame.
                self.buffer.advance(position);
                Ok(Some(frame))
            }
            // The frame is incomplete.
            Err(mini_redis::frame::Error::Incomplete) => Ok(None),
            // The frame is invalid.
            Err(e) => Err(e.into()),
        }
    }

    /// Write a frame to the connection.
    pub async fn write_frame(&mut self, frame: &Frame) -> tokio::io::Result<()> {
        match frame {
            // Simple string
            Frame::Simple(value) => {
                self.stream.write_u8(b'+').await?;
                self.stream.write_all(value.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            // Error
            Frame::Error(value) => {
                self.stream.write_u8(b'-').await?;
                self.stream.write_all(value.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            // Integer
            Frame::Integer(value) => {
                self.stream.write_u8(b':').await?;
                self.write_decimal(*value).await?;
            }
            // Null
            Frame::Null => {
                self.stream.write_all(b"$-1\r\n").await?;
            }
            // Bulk string
            Frame::Bulk(value) => {
                self.stream.write_u8(b'$').await?;
                self.write_decimal(value.len() as u64).await?;
                self.stream.write_all(value).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            // Array
            Frame::Array(_value) => unreachable!(),
        }

        // Flush the buffered data to the connection.
        self.stream.flush().await?;

        Ok(())
    }

    /// Write a decimal to the connection.
    async fn write_decimal(&mut self, value: u64) -> tokio::io::Result<()> {
        use std::io::Write;

        let mut buf = [0u8, 12];
        let mut cursor = Cursor::new(&mut buf[..]);
        write!(&mut cursor, "{}", value)?;

        let position = cursor.position() as usize;
        self.stream.write_all(&cursor.get_ref()[..position]).await?;
        self.stream.write_all(b"\r\n").await?;

        Ok(())
    }
}
