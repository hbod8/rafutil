use std::{env, fmt, io, mem};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

#[repr(u16)]
enum RafMetaTag {
    Unknown(u16),
    SensorDimensions = 0x0100,
    ActiveAreaTopLeft = 0x0110,
    ActiveAreaTopRight = 0x0111,
    OutputHeightWidth = 0x0121,
    RawInfo = 0x0130,
    CFAPattern = 0x0131,
    CameraMultiplier = 0x2ff0,
    OtherData = 0x0c00,
}

enum RafMetadataType {
    Unknown(Vec<u8>),
    PixelDimensions((u16, u16)),
    Pixels(u32),
}

impl From<u16> for RafMetaTag {
    fn from(value: u16) -> Self {
        match value {
            0x0100 => RafMetaTag::SensorDimensions,
            0x0110 => RafMetaTag::ActiveAreaTopLeft,
            0x0111 => RafMetaTag::ActiveAreaTopRight,
            0x0121 => RafMetaTag::OutputHeightWidth,
            0x0130 => RafMetaTag::RawInfo,
            0x0131 => RafMetaTag::CFAPattern,
            0x2ff0 => RafMetaTag::CameraMultiplier,
            0x0c00 => RafMetaTag::OtherData,
            other => RafMetaTag::Unknown(other),
        }
    }
}

struct RafMetaItem {
    tag: RafMetaTag,
    size: u16,
    data: RafMetadataType,
}

struct RafMetaContainer {
    count: u32,
    items: Vec<RafMetaItem>,
}

struct RafDirectory {
    version: u32,
    unknown1: [u8; 20],
    jpeg_offset: u32,
    jpeg_size: u32,
    meta_container_offset: u32,
    meta_container_size: u32,
    cfa_offset: u32,
    cfa_size: u32,
    unknown2: [u8; 12],
    unknown3: u32,
}

struct Raf {
    filetype: [u8; 16],
    data0: u32,
    data1: u64,
    camera: [u8; 32],
    dir: RafDirectory,
    meta: RafMetaContainer,
}

trait FromBinary: Sized {
    fn read_from<R: Read + Seek>(reader: &mut R) -> io::Result<Self>;
}

impl FromBinary for RafMetaItem {
    fn read_from<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        let tag = {
            let mut buf = [0u8; 2];
            reader.read_exact(&mut buf)?;
            RafMetaTag::from(u16::from_be_bytes(buf))
        };

        let size= {
            let mut buf = [0u8; 2];
            reader.read_exact(&mut buf)?;
            u16::from_be_bytes(buf)
        };

        let mut buf = vec![0u8; size as usize];
        reader.read_exact(&mut buf)?;

        let mut data: RafMetadataType;

        match tag {
            RafMetaTag::SensorDimensions
            if size == 4 => {
                data = RafMetadataType::PixelDimensions((u16::from_be_bytes([buf[0], buf[1]]), u16::from_be_bytes([buf[2], buf[3]])));
            }
            RafMetaTag::ActiveAreaTopLeft |
            RafMetaTag::ActiveAreaTopRight
            if size == 4 => {
                // Maybe little endian here, need to find the meaning of these values...
                data = RafMetadataType::Pixels(u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]));
            }
            _ => {
                data = RafMetadataType::Unknown(buf);
            }
        }

        Ok(Self { tag, size, data })
    }
}

impl FromBinary for RafMetaContainer {
    fn read_from<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        let count = {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            u32::from_be_bytes(buf)
        };

        // Put a sanity check here

        let mut items = Vec::with_capacity(count as usize);
        for _ in 0..count {
            items.push(RafMetaItem::read_from(reader)?);
        }

        Ok(Self { count, items })
    }
}

impl FromBinary for RafDirectory {
    fn read_from<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        Ok(Self {
            version: {
                let mut buf = [0u8; 4];
                reader.read_exact(&mut buf)?;
                u32::from_be_bytes(buf)
            },

            unknown1: {
                let mut buf = [0u8; 20];
                reader.read_exact(&mut buf)?;
                buf
            },

            jpeg_offset: {
                let mut buf = [0u8; 4];
                reader.read_exact(&mut buf)?;
                u32::from_be_bytes(buf)
            },

            jpeg_size: {
                let mut buf = [0u8; 4];
                reader.read_exact(&mut buf)?;
                u32::from_be_bytes(buf)
            },

            meta_container_offset: {
                let mut buf = [0u8; 4];
                reader.read_exact(&mut buf)?;
                u32::from_be_bytes(buf)
            },

            meta_container_size: {
                let mut buf = [0u8; 4];
                reader.read_exact(&mut buf)?;
                u32::from_be_bytes(buf)
            },

            cfa_offset: {
                let mut buf = [0u8; 4];
                reader.read_exact(&mut buf)?;
                u32::from_be_bytes(buf)
            },

            cfa_size: {
                let mut buf = [0u8; 4];
                reader.read_exact(&mut buf)?;
                u32::from_be_bytes(buf)
            },

            unknown2: {
                let mut buf = [0u8; 12];
                reader.read_exact(&mut buf)?;
                buf
            },

            unknown3: {
                let mut buf = [0u8; 4];
                reader.read_exact(&mut buf)?;
                u32::from_be_bytes(buf)
            },
        })
    }
}

