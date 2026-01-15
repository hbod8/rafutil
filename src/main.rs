use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::{env, fmt, io, mem};

#[repr(u16)]
enum RafMetaTag {
    Unknown(u16),
    SensorDimensions = 0x0100,
    ActiveAreaTopLeft = 0x0110,
    ActiveAreaTopRight = 0x0111,
    ActiveAreaAspectRatio = 0x0115,
    OutputHeightWidth = 0x0121,
    RawInfo = 0x0130,
    CFAPattern = 0x0131,
    WhiteBalancePreset = 0x1002,
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
    OtherData = 0xc000, // Also listed as RAF Data
}

enum RafMetadataType {
    Unknown(Vec<u8>),
    PixelDimensions((u16, u16)),
    Pixels(u32),
    Kelvin(u32),
    ExposureValue(f32),
    GeneralValue(u32),
    GeneralValuePair((u16, u16)),
    ASCIIText(Vec<u8>),
}

impl From<u16> for RafMetaTag {
    fn from(value: u16) -> Self {
        match value {
            0x0100 => RafMetaTag::SensorDimensions,
            0x0110 => RafMetaTag::ActiveAreaTopLeft,
            0x0111 => RafMetaTag::ActiveAreaTopRight,
            0x0115 => RafMetaTag::ActiveAreaAspectRatio,
            0x0121 => RafMetaTag::OutputHeightWidth,
            0x0130 => RafMetaTag::RawInfo,
            0x0131 => RafMetaTag::CFAPattern,
            0x1002 => RafMetaTag::WhiteBalancePreset,
            0x1011 => RafMetaTag::FlashExposureComp,
            0x1020 => RafMetaTag::MacroMode,
            0x1021 => RafMetaTag::FocusMode,
            0x1022 => RafMetaTag::AFMode,
            0x1023 => RafMetaTag::FocusPixel,
            0x102b => RafMetaTag::PrioritySettings,
            0x102d => RafMetaTag::FocusSettings,
            0x102e => RafMetaTag::AFCSettings,
            0x1034 => RafMetaTag::ExrMode,
            0x104d => RafMetaTag::FujiCropMode,
            0x1050 => RafMetaTag::ShutterType,
            0x1100 => RafMetaTag::AutoBracketing,
            0x1101 => RafMetaTag::SequenceNumber,
            0x1103 => RafMetaTag::DriveMode,
            0x1105 => RafMetaTag::SeriesLength,
            0x1106 => RafMetaTag::PixelShiftOffset,
            0x1301 => RafMetaTag::FocusWarning,
            0x1400 => RafMetaTag::DynamicRange,
            0x1401 => RafMetaTag::FilmMode,
            0x1402 => RafMetaTag::DynamicRangeSetting,
            0x1403 => RafMetaTag::DevDynamicRange,
            0x1404 => RafMetaTag::MinFocalLength,
            0x1405 => RafMetaTag::MaxFocalLength,
            0x1406 => RafMetaTag::MaxApertureMinFocal,
            0x1407 => RafMetaTag::MaxApertureMaxFocal,
            0x140b => RafMetaTag::AutoDynamicRange,
            0x1422 => RafMetaTag::ImageStabilization,
            0x1438 => RafMetaTag::ImageCount,
            0x1431 => RafMetaTag::Rating,
            0x1443 => RafMetaTag::DRangePriority,
            0x1444 => RafMetaTag::DRangePriorityAuto,
            0x1445 => RafMetaTag::DRangePriorityFixed,
            0x1447 => RafMetaTag::FujiModel,
            0x1448 => RafMetaTag::FujiModel2,
            0x2f00 => RafMetaTag::WhiteBalanceRGB,
            0x2ff0 => RafMetaTag::CameraMultiplier,
            0x9200 => RafMetaTag::RelativeExposure,
            0x9650 => RafMetaTag::RAWExposureBias,
            0xc000 => RafMetaTag::OtherData,
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

        let size = {
            let mut buf = [0u8; 2];
            reader.read_exact(&mut buf)?;
            u16::from_be_bytes(buf)
        };

        let mut buf = vec![0u8; size as usize];
        reader.read_exact(&mut buf)?;

        let mut data: RafMetadataType;

        match tag {
            RafMetaTag::SensorDimensions if size == 4 => {
                data = RafMetadataType::PixelDimensions((
                    u16::from_be_bytes([buf[0], buf[1]]),
                    u16::from_be_bytes([buf[2], buf[3]]),
                ));
            }
            RafMetaTag::ActiveAreaTopLeft | RafMetaTag::ActiveAreaTopRight if size == 4 => {
                // Maybe little endian here, need to find the meaning of these values...
                data =
                    RafMetadataType::Pixels(u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]));
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
            RafMetaTag::ActiveAreaAspectRatio if size == 4 => {
                data = RafMetadataType::GeneralValuePair((
                    u16::from_be_bytes([buf[0], buf[1]]),
                    u16::from_be_bytes([buf[2], buf[3]]),
                ));
            }
            RafMetaTag::MacroMode
            | RafMetaTag::FocusMode
            | RafMetaTag::AFMode
            | RafMetaTag::FocusPixel
            | RafMetaTag::PrioritySettings
            | RafMetaTag::FocusSettings
            | RafMetaTag::AFCSettings
            | RafMetaTag::ExrMode
            | RafMetaTag::FujiCropMode
            | RafMetaTag::ShutterType
            | RafMetaTag::AutoBracketing
            | RafMetaTag::SequenceNumber
            | RafMetaTag::DriveMode
            | RafMetaTag::SeriesLength
            | RafMetaTag::PixelShiftOffset
            | RafMetaTag::FocusWarning
            | RafMetaTag::DynamicRange
            | RafMetaTag::FilmMode
            | RafMetaTag::DynamicRangeSetting
            | RafMetaTag::DevDynamicRange
            | RafMetaTag::MinFocalLength
            | RafMetaTag::MaxFocalLength
            | RafMetaTag::MaxApertureMinFocal
            | RafMetaTag::MaxApertureMaxFocal
            | RafMetaTag::AutoDynamicRange
            | RafMetaTag::ImageStabilization
            | RafMetaTag::ImageCount
            | RafMetaTag::Rating
            | RafMetaTag::DRangePriority
            | RafMetaTag::DRangePriorityAuto
            | RafMetaTag::DRangePriorityFixed
                if size == 4 =>
            {
                data = RafMetadataType::GeneralValue(u32::from_be_bytes([
                    buf[0], buf[1], buf[2], buf[3],
                ]));
            }
            RafMetaTag::FujiModel | RafMetaTag::FujiModel2 if size == 4 => {
                data = RafMetadataType::ASCIIText(buf.to_vec());
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

        Ok(Self {
            filetype,
            data0,
            data1,
            camera,
            dir,
            meta,
        })
    }
}

impl Display for RafMetaTag {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RafMetaTag::SensorDimensions => write!(f, "SensorDimensions"),
            RafMetaTag::ActiveAreaTopLeft => write!(f, "ActiveAreaTopLeft"),
            RafMetaTag::ActiveAreaTopRight => write!(f, "ActiveAreaTopRight"),
            RafMetaTag::ActiveAreaAspectRatio => write!(f, "ActiveAreaAspectRatio"),
            RafMetaTag::OutputHeightWidth => write!(f, "OutputHeightWidth"),
            RafMetaTag::CFAPattern => write!(f, "CFAPattern"),
            RafMetaTag::WhiteBalancePreset => write!(f, "WhiteBalancePreset"),
            RafMetaTag::FlashExposureComp => write!(f, "FlashExposureComp"),
            RafMetaTag::MacroMode => write!(f, "MacroMode"),
            RafMetaTag::FocusMode => write!(f, "FocusMode"),
            RafMetaTag::AFMode => write!(f, "AFMode"),
            RafMetaTag::FocusPixel => write!(f, "FocusPixel"),
            RafMetaTag::PrioritySettings => write!(f, "PrioritySettings"),
            RafMetaTag::FocusSettings => write!(f, "FocusSettings"),
            RafMetaTag::AFCSettings => write!(f, "AFCSettings"),
            RafMetaTag::ExrMode => write!(f, "ExrMode"),
            RafMetaTag::FujiCropMode => write!(f, "FujiCropMode"),
            RafMetaTag::ShutterType => write!(f, "ShutterType"),
            RafMetaTag::AutoBracketing => write!(f, "AutoBracketing"),
            RafMetaTag::SequenceNumber => write!(f, "SequenceNumber"),
            RafMetaTag::DriveMode => write!(f, "DriveMode"),
            RafMetaTag::SeriesLength => write!(f, "SeriesLength"),
            RafMetaTag::PixelShiftOffset => write!(f, "PixelShiftOffset"),
            RafMetaTag::FocusWarning => write!(f, "FocusWarning"),
            RafMetaTag::DynamicRange => write!(f, "DynamicRange"),
            RafMetaTag::FilmMode => write!(f, "FilmMode"),
            RafMetaTag::DynamicRangeSetting => write!(f, "DynamicRangeSetting"),
            RafMetaTag::DevDynamicRange => write!(f, "DevDynamicRange"),
            RafMetaTag::MinFocalLength => write!(f, "MinFocalLength"),
            RafMetaTag::MaxFocalLength => write!(f, "MaxFocalLength"),
            RafMetaTag::MaxApertureMinFocal => write!(f, "MaxApertureMinFocal"),
            RafMetaTag::MaxApertureMaxFocal => write!(f, "MaxApertureMaxFocal"),
            RafMetaTag::AutoDynamicRange => write!(f, "AutoDynamicRange"),
            RafMetaTag::ImageStabilization => write!(f, "ImageStabilization"),
            RafMetaTag::ImageCount => write!(f, "ImageCount"),
            RafMetaTag::Rating => write!(f, "Rating"),
            RafMetaTag::DRangePriority => write!(f, "DRangePriority"),
            RafMetaTag::DRangePriorityAuto => write!(f, "DRangePriorityAuto"),
            RafMetaTag::DRangePriorityFixed => write!(f, "DRangePriorityFixed"),
            RafMetaTag::WhiteBalanceRGB => write!(f, "WhiteBalanceRGB"),
            RafMetaTag::CameraMultiplier => write!(f, "CameraMultiplier"),
            RafMetaTag::RelativeExposure => write!(f, "RelativeExposure"),
            RafMetaTag::RAWExposureBias => write!(f, "RAWExposureBias"),
            RafMetaTag::FujiModel => write!(f, "FujiModel"),
            RafMetaTag::FujiModel2 => write!(f, "FujiModel2"),
            RafMetaTag::OtherData => write!(f, "OtherData"),
            RafMetaTag::RawInfo => write!(f, "RawInfo"),
            RafMetaTag::Unknown(value) => write!(f, "Unknown(0x{:04X})", value),
        }
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
            RafMetadataType::PixelDimensions((x, y)) => write!(f, "{x} x {y}"),
            RafMetadataType::Pixels(pixels) => write!(f, "{pixels}px"),
            RafMetadataType::Kelvin(value) => write!(f, "{value}K"),
            RafMetadataType::ExposureValue(value) => write!(f, "{value} EV"),
            RafMetadataType::GeneralValue(value) => write!(f, "{value}"),
            RafMetadataType::GeneralValuePair((a, b)) => write!(f, "{a}:{b}"),
            RafMetadataType::ASCIIText(value) => write!(f, "{}", String::from_utf8_lossy(value)),
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
