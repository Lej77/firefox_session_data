use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::io;

#[test]
fn magic_header_length() {
    assert_eq!(super::MAGIC_HEADER_LENGTH, super::MAGIC_HEADER.len())
}

/// Print information about two buffers that should be equal but isn't.
#[derive(Debug)]
struct BufferComparer<'a> {
    pub actual: &'a [u8],
    pub expected: &'a [u8],
    pub compressed: bool,
}
impl std::fmt::Display for BufferComparer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use super::*;
        writeln!(f)?;
        writeln!(f)?;
        writeln!(f, "Actual length:   {}", self.actual.len())?;
        writeln!(f, "Expected length: {}", self.expected.len())?;
        writeln!(f)?;
        if self.compressed {
            let a_to_short = self.actual.len() < HEADER_LENGTH;
            let e_to_short = self.expected.len() < HEADER_LENGTH;
            if a_to_short && e_to_short {
                writeln!(
                    f,
                    "Both buffers were to short to fit a header ({} bytes required).",
                    HEADER_LENGTH
                )?;
            } else if a_to_short {
                writeln!(
                    f,
                    "Actual buffer were to short to fit a header ({} bytes required).",
                    HEADER_LENGTH
                )?;
            } else if e_to_short {
                writeln!(
                    f,
                    "Expected buffer were to short to fit a header ({} bytes required).",
                    HEADER_LENGTH
                )?;
            }
            if a_to_short || e_to_short {
                return Ok(());
            }
        }
        if self.compressed {
            if self.actual[..HEADER_LENGTH] == self.expected[..HEADER_LENGTH] {
                writeln!(f, "Headers matched!.")?;
                writeln!(f, "Header data: {:?}", &self.actual[..HEADER_LENGTH])?;
                writeln!(f)?;
            } else {
                writeln!(f)?;
                writeln!(f, "Headers didn't match!.")?;
                writeln!(f, "Actual Header:   {:?}", &self.actual[..HEADER_LENGTH])?;
                writeln!(f, "Expected Header: {:?}", &self.expected[..HEADER_LENGTH])?;
                writeln!(f, "Magic Header:    {:?}", MAGIC_HEADER)?;
                writeln!(f)?;
                writeln!(
                    f,
                    "Actual Header Text:   {}",
                    String::from_utf8_lossy(&self.actual[..HEADER_LENGTH])
                )?;
                writeln!(
                    f,
                    "Expected Header text: {}",
                    String::from_utf8_lossy(&self.expected[..HEADER_LENGTH])
                )?;
                writeln!(
                    f,
                    "Magic Header text:    {}",
                    String::from_utf8_lossy(MAGIC_HEADER)
                )?;
                writeln!(f)?;
                return Ok(());
            }
        }

        for (index, (actual, expected)) in self.actual.iter().zip(self.expected.iter()).enumerate()
        {
            if actual != expected {
                writeln!(f, "First error at index {}.", index)?;
                let mut end_index = index + 1000;
                if end_index >= self.actual.len() {
                    end_index = self.actual.len();
                }
                if end_index >= self.expected.len() {
                    end_index = self.expected.len();
                }

                writeln!(f)?;
                writeln!(
                    f,
                    "Actual data   [{}..{}]: {:?}.",
                    index,
                    end_index,
                    &self.actual[index..end_index]
                )?;
                writeln!(f)?;
                writeln!(
                    f,
                    "Expected data [{}..{}]: {:?}.",
                    index,
                    end_index,
                    &self.expected[index..end_index]
                )?;
                writeln!(f)?;
                writeln!(f)?;
                writeln!(f)?;

                if !self.compressed {
                    writeln!(
                        f,
                        "Actual text  [{}..{}]: {}.",
                        index,
                        end_index,
                        String::from_utf8_lossy(&self.actual[index..end_index])
                    )?;
                    writeln!(
                        f,
                        "Expected text [{}..{}]: {:?}.",
                        index,
                        end_index,
                        String::from_utf8_lossy(&self.expected[index..end_index])
                    )?;
                    writeln!(f)?;
                    writeln!(f)?;
                    writeln!(f)?;
                }
                break;
            }
        }
        Ok(())
    }
}

trait PrettyPanic<V> {
    fn unwrap_pretty(self) -> V;
}
impl<V, E> PrettyPanic<V> for Result<V, E>
where
    E: fmt::Display,
{
    fn unwrap_pretty(self) -> V {
        match self {
            Ok(t) => t,
            Err(e) => panic!(
                "called `PrettyPanic::unwrap_pretty()` on an `Err` value.\n\n{}",
                e
            ),
        }
    }
}

