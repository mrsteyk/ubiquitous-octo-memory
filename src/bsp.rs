use std::error::Error;
use std::fmt;

#[macro_export]
macro_rules! lump_helper {
    ($value:expr, $pattern:pat => $extracted_value:expr) => {
        match $value {
            $pattern => $extracted_value,
            _ => unreachable!(),
        }
    };
}

#[derive(Debug)]
pub enum BSPError {
    InvalidHeader(u32),
    InvalidVersion(u32),

    InvalidPakFile(u32),
}

impl fmt::Display for BSPError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHeader(v) => write!(f, "Invalid header: {:08X}!", v),
            Self::InvalidVersion(v) => write!(f, "Invalid version: {}, must be 19 or 20!", v),
            Self::InvalidPakFile(v) => write!(f, "Invalid PakFile data at: {:X}!", v),
        }
    }
}

impl Error for BSPError {}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct BasicLump {
    pub offset: u32,
    pub size: u32,
    pub version: u32,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct EntityLump {
    pub base: BasicLump,

    pub string: String, // TODO...
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct PakFile {
    pub name: (u32, u32),
    pub data: (u32, u32),

    pub remove: bool,
}

impl PakFile {
    pub fn data<'a>(&self, pak: &'a [u8]) -> &'a [u8] {
        &pak[self.data.0 as usize..(self.data.0 + self.data.1) as usize]
    }

    pub fn name<'a>(&self, pak: &'a [u8]) -> &'a str {
        unsafe {
            std::str::from_utf8_unchecked(
                &pak[self.name.0 as usize..(self.name.0 + self.name.1) as usize],
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct PakFileLump {
    pub base: BasicLump,

    pub files: Vec<PakFile>,
}

impl PakFileLump {
    pub fn data<'a>(&self, buf: &'a [u8]) -> &'a [u8] {
        &buf[self.base.offset as usize..(self.base.offset + self.base.size) as usize]
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum BSPLump {
    Entities(EntityLump), // 0
    PakFile(PakFileLump), // 40
    // ---
    Unknown(BasicLump),
    None,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct ParsedMap {
    // pub name: String,
    pub version: u32,
    pub iteration: u32,
    pub lumps: [BSPLump; 64],
    pub buf: Vec<u8>, // move out vector to allow less memory allocation for the web

    pub order: Vec<(u8, u32)>,
}

impl ParsedMap {
    pub fn new(buf: Vec<u8>) -> Result<Self, Box<dyn Error>> {
        // TBH, I don't really feel like pulling byteorder for this...
        let version = unsafe {
            let buf_u32 = buf.as_ptr().cast::<u32>();
            if *buf_u32 != 0x50_53_42_56 {
                return Err(Box::new(BSPError::InvalidHeader(*buf_u32)));
            }
            let ver = *buf_u32.add(1);
            if ver != 19 && ver != 20 {
                return Err(Box::new(BSPError::InvalidVersion(ver)));
            }

            ver
        };

        // hacky way to not allocate memory that much...
        const INIT: BSPLump = BSPLump::None;
        let mut lumps = [INIT; 64];
        for i in 0..64usize {
            let (offset, size, version) = unsafe {
                let lump_start = buf.as_ptr().add(8 + i * 16).cast::<u32>();
                (*lump_start, *lump_start.add(1), *lump_start.add(2))
            };
            let base = BasicLump {
                offset,
                size,
                version,
            };
            lumps[i] = match i {
                0 => BSPLump::Entities(EntityLump {
                    base,
                    string: unsafe {
                        let delta = if buf[(offset + size - 1) as usize] == 0 {
                            1
                        } else {
                            0
                        };
                        std::str::from_utf8_unchecked(
                            &buf[offset as usize..(offset + size - delta) as usize],
                        )
                        .to_string()
                    },
                }),
                40 => {
                    // Coding at night with constraints be like
                    let file = &buf[offset as usize..(offset + size) as usize];
                    let mut position = 0usize;

                    let mut files = Vec::<PakFile>::new();

                    while (position as u32) < size {
                        let header_pos = position as u32;

                        let header = &file[position..position + 30];
                        position += 30;
                        match &header[0..4] {
                            &[0x50, 0x4B, 3, 4] => {
                                // // Min version
                                // if header[4] > 0xA {
                                //     return Err(Box::new(BSPError::InvalidPakFile(header_pos + 4)));
                                // }

                                // Must be STORE
                                if header[8] != 0 || header[9] != 0 {
                                    return Err(Box::new(BSPError::InvalidPakFile(header_pos + 8)));
                                }

                                let data_size =
                                    unsafe { *((&header[22..26]).as_ptr().cast::<u32>()) };
                                let name_size =
                                    unsafe { *((&header[26..28]).as_ptr().cast::<u16>()) };
                                let extra_size =
                                    unsafe { *((&header[28..30]).as_ptr().cast::<u16>()) };

                                // let name = unsafe {
                                //     std::str::from_utf8_unchecked(
                                //         &file[position..position + name_size as usize],
                                //     )
                                // };
                                let name = (position as u32, name_size as u32);
                                position += name_size as usize;
                                position += extra_size as usize;

                                // let data = &file[position..position + data_size as usize];
                                let data = (position as u32, data_size);
                                position += data_size as usize;

                                files.push(PakFile {
                                    name,
                                    data,
                                    remove: false,
                                })
                            }
                            &[0x50, 0x4B, 1, 2] => {
                                break; // Central directory aka ending stuff
                            }
                            _ => {
                                return Err(Box::new(BSPError::InvalidPakFile(header_pos)));
                            }
                        }
                    }

                    BSPLump::PakFile(PakFileLump { base, files })
                }
                _ => BSPLump::Unknown(base),
            };
        }

        let iteration = unsafe { *buf.as_ptr().add(0x408).cast::<u32>() };

        let mut order = (0..64u8)
            .map(|i| {
                let offset = unsafe {
                    let lump_start = buf.as_ptr().add(8 + i as usize * 16).cast::<u32>();
                    *lump_start
                };
                (i, offset)
            })
            .collect::<Vec<_>>();
        order.sort_by_key(|f| f.1);

        Ok(Self {
            version,
            iteration,
            lumps,
            buf,

            order, // for best compatibility
        })
    }
}
