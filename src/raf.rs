use std::fmt::{Display, Formatter};
use std::{fmt, io};
use std::io::{Read, Seek, SeekFrom};
use crate::binary::{BufBinaryReader, Endian, FromBinary};
use crate::exif::{ExifData};

macro_rules! impl_raf_meta_tag {
    (
        $( $name:ident = $value:expr ),* $(,)?
    ) => {
        #[repr(u16)]
        pub enum RafMetadataTag {
            Unknown(u16),
            $( $name = $value ),*
        }

        impl From<u16> for RafMetadataTag {
            fn from(value: u16) -> Self {
                match value {
                    $( $value => RafMetadataTag::$name,)*
                    other => RafMetadataTag::Unknown(other),
                }
            }
        }

        impl Display for RafMetadataTag {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                match self {
                    $(RafMetadataTag::$name => write!(f, stringify!($name)),)*
                    RafMetadataTag::Unknown(value) => write!(f, "Unknown(0x{:04X})", value),
                }
            }
        }
    };
}

/*
These are a list of educated guesses on the meaning of Fujifilm's metadata tags.

1/16/2026 - Seems like there is some kind of pattern between the first byte and
the second one relating to datatype and tag number. Dimensions usually start with
0x01, u32 values usually start with 0x10.  Also in HDR tags, 0x05 and 0x06 are shutter
and aperture data stored as u32/u32 fractions.
*/
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
    ImageSequenceNumber = 0x2001,
    ImageSequenceRelativeExposure = 0x2003, // might be swapped with one below
    ImageSequenceAbsoluteExposure = 0x2004,
    ImageSequenceShutterDuration = 0x2005,
    ImageSequenceApetureRatio = 0x2006,
    ImageSequenceISO = 0x2007,
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

pub enum RafMetadataType {
    Unknown(Vec<u8>),
    Dimensions((u16, u16)),
    Position((u16, u16)),
    Kelvin(u32),
    ExposureValue(f32),
    Number(u32),
    AspectRatio((u16, u16)),
    ASCIIText(Vec<u8>),
    ExposureBias(f32), // maybe same as exposure compensation
    ShutterDuration((u32, u32)),
    ApertureRatio((u32, u32)),
}

pub struct RafMetadataItem {
    tag: RafMetadataTag,
    data: RafMetadataType,
}

pub struct RafMetadataContainer {
    count: u32,
    metadata: Vec<RafMetadataItem>,
}

pub struct RafImageSequenceImage {
    cfa_offset: u64,
    cfa_size: u64,
    metadata: Vec<RafMetadataItem>,
}

pub struct RafImageSequenceContainer {
    additional_image_count: u32,
    total_image_count: u32,
    version: String,
    image_sequence_metadata_size: u16,
    image_sequence_metadata_count: u16,
    image_sequence_images: Vec<RafImageSequenceImage>,
}

pub struct Raf {
    camera: String,
    version: u32,
    // padding[20]
    // jpeg_offset: u32,
    // jpeg_size: u32,
    // meta_container_offset: u32,
    // meta_container_size: u32,
    // cfa_offset: u32,
    // cfa_size: u32,
    // padding[16]
    // All other contents are variably sized
    exif_data: ExifData,
    image_sequence_metadata: Option<RafImageSequenceContainer>,
    metadata_container: RafMetadataContainer,
}

