use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::{env, fmt, io};

macro_rules! impl_raf_meta_tag {
    (
        $( $name:ident = $value:expr ),* $(,)?
    ) => {
        #[repr(u16)]
        enum RafMetaTag {
            Unknown(u16),
            $( $name = $value ),*
        }

        impl From<u16> for RafMetaTag {
            fn from(value: u16) -> Self {
                match value {
                    $( $value => RafMetaTag::$name,)*
                    other => RafMetaTag::Unknown(other),
                }
            }
        }

        impl Display for RafMetaTag {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                match self {
                    $(RafMetaTag::$name => write!(f, stringify!($name)),)*
                    RafMetaTag::Unknown(value) => write!(f, "Unknown(0x{:04X})", value),
                }
            }
        }
    };
}

impl_raf_meta_tag! {
    SensorDimensions = 0x0100,
    ActiveAreaTopLeft = 0x0110, // RawImageCropTopLeft
    ActiveAreaHeightWidth = 0x0111, // RawImageCroppedSize
    UnknownDimensions1 = 0x0112,
    UnknownDimensions1Inverted = 0x0113,
    ActiveAreaAspectRatio = 0x0115, // RawImageAspectRatio
    UnknownDimensions2 = 0x0119,
    OutputHeightWidth = 0x0121, // RawImageSize
    RawInfo = 0x0130, // FujiLayout
    CFAPattern = 0x0131, // XTransLayout
    WhiteBalancePreset = 0x1002, // Possibly wrong
    FlashExposureComp = 0x1011,
    MacroMode = 0x1020,
    FocusMode = 0x1021,
    AFMode = 0x1022,
    FocusPixel = 0x1023,
    PrioritySettings = 0x102b,
    FocusSettings = 0x102d,
    AFCSettings = 0x102e,
    ExrMode = 0x1034,
    FujiCropMode = 0x104d,
    ShutterType = 0x1050,
    AutoBracketing = 0x1100,
    SequenceNumber = 0x1101,
    DriveMode = 0x1103,
    SeriesLength = 0x1105,
    PixelShiftOffset = 0x1106,
    FocusWarning = 0x1301,
    DynamicRange = 0x1400,
    FilmMode = 0x1401,
    DynamicRangeSetting = 0x1402,
    DevDynamicRange = 0x1403,
    MinFocalLength = 0x1404,
    MaxFocalLength = 0x1405,
    MaxApertureMinFocal = 0x1406,
    MaxApertureMaxFocal = 0x1407,
    AutoDynamicRange = 0x140b,
    ImageStabilization = 0x1422,
    Rating = 0x1431,
    ImageCount = 0x1438,
    DRangePriority = 0x1443,
    DRangePriorityAuto = 0x1444,
    DRangePriorityFixed = 0x1445,
    FujiModel = 0x1447,
    FujiModel2 = 0x1448,
    WhiteBalanceRGB = 0x2f00,
    CameraMultiplier = 0x2ff0, // Also listed as White Balance RGB
    RelativeExposure = 0x9200,
    RAWExposureBias = 0x9650,
    UnknownExposureBias = 0x9651,
    OtherData = 0xc000, // Also listed as RAF Data
    UnknownData = 0xca00,
}

enum RafMetadataType {
    Unknown(Vec<u8>),
    Dimensions((u16, u16)),
    Position((u16, u16)),
    Kelvin(u32),
    ExposureValue(f32),
    // GeneralValue(u32),
    AspectRatio((u16, u16)),
    ASCIIText(Vec<u8>),
    ExposureBias(f32), // maybe same as exposure compensation
}

struct RafMetaItem {
    tag: RafMetaTag,
    data: RafMetadataType,
}

struct RafMetaContainer {
    count: u32,
    items: Vec<RafMetaItem>,
}

struct RafDirectory {
    version: u32,
    // unknown1: [u8; 20],
    jpeg_offset: u32,
    jpeg_size: u32,
    meta_container_offset: u32,
    meta_container_size: u32,
    cfa_offset: u32,
    cfa_size: u32,
    // unknown2: [u8; 12],
    // unknown3: u32,
}

struct Raf {
    filetype: [u8; 16],
    // data0: u32,
    // data1: u64,
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

