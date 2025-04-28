//! This module handles compression and decompression of the `mozLz4` format that is used for
//! Firefox user data such a session data and bookmarks. These files can for example have the
//! `.jsonlz4` file extension.
//!
//! # References
//!
//! ## C
//!
//! Uses a lz4 library that is cloned from Mozilla's repository. Focuses on decompression, but has code to do both.
//!
//! [https://github.com/avih/dejsonlz4](https://github.com/avih/dejsonlz4)
//!
//! #### Compression
//!
//! [https://github.com/avih/dejsonlz4/blob/master/src/ref_compress/jsonlz4.c](https://github.com/avih/dejsonlz4/blob/master/src/ref_compress/jsonlz4.c)
//!
//! #### Decompression
//!
//! [https://github.com/avih/dejsonlz4/blob/master/src/dejsonlz4.c](https://github.com/avih/dejsonlz4/blob/master/src/dejsonlz4.c)
//!
//! [https://github.com/andikleen/lz4json](https://github.com/andikleen/lz4json)
//!
//! ## Python
//!
//! Uses "LZ4 bindings for Python". Can do both decompression and compression.
//!
//! [https://gist.github.com/Tblue/62ff47bef7f894e92ed5](https://gist.github.com/Tblue/62ff47bef7f894e92ed5)
//!
//!
//! ## Rust
//!
//! Can do both decompression and compression. Links to an existing lz4 library? Uses the `#[link(name="lz4")]` attribute with `extern` functions.
//!
//! [https://github.com/lilydjwg/mozlz4-tool](https://github.com/lilydjwg/mozlz4-tool)
//!
//!
//! # JavaScript
//!
//! Can do both decompression and compression. Implemented in pure JavaScript.
//!
//! [https://github.com/pierrec/node-lz4/blob/master/lib/binding.js][https://github.com/pierrec/node-lz4/blob/master/lib/binding.js]
//!

#[cfg(feature = "compression")]
use byteorder::{ByteOrder, LittleEndian};
use std::borrow::Cow;
use std::convert::TryFrom;
use std::error::Error;
use std::fmt;
use std::io;

pub mod node_lz4_port;
#[cfg(test)]
mod tests;

pub const MAGIC_HEADER: &[u8] = b"mozLz40\0";
pub const MAGIC_HEADER_LENGTH: usize = 8;
pub const HEADER_LENGTH: usize = 8 + 4;

/// Represents the compression mode to be used.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum CompressionMode {
    /// High compression with compression parameter
    HIGHCOMPRESSION(i32),
    /// Fast compression with acceleration parameter
    FAST(i32),
    /// Default compression
    #[default]
    DEFAULT,
}
#[cfg(all(feature = "compression_lz4", not(target_family = "wasm")))]
impl From<CompressionMode> for lz4::block::CompressionMode {
    fn from(value: CompressionMode) -> lz4::block::CompressionMode {
        use lz4::block::CompressionMode as Lz4CompressionMode;
        match value {
            CompressionMode::HIGHCOMPRESSION(v) => Lz4CompressionMode::HIGHCOMPRESSION(v),
            CompressionMode::FAST(v) => Lz4CompressionMode::FAST(v),
            CompressionMode::DEFAULT => Lz4CompressionMode::DEFAULT,
        }
    }
}

/// Indicate what library to use for compression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionLibrary {
    /// Uses the `lz4` crate with bindings to a C library.
    Lz4,
    /// Uses a pure Rust implementation from the `compress` crate that contains "Various compression algorithms written in rust".
    Compress,
    /// Uses a pure Rust implementation from the `lz4-compression` crate.
    Lz4Compression,
    /// Uses a pure Rust implementation from the `lz4-compress` crate.
    Lz4Compress,
    /// Uses a pure Rust implementation from the `lz4_flex` crate.
    Lz4Flex,
    /// Uses a pure Rust implementation that was ported from `node-lz4`.
    PortedNodeLz4,
}
impl CompressionLibrary {
    pub const fn is_supported(self) -> bool {
        self.try_into_supported().is_some()
    }
    pub const fn try_into_supported(self) -> Option<SupportedCompressionLibrary> {
        SupportedCompressionLibrary::try_from_compression_lib(self)
    }