impl FromBinary for Raf {
    fn read_from<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        let filetype = {
            let mut buf = [0u8; 16];
            reader.read_exact(&mut buf)?;
            if &buf != b"FUJIFILMCCD-RAW " {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid RAF filetype",
                ));
            }
            buf
        };

        let data0 = {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            u32::from_le_bytes(buf)
        };

        let data1 = {
            let mut buf = [0u8; 8];
            reader.read_exact(&mut buf)?;
            u64::from_le_bytes(buf)
        };

        let camera = {
            let mut buf = [0u8; 32];
            reader.read_exact(&mut buf)?;
            buf
        };

        let dir = RafDirectory::read_from(reader)?;

        reader.seek(SeekFrom::Start(dir.meta_container_offset as u64))?;
        let meta = RafMetaContainer::read_from(reader)?;

        Ok(Self { filetype, data0, data1, camera, dir, meta })
    }
}

impl Display for RafMetaTag {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RafMetaTag::SensorDimensions   => write!(f, "SensorDimensions"),
            RafMetaTag::ActiveAreaTopLeft  => write!(f, "ActiveAreaTopLeft"),
            RafMetaTag::ActiveAreaTopRight => write!(f, "ActiveAreaTopRight"),
            RafMetaTag::OutputHeightWidth  => write!(f, "OutputHeightWidth"),
            RafMetaTag::RawInfo            => write!(f, "RawInfo"),
            RafMetaTag::CFAPattern         => write!(f, "CFAPattern"),
            RafMetaTag::CameraMultiplier   => write!(f, "CameraMultiplier"),
            RafMetaTag::OtherData          => write!(f, "OtherData"),
            RafMetaTag::Unknown(value)     => write!(f, "Unknown(0x{:04X})", value),
        }
    }
}

impl Display for RafMetadataType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RafMetadataType::Unknown(value) => {
                if value.len() < 256 {
                    write!(f, "{:?}", value)
                } else {
                    write!(f, "{:?}", &value[0..255])?;
                    write!(f, " trimmed for size...")
                }
            },
            RafMetadataType::PixelDimensions((x, y)) => write!(f, "{x} x {y}"),
            RafMetadataType::Pixels(pixels) => write!(f, "{pixels}px"),
        }
    }
}

impl Display for RafMetaItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "\t{}: {}", self.tag, self.data)
    }
}

impl Display for RafMetaContainer {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for i in 0..(self.count as usize) {
            write!(f, "{}", &self.items[i])?;
        }
        Ok(())
    }
}

impl Display for RafDirectory {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "\tVersion: \"{}\" {} 0x{:X}", str::from_utf8(&u32::to_be_bytes(self.version)).unwrap(), &self.version, &self.version)?;
        writeln!(f, "\tJPEG Offset: {} 0x{:X}", &self.jpeg_offset, &self.jpeg_offset)?;
        writeln!(f, "\tJPEG Size: {} 0x{:X}", &self.jpeg_size, &self.jpeg_size)?;
        writeln!(f, "\tMetadata Container Offset: {} 0x{:X}", &self.meta_container_offset, &self.meta_container_offset)?;
        writeln!(f, "\tMetadata Container Size: {} 0x{:X}", &self.meta_container_size, &self.meta_container_size)?;
        writeln!(f, "\tCFA Offset: {} 0x{:X}", &self.cfa_offset, &self.cfa_offset)?;
        writeln!(f, "\tCFA Size: {} 0x{:X}", &self.cfa_size, &self.cfa_size)
    }
}

impl Display for Raf {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "Magic: {}", str::from_utf8(&self.filetype).unwrap())?;
        writeln!(f, "Camera: {}", str::from_utf8(&self.camera).unwrap())?;
        write!(f, "Directory: \n{}", &self.dir);
        write!(f, "Meta: \n{}", &self.meta)
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut buf: [u8; size_of::<Raf>()] = [0; size_of::<Raf>()];

    let mut source = File::open(args.get(2).expect("File not found")).expect("Unable to open file");

    let parsed = Raf::read_from(&mut source).unwrap();

    println!("{}", parsed);
}
