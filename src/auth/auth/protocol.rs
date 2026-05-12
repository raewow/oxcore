use bytes::{Buf, BufMut, BytesMut};
use std::io;

/// Helper trait for reading from BytesMut
pub trait ReadPacket: Sized {
    fn read_from(buf: &mut BytesMut) -> io::Result<Self>;
}

/// Helper trait for writing to BytesMut
pub trait WritePacket {
    fn write_to(&self, buf: &mut BytesMut);
}

/// Read a null-terminated string from buffer
pub fn read_cstring(buf: &mut BytesMut) -> io::Result<String> {
    let mut bytes = Vec::new();

    while !buf.is_empty() {
        let byte = buf.get_u8();
        if byte == 0 {
            return String::from_utf8(bytes).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8 in string")
            });
        }
        bytes.push(byte);
    }

    Err(io::Error::new(
        io::ErrorKind::UnexpectedEof,
        "String not null-terminated",
    ))
}

/// Write a null-terminated string to buffer
pub fn write_cstring(buf: &mut BytesMut, s: &str) {
    buf.put_slice(s.as_bytes());
    buf.put_u8(0);
}

/// Read a u8 from buffer
impl ReadPacket for u8 {
    fn read_from(buf: &mut BytesMut) -> io::Result<Self> {
        if buf.len() < 1 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Not enough data for u8",
            ));
        }
        Ok(buf.get_u8())
    }
}

/// Read a u16 (little-endian) from buffer
impl ReadPacket for u16 {
    fn read_from(buf: &mut BytesMut) -> io::Result<Self> {
        if buf.len() < 2 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Not enough data for u16",
            ));
        }
        Ok(buf.get_u16_le())
    }
}

/// Read a u32 (little-endian) from buffer
impl ReadPacket for u32 {
    fn read_from(buf: &mut BytesMut) -> io::Result<Self> {
        if buf.len() < 4 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Not enough data for u32",
            ));
        }
        Ok(buf.get_u32_le())
    }
}

/// Write a u8 to buffer
impl WritePacket for u8 {
    fn write_to(&self, buf: &mut BytesMut) {
        buf.put_u8(*self);
    }
}

/// Write a u16 (little-endian) to buffer
impl WritePacket for u16 {
    fn write_to(&self, buf: &mut BytesMut) {
        buf.put_u16_le(*self);
    }
}

/// Write a u32 (little-endian) to buffer
impl WritePacket for u32 {
    fn write_to(&self, buf: &mut BytesMut) {
        buf.put_u32_le(*self);
    }
}