#[derive(Debug)]
enum DecompressValidationError {
    FailedValidation(super::SupportedCompressionLibrary, String),
    ReturnedError(super::SupportedCompressionLibrary, super::DecoderError),
    NotSupported(super::CompressionLibrary),
}
impl fmt::Display for DecompressValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DecompressValidationError::FailedValidation(library, info) => write!(f, "Data that was decompressed using {:?} didn't match the expected uncompressed data.{}", library, info),
            DecompressValidationError::ReturnedError(library, e) => write!(f, "Decompressing test data using \"{:?}\" resulted in an error: {:?}.", library, e),
            DecompressValidationError::NotSupported(library) => write!(f, "Could not test decompression for {:?} since it wasn't supported (i.e. Cargo feature was disabled)", library),
        }
    }
}
impl Error for DecompressValidationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            DecompressValidationError::ReturnedError(_, e) => Some(e),
            _ => None,
        }
    }
}

fn test_decompress(library: super::CompressionLibrary) -> Result<(), DecompressValidationError> {
    use super::*;
    use std::convert::TryFrom;

    let compressed_data = include_bytes!("./expected/sessionstore.jsonlz4");
    let target_data = include_bytes!("./expected/sessionstore.json");

    let library = SupportedCompressionLibrary::try_from(library)
        .map_err(|_| DecompressValidationError::NotSupported(library))?;

    let decompressed_data = decompress(compressed_data, library)
        .map_err(|e| DecompressValidationError::ReturnedError(library, e))?;

    if *decompressed_data != target_data[..] {
        Err(DecompressValidationError::FailedValidation(
            library,
            BufferComparer {
                actual: &decompressed_data,
                expected: &target_data[..],
                compressed: false,
            }
            .to_string(),
        ))
    } else {
        Ok(())
    }
}

#[derive(Debug)]
enum CompressValidationError {
    FailedValidation(super::SupportedCompressionLibrary, String),
    ReturnedError(super::SupportedCompressionLibrary, super::EncoderError),
    NotSupported(super::CompressionLibrary),
    FailedToReadFromEncoder(super::SupportedCompressionLibrary, io::Error),
}
impl fmt::Display for CompressValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompressValidationError::FailedValidation(library, info) => write!(
                f,
                "Data that was compressed using {:?} didn't match the expected compressed data.{}",
                library, info
            ),
            CompressValidationError::ReturnedError(library, e) => write!(
                f,
                "Compressing test data using \"{:?}\" resulted in an error: {:?}.",
                library, e
            ),
            CompressValidationError::NotSupported(library) => {
                write!(f, "Could not test compression for {:?}", library)
            }
            CompressValidationError::FailedToReadFromEncoder(library, e) => write!(
                f,
                "Failed to read from encoder using the {:?} crate. Error: {:?}",
                library, e
            ),
        }
    }
}
impl Error for CompressValidationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CompressValidationError::ReturnedError(_, e) => Some(e),
            CompressValidationError::FailedToReadFromEncoder(_, e) => Some(e),
            _ => None,
        }
    }
}

/// Test that a library can compress some Firefox data. Can also optionally
/// check that the produced output is exactly the same as the compressed file
/// Firefox generated.
fn test_compression(
    library: super::CompressionLibrary,
    expect_same_compression_as_firefox: bool,
) -> Result<(), CompressValidationError> {
    use super::*;
    use std::convert::TryFrom;

    let test_compressed_data = include_bytes!("./expected/sessionstore.jsonlz4");
    let test_decompressed_data = include_bytes!("./expected/sessionstore.json");
    let library = SupportedCompressionLibrary::try_from(library)
        .map_err(|_| CompressValidationError::NotSupported(library))?;

    let mut encoder = Encoder::compress(test_decompressed_data, None, library)
        .map_err(|e| CompressValidationError::ReturnedError(library, e))?;

    let mut buf = Vec::new();
    std::io::copy(&mut encoder, &mut buf)
        .map_err(|e| CompressValidationError::FailedToReadFromEncoder(library, e))?;

    if expect_same_compression_as_firefox && *buf != test_compressed_data[..] {
        Err(CompressValidationError::FailedValidation(
            library,
            BufferComparer {
                actual: &buf,
                expected: &test_compressed_data[..],
                compressed: true,
            }
            .to_string(),
        ))
    } else {
        Ok(())
    }
}

