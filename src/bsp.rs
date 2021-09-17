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

    pub four: u32, // fucking v21
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct EntityLump {
    pub base: BasicLump,

    pub string: String, // TODO...
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum PakAlgo {
    None,
    LZMA(u32, u32), // comp, decomp
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct PakFile {
    pub name: (u32, u32),
    pub data: (u32, u32),

    pub real_data: Option<Vec<u8>>,
    pub compression_algo: PakAlgo,

    pub remove: bool,
}

impl PakFile {
    pub fn data<'a>(&'a self, pak: &'a [u8]) -> &'a [u8] {
        if let Some(data) = &self.real_data {
            data.as_slice()
        } else {
            &pak[self.data.0 as usize..(self.data.0 + self.data.1) as usize]
        }
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
            if ver != 19 && ver != 20 && ver != 21 {
                return Err(Box::new(BSPError::InvalidVersion(ver)));
            }

            ver
        };

        // hacky way to not allocate memory that much...
        const INIT: BSPLump = BSPLump::None;
        let mut lumps = [INIT; 64];
        for i in 0..64usize {
            let (offset, size, version, four) = unsafe {
                let lump_start = buf.as_ptr().add(8 + i * 16).cast::<u32>();
                (*lump_start, *lump_start.add(1), *lump_start.add(2), *lump_start.add(3))
            };
            let base = BasicLump {
                offset,
                size,
                version,
                four,
            };
            lumps[i] = match i {
                0 => BSPLump::Entities(EntityLump {
                    base,
                    string: unsafe {
                        // LZMA
                        if &buf[offset as usize..(offset+4) as usize] == &[0x4C, 0x5A, 0x4D, 0x41] {
                            let buf_ptr = buf.as_ptr().add(offset as usize).cast::<u32>();
                            let decompressed_size = *buf_ptr.add(1);
                            let compressed_size = *buf_ptr.add(2);

                            // eprintln!("{} | {} | {}", compressed_size, size, four);

                            // let mut input = std::io::Cursor::new(&buf[(offset+12) as usize..(offset+compressed_size) as usize]);
                            let mut input = std::io::Cursor::new(&buf[(offset+12) as usize..(offset+size) as usize]); // offset as usize + compressed_size as usize
                            let mut output = Vec::<u8>::with_capacity(decompressed_size as usize);
                            lzma_rs::lzma_decompress_with_options(&mut input, &mut output, &lzma_rs::decompress::Options {
                                unpacked_size: lzma_rs::decompress::UnpackedSize::UseProvided(Some(decompressed_size as u64)),
                                ..Default::default()
                            }).unwrap();
                            let ret = std::str::from_utf8_unchecked(&output).to_string();

                            // eprintln!("{}", &ret);
                            eprintln!("{} | {} | {} | {} ||| {}", compressed_size, decompressed_size, size, four, ret.len());

                            #[cfg(debug_assertions)]
                            {
                                assert_eq!(decompressed_size, four);
                                assert_eq!(decompressed_size, ret.len() as u32);
                            }

                            ret
                        } else {
                            let delta = if buf[(offset + size - 1) as usize] == 0 {
                                1
                            } else {
                                0
                            };
                            std::str::from_utf8_unchecked(
                                &buf[offset as usize..(offset + size - delta) as usize],
                            )
                            .to_string()
                        }
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

                                if header[8] != 0 || header[9] != 0 {
                                    //return Err(Box::new(BSPError::InvalidPakFile(header_pos + 8)));
                                    if header[8] == 0xE &&  header[9] == 0 {
                                        // LZMA
                                        let compressed_size =
                                            unsafe { *((&header[18..22]).as_ptr().cast::<u32>()) };
                                        let data_size =
                                            unsafe { *((&header[22..26]).as_ptr().cast::<u32>()) };
                                        let name_size =
                                            unsafe { *((&header[26..28]).as_ptr().cast::<u16>()) };
                                        let extra_size =
                                            unsafe { *((&header[28..30]).as_ptr().cast::<u16>()) };
                                        
                                        if extra_size != 0 {
                                            return Err(Box::new(BSPError::InvalidPakFile(header_pos + 28)));
                                        }

                                        let name = (position as u32, name_size as u32);
                                        position += name_size as usize;
                                        position += extra_size as usize;

                                        let compressed_data = (position as u32, compressed_size);
                                        let real_data = {
                                            // Explanation:
                                            // LZMA in ZIP spec: u16(version), u16(props_size)
                                            let mut r = std::io::Cursor::new(&file[position + 4..position + compressed_size as usize]);
                                            let mut decomp = Vec::<u8>::with_capacity(data_size as usize);
                                            if let Err(v) = lzma_rs::lzma_decompress_with_options(&mut r, &mut decomp, &lzma_rs::decompress::Options {
                                                unpacked_size: lzma_rs::decompress::UnpackedSize::UseProvided(Some(data_size as u64)),
                                                ..Default::default()
                                            }) {
                                                eprintln!("{:#?}", v);
                                                return Err(Box::new(BSPError::InvalidPakFile(position as u32)));
                                            } else {
                                                Some(decomp)
                                            }
                                        };
                                        position += compressed_size as usize;

                                        files.push(PakFile {
                                            name,
                                            data: compressed_data,
                                            remove: false,
        
                                            real_data,
                                            compression_algo: PakAlgo::LZMA(compressed_size, data_size),
                                        })
                                    } else {
                                        return Err(Box::new(BSPError::InvalidPakFile(header_pos + 8)));
                                    }
                                } else {
                                    // STORE

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
    
                                        real_data: None,
                                        compression_algo: PakAlgo::None,
                                    })
                                }
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