        let size = {
            let mut buf = [0u8; 2];
            reader.read_exact(&mut buf)?;
            u16::from_be_bytes(buf)
        };

        let mut buf = vec![0u8; size as usize];
        reader.read_exact(&mut buf)?;

        let data: RafMetadataType;

        match tag {
            RafMetaTag::SensorDimensions
            | RafMetaTag::OutputHeightWidth
            | RafMetaTag::UnknownDimensions1
            | RafMetaTag::UnknownDimensions1Inverted
            | RafMetaTag::UnknownDimensions2
                if size == 4 =>
            {
                data = RafMetadataType::Dimensions((
                    u16::from_be_bytes([buf[0], buf[1]]),
                    u16::from_be_bytes([buf[2], buf[3]]),
                ));
            }
            RafMetaTag::WhiteBalancePreset if size == 4 => {
                data =
                    RafMetadataType::Kelvin(u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]));
            }
            RafMetaTag::FlashExposureComp if size == 4 => {
                data = RafMetadataType::ExposureValue(f32::from_be_bytes([
                    buf[0], buf[1], buf[2], buf[3],
                ]));
            }
            RafMetaTag::ActiveAreaTopLeft | RafMetaTag::ActiveAreaHeightWidth if size == 4 => {
                data = RafMetadataType::Position((
                    u16::from_be_bytes([buf[0], buf[1]]),
                    u16::from_be_bytes([buf[2], buf[3]]),
                ));
            }
            RafMetaTag::ActiveAreaAspectRatio if size == 4 => {
                data = RafMetadataType::AspectRatio((
                    u16::from_be_bytes([buf[0], buf[1]]),
                    u16::from_be_bytes([buf[2], buf[3]]),
                ));
            }
            RafMetaTag::FujiModel | RafMetaTag::FujiModel2 if size == 4 => {
                data = RafMetadataType::ASCIIText(buf.to_vec());
            }
            RafMetaTag::RAWExposureBias | RafMetaTag::UnknownExposureBias if size == 4 => {
                let a = i16::fromq_be_bytes([buf[0], buf[1]]);
                let mut b = u16::from_be_bytes([buf[2], buf[3]]) as f32;
                if b < 1.0 {
                    b = 1.0;
                }
                data = RafMetadataType::ExposureBias(a as f32 / b);
            }
            // @TODO I'm sure there's something valuable here
            // RafMetaTag::OtherData => {
            //     println!("other pos:{}", reader.stream_position()? - size as u64);
            //     println!("other  sz:{}", size);
            //     data = RafMetadataType::Unknown(buf.to_vec());
            // }
            // RafMetaTag::UnknownData => {
            //     println!("unkwn pos:{}", reader.stream_position()? - size as u64);
            //     println!("unkwn  sz:{}", size);
            //     data = RafMetadataType::Unknown(buf.to_vec());
            // }
            _ => {
                data = RafMetadataType::Unknown(buf.to_vec());
            }
        }

        Ok(Self { tag, data })
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
        let version = {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            u32::from_be_bytes(buf)
        };

        // let unknown1 = {
        //     let mut buf = [0u8; 20];
        //     reader.read_exact(&mut buf)?;
        //     buf
        // };

        reader.seek(SeekFrom::Current(20))?;

        let jpeg_offset = {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            u32::from_be_bytes(buf)
        };

        let jpeg_size = {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            u32::from_be_bytes(buf)
        };

        let meta_container_offset = {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            u32::from_be_bytes(buf)
        };

        let meta_container_size = {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            u32::from_be_bytes(buf)
        };

        let cfa_offset = {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            u32::from_be_bytes(buf)
        };

        let cfa_size = {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            u32::from_be_bytes(buf)
        };

        // let unknown2 = {
        //     let mut buf = [0u8; 12];
        //     reader.read_exact(&mut buf)?;
        //     buf
        // };
        //
        // let unknown3 = {
        //     let mut buf = [0u8; 4];
        //     reader.read_exact(&mut buf)?;
        //     u32::from_be_bytes(buf)
        // };

        Ok(Self {
            version,
            // unknown1,
            jpeg_offset,
            jpeg_size,
            meta_container_offset,
            meta_container_size,
            cfa_offset,
            cfa_size,
            // unknown2,
            // unknown3,
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

        // let data0 = {
        //     let mut buf = [0u8; 4];
        //     reader.read_exact(&mut buf)?;
        //     u32::from_le_bytes(buf)
        // };
        //
        // let data1 = {
        //     let mut buf = [0u8; 8];
        //     reader.read_exact(&mut buf)?;
        //     u64::from_le_bytes(buf)
        // };

        reader.seek(SeekFrom::Current(12))?;

        let camera = {
            let mut buf = [0u8; 32];
            reader.read_exact(&mut buf)?;
            buf
        };

        let dir = RafDirectory::read_from(reader)?;

        reader.seek(SeekFrom::Start(dir.meta_container_offset as u64))?;
        let meta = RafMetaContainer::read_from(reader)?;

        Ok(Self {
            filetype,
            // data0,
            // data1,
            camera,
            dir,
            meta,
        })
    }
}