#[derive(Debug)]
enum CompressAndDecompressValidationError {
    NotSupported(super::CompressionLibrary),
    CompressError(super::SupportedCompressionLibrary, super::EncoderError),
    FailedToReadFromEncoder(super::SupportedCompressionLibrary, io::Error),
    DecompressError(super::SupportedCompressionLibrary, super::DecoderError),
    FailedValidation(super::SupportedCompressionLibrary, String),
}
impl fmt::Display for CompressAndDecompressValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompressAndDecompressValidationError::FailedValidation(library, info) => write!(
                f,
                "Data that was compressed using \"{:?}\" didn't match the expected compressed data.{}",
                library, info
            ),
            CompressAndDecompressValidationError::CompressError(library, e) => write!(
                f,
                "Compressing test data using \"{:?}\" resulted in an error: {:?}.",
                library, e
            ),
            CompressAndDecompressValidationError::NotSupported(library) => {
                write!(f, "Could not test compression and decompression for \"{:?}\" since it isn't supported.", library)
            }
            CompressAndDecompressValidationError::FailedToReadFromEncoder(library, e) => write!(
                f,
                "Failed to read from encoder using the \"{:?}\" crate. Error: {:?}",
                library, e
            ),
            CompressAndDecompressValidationError::DecompressError(library, e) => write!(
                f,
                "Decompressing test data using \"{:?}\" resulted in an error: {:?}.",
                library, e
            ),
        }
    }
}
impl Error for CompressAndDecompressValidationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CompressAndDecompressValidationError::CompressError(_, e) => Some(e),
            CompressAndDecompressValidationError::DecompressError(_, e) => Some(e),
            CompressAndDecompressValidationError::FailedToReadFromEncoder(_, e) => Some(e),
            _ => None,
        }
    }
}