impl FromBinary for RafMetadataItem {
    fn read_from<R: Read + Seek>(reader: &mut BufBinaryReader<R>) -> io::Result<Self> {
        reader.set_endianness(Endian::Big);
        let tag = RafMetadataTag::from(reader.read::<u16>()?);
        let size: u16 = reader.read()?;
        let buf = reader.read_bytes(size as usize)?;
        let data: RafMetadataType;

        match tag {
            RafMetadataTag::ImageSequenceNumber | RafMetadataTag::ImageSequenceISO if size == 4 => {
                data =
                    RafMetadataType::Number(u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]));
            }
            RafMetadataTag::SensorDimensions
            | RafMetadataTag::OutputHeightWidth
            | RafMetadataTag::UnknownDimensions1
            | RafMetadataTag::UnknownDimensions1Inverted
            | RafMetadataTag::UnknownDimensions2
            if size == 4 =>
                {
                    data = RafMetadataType::Dimensions((
                        u16::from_be_bytes([buf[0], buf[1]]),
                        u16::from_be_bytes([buf[2], buf[3]]),
                    ));
                }
            RafMetadataTag::WhiteBalancePreset if size == 4 => {
                data =
                    RafMetadataType::Kelvin(u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]));
            }
            RafMetadataTag::FlashExposureComp if size == 4 => {
                data = RafMetadataType::ExposureValue(f32::from_be_bytes([
                    buf[0], buf[1], buf[2], buf[3],
                ]));
            }
            RafMetadataTag::ActiveAreaTopLeft | RafMetadataTag::ActiveAreaHeightWidth
            if size == 4 =>
                {
                    data = RafMetadataType::Position((
                        u16::from_be_bytes([buf[0], buf[1]]),
                        u16::from_be_bytes([buf[2], buf[3]]),
                    ));
                }
            RafMetadataTag::ActiveAreaAspectRatio if size == 4 => {
                data = RafMetadataType::AspectRatio((
                    u16::from_be_bytes([buf[0], buf[1]]),
                    u16::from_be_bytes([buf[2], buf[3]]),
                ));
            }
            RafMetadataTag::FujiModel | RafMetadataTag::FujiModel2 if size == 4 => {
                data = RafMetadataType::ASCIIText(buf.to_vec());
            }
            RafMetadataTag::RAWExposureBias
            | RafMetadataTag::UnknownExposureBias
            | RafMetadataTag::ImageSequenceAbsoluteExposure
            | RafMetadataTag::ImageSequenceRelativeExposure
            if size == 4 =>
                {
                    let a = i16::from_be_bytes([buf[0], buf[1]]);
                    let mut b = u16::from_be_bytes([buf[2], buf[3]]) as f32;
                    if b < 1.0 {
                        b = 1.0;
                    }
                    data = RafMetadataType::ExposureBias(a as f32 / b);
                }
            // Both shutter and aperture are assumed to be positive here (unsigned).
            RafMetadataTag::ImageSequenceShutterDuration if size == 8 => {
                data = {
                    let n = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
                    let d = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
                    if d == 0 {
                        RafMetadataType::Unknown(buf.to_vec())
                    } else {
                        RafMetadataType::ShutterDuration((n, d))
                    }
                }
            }
            RafMetadataTag::ImageSequenceApetureRatio if size == 8 => {
                data = {
                    let n = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
                    let d = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
                    if d == 0 {
                        RafMetadataType::Unknown(buf.to_vec())
                    } else {
                        RafMetadataType::ApertureRatio((n, d))
                    }
                }
            }
            // @TODO I'm sure there's something valuable here
            // RafMetadataTag::OtherData => {
            //     println!("other pos:{}", reader.stream_position()? - size as u64);
            //     println!("other  sz:{}", size);
            //     data = RafMetadataType::Unknown(buf.to_vec());
            // }
            // RafMetadataTag::UnknownData => {
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

impl FromBinary for RafImageSequenceContainer {
    fn read_from<R: Read + Seek>(reader: &mut BufBinaryReader<R>) -> io::Result<Self> {
        reader.set_endianness(Endian::Big);
        let additional_image_count = reader.read()?;
        let total_image_count = reader.read()?;
        let version = String::from_utf8(reader.read_bytes(12)?).unwrap();
        let image_sequence_metadata_size = reader.read()?;
        let image_sequence_metadata_count = reader.read()?;
        let mut image_sequence_images = Vec::with_capacity(total_image_count as usize);

        for _ in 0..total_image_count {
            let cfa_offset = reader.read()?;
            let cfa_size = reader.read()?;
            let mut metadata = Vec::with_capacity(image_sequence_metadata_count as usize);

            for _ in 0..image_sequence_metadata_count {
                metadata.push(RafMetadataItem::read_from(reader)?);
            }

            let image_sequence_image = RafImageSequenceImage {
                cfa_size,
                cfa_offset,
                metadata,
            };

            image_sequence_images.push(image_sequence_image);
        }

        Ok(Self {
            additional_image_count,
            total_image_count,
            version,
            image_sequence_metadata_size,
            image_sequence_metadata_count,
            image_sequence_images,
        })
    }
}

impl FromBinary for RafMetadataContainer {
    fn read_from<R: Read + Seek>(reader: &mut BufBinaryReader<R>) -> io::Result<Self> {
        reader.set_endianness(Endian::Big);

        let count = reader.read()?;

        // Put a sanity check here

        let mut metadata = Vec::with_capacity(count as usize);
        for _ in 0..count {
            metadata.push(RafMetadataItem::read_from(reader)?);
        }

        Ok(Self { count, metadata })
    }
}