    /// `true` if the library is likely to panic when compressing data.
    pub const fn panic_on_compress(self) -> bool {
        match self {
            CompressionLibrary::Lz4 => false,
            CompressionLibrary::Compress => true,
            CompressionLibrary::Lz4Compression => false,
            CompressionLibrary::Lz4Compress => false,
            CompressionLibrary::Lz4Flex => false,
            CompressionLibrary::PortedNodeLz4 => true,
        }
    }

    /// `true` if the library produces byte perfect compressed files that would
    /// match what Firefox would produce when compressing some data.
    pub const fn same_as_firefox_compression(self) -> bool {
        match self {
            CompressionLibrary::Lz4 => true,
            CompressionLibrary::Compress => false,
            CompressionLibrary::Lz4Compression => false,
            CompressionLibrary::Lz4Compress => false,
            CompressionLibrary::Lz4Flex => false,
            CompressionLibrary::PortedNodeLz4 => false,
        }
    }

    pub const fn get_all() -> &'static [Self] {
        macro_rules! all {
            ($($variant:ident),* $(,)?) => {{
                let _ = |this: Self| {
                    match this {
                        $(Self::$variant {} => {},)*
                    }
                };
                [$(Self::$variant,)*]
            }};
        }
        &all![
            Lz4,
            Compress,
            Lz4Compression,
            Lz4Compress,
            Lz4Flex,
            PortedNodeLz4,
        ]
    }
}

/// Indicate what library to use for compression. Contains only libraries that are currently supported (controlled by cargo features).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedCompressionLibrary {
    #[cfg(all(feature = "compression_lz4", not(target_family = "wasm")))]
    /// Uses the `lz4` crate with bindings to a C library.
    Lz4,
    #[cfg(feature = "compression_compress")]
    /// Uses a pure Rust implementation from the `compress` crate that contains "Various compression algorithms written in rust".
    Compress,
    #[cfg(feature = "compression_lz4_compression")]
    /// Uses a pure Rust implementation from the `lz4-compression` crate.
    Lz4Compression,
    #[cfg(feature = "compression_lz4_compress")]
    /// Uses a pure Rust implementation from the `lz4-compress` crate.
    Lz4Compress,
    /// Uses a pure Rust implementation from the `lz4_flex` crate.
    #[cfg(feature = "compression_lz4_flex")]
    Lz4Flex,
    /// Uses a pure Rust implementation that was ported from `node-lz4`.
    PortedNodeLz4,
}
impl SupportedCompressionLibrary {
    pub const fn try_from_compression_lib(lib: CompressionLibrary) -> Option<Self> {
        match lib {
            CompressionLibrary::Lz4 => {
                #[cfg(all(feature = "compression_lz4", not(target_family = "wasm")))]
                return Some(SupportedCompressionLibrary::Lz4);
            }
            CompressionLibrary::Compress => {
                #[cfg(feature = "compression_compress")]
                return Some(SupportedCompressionLibrary::Compress);
            }
            CompressionLibrary::Lz4Compression => {
                #[cfg(feature = "compression_lz4_compression")]
                return Some(SupportedCompressionLibrary::Lz4Compression);
            }
            CompressionLibrary::Lz4Compress => {
                #[cfg(feature = "compression_lz4_compress")]
                return Some(SupportedCompressionLibrary::Lz4Compress);
            }
            CompressionLibrary::Lz4Flex => {
                #[cfg(feature = "compression_lz4_flex")]
                return Some(SupportedCompressionLibrary::Lz4Flex);
            }
            CompressionLibrary::PortedNodeLz4 => {
                return Some(SupportedCompressionLibrary::PortedNodeLz4);
            }
        };

        #[allow(unreachable_code)]
        None
    }
    pub const fn to_compression_lib(self) -> CompressionLibrary {
        match self {
            #[cfg(all(feature = "compression_lz4", not(target_family = "wasm")))]
            SupportedCompressionLibrary::Lz4 => CompressionLibrary::Lz4,
            #[cfg(feature = "compression_compress")]
            SupportedCompressionLibrary::Compress => CompressionLibrary::Compress,
            #[cfg(feature = "compression_lz4_compression")]
            SupportedCompressionLibrary::Lz4Compression => CompressionLibrary::Lz4Compression,
            #[cfg(feature = "compression_lz4_compress")]
            SupportedCompressionLibrary::Lz4Compress => CompressionLibrary::Lz4Compress,
            #[cfg(feature = "compression_lz4_flex")]
            SupportedCompressionLibrary::Lz4Flex => CompressionLibrary::Lz4Flex,
            SupportedCompressionLibrary::PortedNodeLz4 => CompressionLibrary::PortedNodeLz4,
        }
    }
}
impl TryFrom<CompressionLibrary> for SupportedCompressionLibrary {
    type Error = ();
    fn try_from(value: CompressionLibrary) -> Result<Self, Self::Error> {
        Self::try_from_compression_lib(value).ok_or(())
    }
}
impl From<SupportedCompressionLibrary> for CompressionLibrary {
    fn from(value: SupportedCompressionLibrary) -> Self {
        value.to_compression_lib()
    }
}

