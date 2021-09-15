#[allow(non_snake_case)]
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct VTFHEADER {
    pub signature: [::std::os::raw::c_char; 4usize],
    pub version: [::std::os::raw::c_uint; 2usize],
    pub headerSize: ::std::os::raw::c_uint,
    pub width: ::std::os::raw::c_ushort,
    pub height: ::std::os::raw::c_ushort,
    pub flags: ::std::os::raw::c_uint,
    pub frames: ::std::os::raw::c_ushort,
    pub firstFrame: ::std::os::raw::c_ushort,
    pub padding0: [::std::os::raw::c_uchar; 4usize],
    pub reflectivity: [f32; 3usize],
    pub padding1: [::std::os::raw::c_uchar; 4usize],
    pub bumpmapScale: f32,
    pub highResImageFormat: ::std::os::raw::c_uint,
    pub mipmapCount: ::std::os::raw::c_uchar,
    pub lowResImageFormat: ::std::os::raw::c_uint,
    pub lowResImageWidth: ::std::os::raw::c_uchar,
    pub lowResImageHeight: ::std::os::raw::c_uchar,
    pub depth: ::std::os::raw::c_ushort,
    pub padding2: [::std::os::raw::c_uchar; 3usize],
    pub numResources: ::std::os::raw::c_uint,
    pub padding3: [::std::os::raw::c_uchar; 8usize],
}