impl Display for RafMetadataType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RafMetadataType::Unknown(value) => {
                if value.len() == 2 {
                    write!(
                        f,
                        "{:?} u16: {} char: {}",
                        value,
                        u16::from_be_bytes([value[0], value[1]]),
                        String::from_utf8_lossy(value)
                    )
                } else if value.len() == 4 {
                    write!(
                        f,
                        "{:?} u32: {} [u16; 2]: {:?} f32: {:.8} char: {}",
                        value,
                        u32::from_be_bytes([value[0], value[1], value[2], value[3]]),
                        [
                            u16::from_be_bytes([value[0], value[1]]),
                            u16::from_be_bytes([value[2], value[3]])
                        ],
                        f32::from_be_bytes([value[0], value[1], value[2], value[3]]),
                        String::from_utf8_lossy(value)
                    )
                } else if value.len() < 256 {
                    write!(f, "{:?}", value)
                } else {
                    write!(f, "{:?}", &value[0..255])?;
                    write!(f, " trimmed for size...")
                }
            }
            RafMetadataType::Dimensions((x, y)) => write!(f, "{x} x {y}"),
            RafMetadataType::Position((x, y)) => write!(f, "{x} {y}"),
            RafMetadataType::Kelvin(value) => write!(f, "{value}K"),
            RafMetadataType::ExposureValue(value) => write!(f, "{value} EV"),
            // RafMetadataType::GeneralValue(value) => write!(f, "{value}"),
            RafMetadataType::AspectRatio((a, b)) => write!(f, "{a}:{b}"),
            RafMetadataType::ASCIIText(value) => write!(f, "{}", String::from_utf8_lossy(value)),
            RafMetadataType::ExposureBias(value) => write!(f, "{value}"),
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
        writeln!(
            f,
            "\tVersion: \"{}\" {} 0x{:X}",
            str::from_utf8(&u32::to_be_bytes(self.version)).unwrap(),
            &self.version,
            &self.version
        )?;
        writeln!(
            f,
            "\tJPEG Offset: {} 0x{:X}",
            &self.jpeg_offset, &self.jpeg_offset
        )?;
        writeln!(
            f,
            "\tJPEG Size: {} 0x{:X}",
            &self.jpeg_size, &self.jpeg_size
        )?;
        writeln!(
            f,
            "\tMetadata Container Offset: {} 0x{:X}",
            &self.meta_container_offset, &self.meta_container_offset
        )?;
        writeln!(
            f,
            "\tMetadata Container Size: {} 0x{:X}",
            &self.meta_container_size, &self.meta_container_size
        )?;
        writeln!(
            f,
            "\tCFA Offset: {} 0x{:X}",
            &self.cfa_offset, &self.cfa_offset
        )?;
        writeln!(f, "\tCFA Size: {} 0x{:X}", &self.cfa_size, &self.cfa_size)
    }
}

impl Display for Raf {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "Magic: {}", str::from_utf8(&self.filetype).unwrap())?;
        writeln!(f, "Camera: {}", str::from_utf8(&self.camera).unwrap())?;
        write!(f, "Directory: \n{}", &self.dir)?;
        write!(f, "Metadata: \n{}", &self.meta)
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut source = File::open(args.get(2).expect("File not found")).expect("Unable to open file");

    let parsed = Raf::read_from(&mut source).unwrap();

    println!("{}", parsed);
}