#[derive(Debug)]
pub enum EncoderError {
    UncompressedDataBufferIsTooLong(io::Error),
    InternalCLibraryError(io::Error),
    UnknownError(io::Error),
}
impl fmt::Display for EncoderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use EncoderError::*;
        match self {
            UncompressedDataBufferIsTooLong(_) => write!(f, "Failed to compress data because the uncompressed data buffer was too long."),
            InternalCLibraryError(_) => write!(f, "Failed to compress data because of an internal compression error in the C Library."),
            UnknownError(_) => write!(f, "Failed to compress data."),
        }
    }
}
impl Error for EncoderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        use EncoderError::*;
        match &self {
            UncompressedDataBufferIsTooLong(e) => Some(e),
            InternalCLibraryError(e) => Some(e),
            UnknownError(e) => Some(e),
        }
    }
}
impl From<io::Error> for EncoderError {
    fn from(value: io::Error) -> Self {
        match value.kind() {
            io::ErrorKind::Other => EncoderError::InternalCLibraryError(value),
            io::ErrorKind::InvalidInput => EncoderError::UncompressedDataBufferIsTooLong(value),
            _ => EncoderError::UnknownError(value),
        }
    }
}

pub struct Encoder {
    compressed_data: Vec<u8>,
    uncompressed_size: usize,
    index: usize,
}
impl Encoder {
    #[allow(unreachable_code, unused_variables)] // <- when all features are disabled
    pub fn compress(
        uncompressed_data: &[u8],
        mode: Option<CompressionMode>,
        library: SupportedCompressionLibrary,
    ) -> Result<Self, EncoderError> {
        // TODO: Figure out which compression crates include size as header info before compressed data.
        let compressed_data = match library {
            #[cfg(all(feature = "compression_lz4", not(target_family = "wasm")))]
            SupportedCompressionLibrary::Lz4 => {
                lz4::block::compress(uncompressed_data, mode.map(Into::into), false)?
            }
            #[cfg(feature = "compression_compress")]
            SupportedCompressionLibrary::Compress => {
                let mut data = match compress::lz4::compression_bound(uncompressed_data.len() as u32)
                {
                    Some(upper_bound) => Vec::with_capacity(upper_bound as usize),
                    None => Vec::new(),
                };
                compress::lz4::encode_block(uncompressed_data, &mut data);
                data
            }
            #[cfg(feature = "compression_lz4_compression")]
            SupportedCompressionLibrary::Lz4Compression => {
                lz4_compression::compress::compress(uncompressed_data)
            }
            #[cfg(feature = "compression_lz4_compress")]
            SupportedCompressionLibrary::Lz4Compress => lz4_compress::compress(uncompressed_data),
            #[cfg(feature = "compression_lz4_flex")]
            SupportedCompressionLibrary::Lz4Flex => lz4_flex::compress(uncompressed_data),
            SupportedCompressionLibrary::PortedNodeLz4 => unimplemented!(),
        };

        Ok(Self {
            compressed_data,
            uncompressed_size: uncompressed_data.len(),
            index: 0,
        })
    }