/// Compress some data and then decompress it. Assert that the original data is equal to the decompressed data.
fn test_compression_and_decompression(
    library: super::CompressionLibrary,
) -> Result<(), CompressAndDecompressValidationError> {
    use super::*;
    use std::convert::TryFrom;

    let test_decompressed_data = include_bytes!("./expected/sessionstore.json");
    let library = SupportedCompressionLibrary::try_from(library)
        .map_err(|_| CompressAndDecompressValidationError::NotSupported(library))?;

    let mut encoder = Encoder::compress(test_decompressed_data, None, library)
        .map_err(|e| CompressAndDecompressValidationError::CompressError(library, e))?;

    let mut compressed_data = Vec::new();
    std::io::copy(&mut encoder, &mut compressed_data)
        .map_err(|e| CompressAndDecompressValidationError::FailedToReadFromEncoder(library, e))?;

    let decompressed_data = decompress(&compressed_data, library)
        .map_err(|e| CompressAndDecompressValidationError::DecompressError(library, e))?;

    if *decompressed_data != test_decompressed_data[..] {
        Err(CompressAndDecompressValidationError::FailedValidation(
            library,
            BufferComparer {
                actual: &decompressed_data,
                expected: &test_decompressed_data[..],
                compressed: false,
            }
            .to_string(),
        ))
    } else {
        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////
// Decompress
////////////////////////////////////////////////////////////////////////////////

#[test]
fn decompress_all() {
    for &library in super::CompressionLibrary::get_all() {
        test_decompress(library).unwrap_pretty();
    }
}

macro_rules! individual_decompress {
    ($(  $(#[$attr:meta])*  $variant:ident as $fn_name:ident),* $(,)?) => {
        const _: fn(super::CompressionLibrary) = |this| {
            match this {
                $(super::CompressionLibrary::$variant => {},)*
            }
        };
        $(
            $(#[$attr])*
            #[test]
            fn $fn_name() {
                test_decompress(super::CompressionLibrary::$variant).unwrap_pretty();
            }
        )*
    };
}
individual_decompress![
    Lz4 as decompress_lz4,
    Lz4Compress as decompress_lz4_compress,
    Lz4Compression as decompress_lz4_compression,
    Compress as decompress_compress,
    Lz4Flex as decompress_lz4_flex,
    PortedNodeLz4 as decompress_ported_node_lz4,
];

////////////////////////////////////////////////////////////////////////////////
// Compress
////////////////////////////////////////////////////////////////////////////////

#[test]
fn compress_all() {
    for &library in super::CompressionLibrary::get_all() {
        if library.panic_on_compress() {
            continue;
        }
        test_compression(library, library.same_as_firefox_compression()).unwrap_pretty();
    }
}

macro_rules! individual_compress {
    ($(  $(#[$attr:meta])*  $variant:ident as $fn_name:ident),* $(,)?) => {
        const _: fn(super::CompressionLibrary) = |this| {
            match this {
                $(super::CompressionLibrary::$variant => {},)*
            }
        };
        $(
            $(#[$attr])*
            #[test]
            fn $fn_name() {
                let library = super::CompressionLibrary::$variant;
                test_compression(library, library.same_as_firefox_compression()).unwrap_pretty();
            }
        )*
    };
}
individual_compress![
    Lz4 as compress_lz4,
    Lz4Compress as compress_lz4_compress,
    Lz4Compression as compress_lz4_compression,
    #[ignore = "this library panics when compressing"]
    Compress as compress_compress,
    Lz4Flex as compress_lz4_flex,
    #[ignore = "haven't ported code for compression yet"]
    PortedNodeLz4 as compress_ported_node_lz4,
];

////////////////////////////////////////////////////////////////////////////////
// Roundtrip (Compress and decompress back to original)
////////////////////////////////////////////////////////////////////////////////

#[test]
fn compress_and_decompress_all() {
    for &library in super::CompressionLibrary::get_all() {
        if library.panic_on_compress() {
            continue;
        }
        test_compression_and_decompression(library).unwrap_pretty();
    }
}

macro_rules! individual_compress_and_decompress {
    ($(  $(#[$attr:meta])*  $variant:ident as $fn_name:ident),* $(,)?) => {
        const _: fn(super::CompressionLibrary) = |this| {
            match this {
                $(super::CompressionLibrary::$variant => {},)*
            }
        };
        $(
            $(#[$attr])*
            #[test]
            fn $fn_name() {
                test_compression_and_decompression(super::CompressionLibrary::$variant).unwrap_pretty();
            }
        )*
    };
}
individual_compress_and_decompress![
    Lz4 as compress_and_decompress_lz4,
    Lz4Compress as compress_and_decompress_lz4_compress,
    Lz4Compression as compress_and_decompress_lz4_compression,
    #[ignore = "this library panics when compressing"]
    Compress as compress_and_decompress_compress,
    Lz4Flex as compress_and_decompress_lz4_flex,
    #[ignore = "haven't ported code for compression yet"]
    PortedNodeLz4 as compress_and_decompress_ported_node_lz4,
];

////////////////////////////////////////////////////////////////////////////////
// Check that library guarantees are correct
////////////////////////////////////////////////////////////////////////////////

#[test]
fn panic_on_compress_info() {
    for &library in super::CompressionLibrary::get_all() {
        let panic_err = std::panic::catch_unwind(|| {
            test_compression(library, false).ok();
        })
        .err();
        let panicked = panic_err.is_some();
        if panicked != library.panic_on_compress() {
            panic!(
                "panic_on_compress info is incorrect for {:?}. It {} when info indicated that it should{} have.{}{}",
                library,
                if panicked { "panicked" } else { "didn't panic" },
                if panicked { "'t" } else { "" },
                if panicked { "\nPanic message:\n" } else { "" },
                if let Some(panic_err) = panic_err {
                    if let Some(panic_msg) = panic_err.downcast_ref::<String>() {
                        Cow::from(panic_msg.clone())
                    } else if let Some(panic_msg) = panic_err.downcast_ref::<&'static str>() {
                        Cow::from(*panic_msg)
                    } else {
                        Cow::from("")
                    }
                } else {
                    Cow::from("")
                }
            );
        }
    }
}

#[test]
fn same_as_firefox_info() {
    for &library in super::CompressionLibrary::get_all() {
        let expected = library.same_as_firefox_compression();
        let result = std::panic::catch_unwind(|| test_compression(library, true));
        if matches!(result, Ok(Ok(()))) != expected {
            panic!(
                "`same_as_firefox_compression` info is incorrect for {:?}. {}",
                library,
                match result {
                    Err(panic_err) =>
                        if let Some(panic_msg) = panic_err.downcast_ref::<String>() {
                            format!("It panicked with the message: {}", panic_msg)
                        } else if let Some(panic_msg) = panic_err.downcast_ref::<&'static str>() {
                            format!("It panicked with the message: {}", panic_msg)
                        } else {
                            "It panicked.".to_string()
                        },
                    Ok(Err(validation_err)) => format!("It didn't produce the same compressed file as Firefox did: {}", validation_err),
                    Ok(Ok(())) => "Succeeded in producing the same compressed data as Firefox did when this wasn't expected.".to_owned(),
                }
            );
        }
    }
}
