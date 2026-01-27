use std::io;
use std::io::{Read, Seek, SeekFrom};
use crate::binary::{BufBinaryReader, FromBinary};
// pub enum EXIFTags {
//
// }

macro_rules! impl_byte_order {
    (
        $( $name:ident = $value:expr ),* $(,)?
    ) => {
        #[repr(u16)]
        pub enum ByteOrder {
            Unknown(u16),
            $( $name = u16::from_be_bytes(*$value) ),*
        }

        impl From<u16> for ByteOrder {
            fn from(value: u16) -> Self {
                match value {
                    $( x if x == u16::from_be_bytes(*$value) => ByteOrder::$name,)*
                    other => ByteOrder::Unknown(other),
                }
            }
        }
    };
}

impl_byte_order! {
    Intel = b"II",
    Motorola = b"MM",
}

#[repr(u16)]
enum EXIFTag {
    Unknown(u16),
}

enum EXIFDataType {
    UnsignedByte(u8),
    ASCIIString(Vec<u8>),
    UnsignedShort(u16),
    UnsignedLong(u32),
    UnsignedRational(u32, u32),
    SignedByte(i8),
    Unknown(Vec<u8>),
    SignedShort(i16),
    SignedLong(i32),
    SignedRational(i32, u32),
    SingleFloat(f32),
    DoubleFloat(f64),
}

struct ImageFileDirectoryEntry {
    tag: EXIFTag,
    // datatype 2byte
    // count: u32,
    data: EXIFDataType,
}

struct ImageFileDirectory {
    // 4 byte num of entries
    // Remember to disregard the thumbnail image.
    entries: Vec<ImageFileDirectoryEntry>,
    // 4 byte next entry offset, 0 means no more
}
pub struct ExifData {
    size: u16,
    byte_order: ByteOrder,
    // Remember to ensure byte order matches the 0x002a value
    // ifd_offset: u32,
    image_file_directories: Vec<ImageFileDirectory>,
}

impl FromBinary for Vec<ImageFileDirectory> {
    fn read_from<R: Read + Seek>(reader: &mut BufBinaryReader<R>) -> io::Result<Self> {
        let entry_count: u16 = reader.read()?;

        let entries: Vec<ImageFileDirectory> = Vec::new();

        Ok(entries)
    }
}

impl FromBinary for ExifData {
    fn read_from<R: Read + Seek>(reader: &mut BufBinaryReader<R>) -> io::Result<Self> {
        // Assume we're reading from the start of a JPEG Image
        let mut buf = reader.read_bytes(2)?;

        // Make sure we're starting from a Tag at least.
        if buf[0] != 0xFF {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid JPEG filetype",
            ));
        }

        let mut size: u16 = 0;
        let mut done: bool = false;

        // Seek to correct APP1 Tag
        while !done {
            match (buf[1]) {
                0xC0..=0xCF | 0xDB..=0xDF | 0xE0 | 0xE2..=0xEF => {
                    // SOF1-SOF15, DQT, DNL, DRI, DHP, EXP, APP0, APP2-APP15
                    let len: u16 = reader.read()?;
                    reader.seek(SeekFrom::Current((len) as i64))?;
                    buf = reader.read_bytes(2)?;
                }
                0xD0..=0xD8 | 0xFE => {
                    // RST, DQT, DNL, DRI, DHP, EXP
                    buf = reader.read_bytes(2)?;
                }
                0xE1 => {
                    // APP1
                    let len: u16 = reader.read()?;

                    if len >= 6 {
                        let mut magic = reader.read_bytes(6)?;

                        if &magic == b"Exif\0\0" {
                            // Found the data!
                            size = len;
                            done = true;
                        } else {
                            reader.seek(SeekFrom::Current((len - 6)  as i64))?;
                            buf = reader.read_bytes(2)?;
                        }
                    } else {
                        reader.seek(SeekFrom::Current((len)  as i64))?;
                        buf = reader.read_bytes(2)?;
                    }
                }
                0xD9 | 0xDA => {
                    // SOS EOI
                    dbg!("sos eoi");
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "invalid JPEG filetype, missing APP1 section",
                    ));
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "invalid JPEG filetype, unknown section",
                    ));
                }
            }
        }

        // @TODO: Make better bounds on reading past limit

        // Reader is now just after Exif magic
        let byte_order = ByteOrder::from(reader.read::<u16>()?);

        // 2 byte check goes here

        let image_file_directories = Vec::<ImageFileDirectory>::read_from(reader)?;

        Ok(Self {
            size,
            byte_order,
            image_file_directories,
        })
    }
}