impl FromBinary for Raf {
    fn read_from<R: Read + Seek>(reader: &mut BufBinaryReader<R>) -> io::Result<Self> {
        let buf = reader.read_bytes(16)?;
        if &buf != b"FUJIFILMCCD-RAW " {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid RAF filetype",
            ));
        }

        reader.set_endianness(Endian::Big);
        reader.skip_bytes(12)?;
        let camera = String::from_utf8(reader.read_bytes(32)?).unwrap();
        let version = reader.read()?;
        reader.skip_bytes(20)?;
        let jpeg_offset: u32 = reader.read()?;
        let jpeg_size: u32 = reader.read()?;
        let meta_container_offset: u32 = reader.read()?;

        // let meta_container_size: u32
        // let cfa_offset: u32
        // let cfa_size: u32

        reader.skip_bytes(12);

        // We can sense if this is an Image Sequence if the magic is here, or if we're at the JPEG Offset.
        let image_sequence_metadata = {
            if reader.stream_position()? + 60 >= jpeg_offset as u64 {
                None
            } else {
                reader.skip_bytes(40);
                let buf = reader.read_bytes(20)?;
                if &buf != b"FUJIFILMM-RAW   1.00" {
                    None
                } else {
                    Some(RafImageSequenceContainer::read_from(reader)?)
                }
            }
        };

        reader.seek(SeekFrom::Start(meta_container_offset as u64))?;
        let meta = RafMetadataContainer::read_from(reader)?;

        reader.seek(SeekFrom::Start(jpeg_offset as u64))?;
        let exif_data = ExifData::read_from(reader)?;

        Ok(Self {
            camera,
            version,
            exif_data,
            image_sequence_metadata,
            metadata_container: meta,
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
                        "{:?} u16: {}",
                        value,
                        u16::from_be_bytes([value[0], value[1]])
                    )
                } else if value.len() == 4 {
                    write!(
                        f,
                        "{:?} u32: {} [u16; 2]: {:?} f32: {:.8}",
                        value,
                        u32::from_be_bytes([value[0], value[1], value[2], value[3]]),
                        [
                            u16::from_be_bytes([value[0], value[1]]),
                            u16::from_be_bytes([value[2], value[3]])
                        ],
                        f32::from_be_bytes([value[0], value[1], value[2], value[3]])
                    )
                } else if value.len() == 8 {
                    write!(
                        f,
                        "{:?} [u32; 2]: {:?}",
                        value,
                        [
                            u32::from_be_bytes([value[0], value[1], value[2], value[3]]),
                            u32::from_be_bytes([value[4], value[5], value[6], value[7]])
                        ]
                    )
                } else if value.len() < 64 {
                    write!(f, "{:?}", value)
                } else {
                    write!(f, "{:?}", &value[0..64])?;
                    write!(f, " trimmed for size...")
                }
            }
            RafMetadataType::Dimensions((x, y)) => write!(f, "{x} x {y}"),
            RafMetadataType::Position((x, y)) => write!(f, "{x} {y}"),
            RafMetadataType::Kelvin(value) => write!(f, "{value}K"),
            RafMetadataType::ExposureValue(value) => write!(f, "{value} EV"),
            RafMetadataType::Number(value) => write!(f, "{value}"),
            RafMetadataType::AspectRatio((a, b)) => write!(f, "{a}:{b}"),
            RafMetadataType::ASCIIText(value) => write!(f, "{}", String::from_utf8_lossy(value)),
            RafMetadataType::ExposureBias(value) => write!(f, "{value:+.02}EV"),
            RafMetadataType::ShutterDuration((n, d)) => {
                if *n as f64 / *d as f64 > 1.0 {
                    write!(f, "{:.02}s", *n as f64 / *d as f64)
                } else {
                    write!(f, "{n}/{d}s")
                }
            }
            RafMetadataType::ApertureRatio((a, b)) => write!(f, "f/{:.02}", *a as f64 / *b as f64),
        }
    }
}

impl Display for RafMetadataItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "\t{}: {}", self.tag, self.data)
    }
}

impl Display for RafMetadataContainer {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for i in 0..(self.count as usize) {
            write!(f, "{}", &self.metadata[i])?;
        }
        Ok(())
    }
}

impl Display for RafImageSequenceImage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "\tCFA Offset: {}", self.cfa_offset)?;
        writeln!(f, "\tCFA Size: {}", self.cfa_size)?;
        for i in self.metadata.iter() {
            write!(f, "{}", i)?;
        }
        Ok(())
    }
}

impl Display for RafImageSequenceContainer {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "\tTotal Images in Sequence: {}", self.total_image_count)?;
        writeln!(f, "\tTag count: {}", self.image_sequence_metadata_count)?;
        writeln!(f, "\tVersion: {}", self.version)?;
        for i in 0..self.total_image_count as usize {
            writeln!(f, "Image {}:", i + 1)?;
            write!(f, "{}", &self.image_sequence_images[i])?;
        }
        Ok(())
    }
}

impl Display for Raf {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "Camera: {}", &self.camera)?;
        writeln!(
            f,
            "Version: \"{}\" {} 0x{:X}",
            str::from_utf8(&u32::to_be_bytes(self.version)).unwrap(),
            &self.version,
            &self.version
        )?;
        // writeln!(
        //     f,
        //     "JPEG Offset: {} 0x{:X}",
        //     &self.jpeg_offset, &self.jpeg_offset
        // )?;
        // writeln!(f, "JPEG Size: {} 0x{:X}", &self.jpeg_size, &self.jpeg_size)?;
        // writeln!(
        //     f,
        //     "Metadata Container Offset: {} 0x{:X}",
        //     &self.meta_container_offset, &self.meta_container_offset
        // )?;
        // writeln!(
        //     f,
        //     "Metadata Container Size: {} 0x{:X}",
        //     &self.meta_container_size, &self.meta_container_size
        // )?;
        // writeln!(
        //     f,
        //     "CFA Offset: {} 0x{:X}",
        //     &self.cfa_offset, &self.cfa_offset
        // )?;
        // writeln!(f, "CFA Size: {} 0x{:X}", &self.cfa_size, &self.cfa_size)?;
        if let Some(items) = &self.image_sequence_metadata {
            write!(f, "Image Sequence Metadata:\n{}", items)?;
        }
        write!(f, "Metadata: \n{}", &self.metadata_container)
    }
}