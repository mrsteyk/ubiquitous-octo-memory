#pragma pack(1)
typedef struct tagVTFHEADER
{
    char            signature[4];       // File signature ("VTF\0"). (or as little-endian integer, 0x00465456)
    unsigned int    version[2];         // version[0].version[1] (currently 7.2).
    unsigned int    headerSize;         // Size of the header struct  (16 byte aligned; currently 80 bytes) + size of the resources dictionary (7.3+).
    unsigned short  width;              // Width of the largest mipmap in pixels. Must be a power of 2.
    unsigned short  height;             // Height of the largest mipmap in pixels. Must be a power of 2.
    unsigned int    flags;              // VTF flags.
    unsigned short  frames;             // Number of frames, if animated (1 for no animation).
    unsigned short  firstFrame;         // First frame in animation (0 based). Can be -1 in environment maps older than 7.5, meaning there are 7 faces, not 6.
    unsigned char   padding0[4];        // reflectivity padding (16 byte alignment).
    float           reflectivity[3];    // reflectivity vector.
    unsigned char   padding1[4];        // reflectivity padding (8 byte packing).
    float           bumpmapScale;       // Bumpmap scale.
    unsigned int    highResImageFormat; // High resolution image format.
    unsigned char   mipmapCount;        // Number of mipmaps.
    unsigned int    lowResImageFormat;  // Low resolution image format (always DXT1).
    unsigned char   lowResImageWidth;   // Low resolution image width.
    unsigned char   lowResImageHeight;  // Low resolution image height.

    // 7.2+
    unsigned short  depth;              // Depth of the largest mipmap in pixels. Must be a power of 2. Is 1 for a 2D texture.

    // 7.3+
    unsigned char   padding2[3];        // depth padding (4 byte alignment).
    unsigned int    numResources;       // Number of resources this vtf has. The max appears to be 32.

    unsigned char   padding3[8];        // Necessary on certain compilers
} VTFHEADER;