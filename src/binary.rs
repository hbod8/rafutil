use std::io;
use std::io::{BufReader, Read, Seek, SeekFrom};

pub trait FromBinary: Sized {
    fn read_from<R: Read + Seek>(reader: &mut BufBinaryReader<R>) -> io::Result<Self>;
}

impl FromBinary for u16 {
    fn read_from<R: Read + Seek>(reader: &mut BufBinaryReader<R>) -> io::Result<Self> {
        reader.read_u16()
    }
}

impl FromBinary for u32 {
    fn read_from<R: Read + Seek>(reader: &mut BufBinaryReader<R>) -> io::Result<Self> {
        reader.read_u32()
    }
}

impl FromBinary for u64 {
    fn read_from<R: Read + Seek>(reader: &mut BufBinaryReader<R>) -> io::Result<Self> {
        reader.read_u64()
    }
}

pub enum Endian {
    Big,
    Little,
}

pub struct BufBinaryReader<R: Read + Seek> {
    inner: BufReader<R>,
    endianness: Endian,
}

impl<R: Read + Seek> BufBinaryReader<R> {
    pub fn new(reader: R, endianness: Endian) -> BufBinaryReader<R> {
        Self {
            inner: BufReader::new(reader),
            endianness,
        }
    }

    pub fn set_endianness(&mut self, endianness: Endian) {
        self.endianness = endianness;
    }

    pub fn stream_position(&mut self) -> io::Result<u64> {
        self.inner.stream_position()
    }

    pub fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match pos {
            SeekFrom::Start(offset) => {
                let diff= offset as i64 - self.stream_position()? as i64;
                self.inner.seek_relative(diff)?;
                Ok(diff as u64)
            }
            SeekFrom::End(offset) => {
                self.inner.seek(SeekFrom::End(offset))
            }
            SeekFrom::Current(offset) => {
                self.skip_bytes(offset);
                Ok(self.stream_position()? + (offset as u64))
            }
        }
    }

    pub fn skip_bytes(&mut self, bytes: i64) -> io::Result<()> {
        self.inner.seek_relative(bytes)
    }

    pub fn read<T: FromBinary>(&mut self) -> io::Result<T> {
        T::read_from(self)
    }

    pub fn read_bytes(&mut self, size: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0; size];
        self.inner.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.inner.read_exact(buf)
    }

    fn read_u16(&mut self) -> io::Result<u16> {
        let mut buf = [0u8; 2];
        self.inner.read_exact(&mut buf)?;
        match self.endianness {
            Endian::Big => Ok(u16::from_be_bytes(buf)),
            Endian::Little => Ok(u16::from_le_bytes(buf)),
        }
    }

    fn read_u32(&mut self) -> io::Result<u32> {
        let mut buf = [0u8; 4];
        self.inner.read_exact(&mut buf)?;
        match self.endianness {
            Endian::Big => Ok(u32::from_be_bytes(buf)),
            Endian::Little => Ok(u32::from_le_bytes(buf)),
        }
    }

    fn read_u64(&mut self) -> io::Result<u64> {
        let mut buf = [0u8; 8];
        self.inner.read_exact(&mut buf)?;
        match self.endianness {
            Endian::Big => Ok(u64::from_be_bytes(buf)),
            Endian::Little => Ok(u64::from_le_bytes(buf)),
        }
    }
}