    /// Get the header that this encoder would write.
    pub fn get_header(&self) -> [u8; HEADER_LENGTH] {
        let mut buf = [0; HEADER_LENGTH];

        buf[0..MAGIC_HEADER_LENGTH].copy_from_slice(MAGIC_HEADER);
        #[cfg(feature = "compression")]
        LittleEndian::write_u32(
            &mut buf[MAGIC_HEADER_LENGTH..],
            self.uncompressed_size as u32,
        );
        #[cfg(not(feature = "compression"))]
        unreachable!("No compression feature enabled.");
        buf
    }
    /// This will contain the compressed data without the header that should be written before it.
    pub fn get_vec_without_header(self) -> Vec<u8> {
        self.compressed_data
    }
}
impl io::Read for Encoder {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut n = 0;
        let mut write = |data: &[u8]| {
            let buf = &mut buf[n..];
            let n_to_write = if buf.len() < data.len() {
                buf.len()
            } else {
                data.len()
            };
            buf[..n_to_write].copy_from_slice(&data[..n_to_write]);
            n += n_to_write;
            n_to_write
        };
        if self.index < HEADER_LENGTH {
            // Need to write header.
            self.index += write(&self.get_header());
        }
        if self.index >= HEADER_LENGTH {
            let data_start = self.index - HEADER_LENGTH;
            if data_start < self.compressed_data.len() {
                self.index += write(&self.compressed_data[data_start..]);
            }
        }

        Ok(n)
    }
}

#[derive(Debug)]
pub enum DecoderError {
    UncompressedDataBufferIsTooShort(Option<io::Error>, Option<u32>),
    BadHeader([u8; MAGIC_HEADER_LENGTH]),
    InternalCLibraryError(io::Error),
    UnknownIoError(io::Error),
    TextError(String),
    InvalidDeduplicationOffset,
    PortedNodeLz4Error,
}
impl fmt::Display for DecoderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use DecoderError::*;
        match self {
            UncompressedDataBufferIsTooShort(_, uncompressed_size) => {
                write!(f, "Failed to decompress data because the compressed data buffer was too short or because the uncompressed size that was parsed from the header{} was too large or negative.", match uncompressed_size {
                    Some(uncompressed_size) => Cow::from(format!(" ({})", uncompressed_size)),
                    None => Cow::from(""),
                })
            } ,
            InternalCLibraryError(_) => write!(f, "Failed to decompress data because of an internal decompression error in the C Library"),
            UnknownIoError(_) => write!(f, "Failed to decompress data"),
            BadHeader(d) => write!(f, "Failed to decompress data because of a Bad header: expected \"{:?}\" followed by 4 bytes of uncompressed size but found \"{:?}\"", MAGIC_HEADER, d),
            InvalidDeduplicationOffset => write!(f, "Failed to decompress data because the offset for a de-duplication was out of bounds. The offset to copy was not contained in the decompressed buffer"),
            TextError(s) => write!(f, "Failed to decompress data: {}", s),
            PortedNodeLz4Error => write!(f, "Failed to decompress data using code ported from the \"node-lz4\" library")
        }
    }
}
impl Error for DecoderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        use DecoderError::*;
        match &self {
            UncompressedDataBufferIsTooShort(e, _) => Some(e.as_ref()?),
            InternalCLibraryError(e) => Some(e),
            UnknownIoError(e) => Some(e),
            BadHeader(_) => None,
            InvalidDeduplicationOffset => None,
            TextError(_) => None,
            PortedNodeLz4Error => None,
        }
    }
}

pub fn decompress(
    mut data: &[u8],
    library: SupportedCompressionLibrary,
) -> Result<Vec<u8>, DecoderError> {
    if data.len() < HEADER_LENGTH {
        return Err(DecoderError::UncompressedDataBufferIsTooShort(None, None));
    }
    if data.len() < MAGIC_HEADER_LENGTH || &data[..MAGIC_HEADER_LENGTH] != MAGIC_HEADER {
        let mut header_data = [0; MAGIC_HEADER_LENGTH];
        header_data.copy_from_slice(&data[..MAGIC_HEADER_LENGTH]);
        return Err(DecoderError::BadHeader(header_data));
    }
    data = &data[MAGIC_HEADER_LENGTH..];

    #[cfg(not(feature = "compression"))]
    unreachable!("No compression feature enabled.");

    #[cfg(feature = "compression")]
    {
        let _data_with_size = data;
        let uncompressed_size = LittleEndian::read_u32(data);
        data = &data[4..];

        match library {
            #[cfg(all(feature = "compression_lz4", not(target_family = "wasm")))]
            SupportedCompressionLibrary::Lz4 => {
                lz4::block::decompress(data, Some(uncompressed_size as i32)).map_err(|e| {
                    match e.kind() {
                        io::ErrorKind::InvalidData => DecoderError::InternalCLibraryError(e),
                        io::ErrorKind::InvalidInput => {
                            DecoderError::UncompressedDataBufferIsTooShort(
                                Some(e),
                                Some(uncompressed_size),
                            )
                        }
                        _ => DecoderError::UnknownIoError(e),
                    }
                })
            }
            #[cfg(feature = "compression_compress")]
            SupportedCompressionLibrary::Compress => {
                let mut uncompressed_data = Vec::with_capacity(uncompressed_size as usize);
                let _processed_bytes = compress::lz4::decode_block(data, &mut uncompressed_data);
                Ok(uncompressed_data)
            }
            #[cfg(feature = "compression_lz4_compression")]
            SupportedCompressionLibrary::Lz4Compression => {
                lz4_compression::decompress::decompress(data).map_err(|e| {
                    use lz4_compression::decompress::Error::*;
                    match e {
                        UnexpectedEnd => DecoderError::UncompressedDataBufferIsTooShort(
                            None,
                            Some(uncompressed_size),
                        ),
                        InvalidDeduplicationOffset => DecoderError::InvalidDeduplicationOffset,
                    }
                })
            }
            #[cfg(feature = "compression_lz4_compress")]
            SupportedCompressionLibrary::Lz4Compress => {
                lz4_compress::decompress(data).map_err(|e| DecoderError::TextError(e.to_string()))
            }
            #[cfg(feature = "compression_lz4_flex")]
            SupportedCompressionLibrary::Lz4Flex => {
                lz4_flex::decompress_size_prepended(_data_with_size).map_err(|e| {
                    use lz4_flex::block::DecompressError::*;
                    match e {
                        OffsetOutOfBounds => DecoderError::InvalidDeduplicationOffset,
                        _ => DecoderError::TextError(e.to_string()),
                    }
                })
            }
            SupportedCompressionLibrary::PortedNodeLz4 => {
                let mut output = Vec::with_capacity(uncompressed_size as usize);
                node_lz4_port::decompress(data, &mut output)
                    .map_err(|_| DecoderError::PortedNodeLz4Error)?;
                Ok(output)
            }
        }
    }
}
