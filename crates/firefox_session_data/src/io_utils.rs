use std::{
    borrow::Cow,
    convert::AsRef,
    error::Error as StdError,
    fmt,
    fs::File,
    io::{self, BufReader, BufWriter, Read, StdoutLock},
    path::{Path, PathBuf},
    sync::Arc,
};

use either::*;
use eyre::WrapErr;
use html_to_pdf::{WriteBuilder, WriteBuilderLifetime};

use crate::{compression, find, Result};

////////////////////////////////////////////////////////////////////////////////
// Read (and decompress) input file
////////////////////////////////////////////////////////////////////////////////

/// Indicates what type of compression is used to store a JSON file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JSONCompression {
    Lz4Compression,
    NoCompression,
}
impl JSONCompression {
    pub fn auto_detect_from_path(path: impl AsRef<Path>) -> Self {
        fn inner(path: &Path) -> JSONCompression {
            path.extension()
                .and_then(|ext| ext.to_str().map(|v| v.ends_with("lz4")))
                .map(|is_compressed| {
                    if is_compressed {
                        JSONCompression::Lz4Compression
                    } else {
                        JSONCompression::NoCompression
                    }
                })
                .unwrap_or(JSONCompression::NoCompression)
        }
        inner(path.as_ref())
    }
}

/// Wraps a `Vec<u8>` and implements `Read` for it. Normally you can `Read` from
/// a `&[u8]` but that doesn't work if you need to own the data.
pub struct SliceReader {
    pub data: Vec<u8>,
    pub index: usize,
}
impl SliceReader {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data, index: 0 }
    }
}
impl Read for SliceReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut slice = &self.data[self.index..];
        let value = slice.read(buf)?;
        self.index += value;
        Ok(value)
    }
}
impl From<SliceReader> for Vec<u8> {
    fn from(value: SliceReader) -> Vec<u8> {
        value.data
    }
}
impl From<Vec<u8>> for SliceReader {
    fn from(value: Vec<u8>) -> Self {
        SliceReader::new(value)
    }
}

#[derive(Debug)]
pub enum ReadFirefoxJsonError {
    Decompression(compression::DecoderError),
    Io(io::Error),
    OpenFile(io::Error),
    ReadFile(io::Error),
}
impl fmt::Display for ReadFirefoxJsonError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ReadFirefoxJsonError::Decompression(_) => write!(f, "Failed to decompress JSON data."),
            ReadFirefoxJsonError::Io(_) => write!(f, "Unspecified IO error."),
            ReadFirefoxJsonError::OpenFile(_) => write!(f, "Failed to open the file."),
            ReadFirefoxJsonError::ReadFile(_) => {
                write!(f, "Failed to read the contents of the file to memory.")
            }
        }
    }
}
impl StdError for ReadFirefoxJsonError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            ReadFirefoxJsonError::Decompression(e) => Some(e),
            ReadFirefoxJsonError::Io(e) => Some(e),
            ReadFirefoxJsonError::OpenFile(e) => Some(e),
            ReadFirefoxJsonError::ReadFile(e) => Some(e),
        }
    }
}
impl From<io::Error> for ReadFirefoxJsonError {
    fn from(value: io::Error) -> Self {
        ReadFirefoxJsonError::Io(value)
    }
}
impl From<compression::DecoderError> for ReadFirefoxJsonError {
    fn from(value: compression::DecoderError) -> Self {
        ReadFirefoxJsonError::Decompression(value)
    }
}

/// Decompress lz4 data.
pub fn decompress_lz4_data(mut reader: Either<SliceReader, impl Read>) -> Result<SliceReader> {
    let (buf, index) = if let Left(slice_reader) = reader {
        (slice_reader.data, slice_reader.index)
    } else {
        let mut buf = Vec::new();
        reader
            .read_to_end(&mut buf)
            .map_err(ReadFirefoxJsonError::ReadFile)?;
        (buf, 0)
    };
    let buf_ref = &buf[index..];
    let decompressed = crate::compression::decompress(buf_ref, crate::COMPRESSION_LIBRARY)?;

    Ok(SliceReader::new(decompressed))
}

/// Open a file and create a reader for its content.
///
/// If `cache_file` is true then the file's content will be read to a `Vec` and then the file
/// will be closed immediately. Otherwise the file will be kept open until the reader is closed.
///
/// `compression` determines if the file's content will be decompressed when it is read.
pub fn read_json_file<'a>(
    path: impl AsRef<Path> + 'a,
    cache_file: bool,
    compression: JSONCompression,
) -> Result<Either<SliceReader, impl Read>> {
    let path = path.as_ref();
    let reader = {
        let file = File::open(path).map_err(ReadFirefoxJsonError::OpenFile)?;
        let mut buffer = BufReader::new(file);
        if cache_file {
            let mut data = Vec::new();
            buffer
                .read_to_end(&mut data)
                .map_err(ReadFirefoxJsonError::ReadFile)?;
            Left(SliceReader::new(data))
        } else {
            Right(buffer)
        }
    };

    Ok(if JSONCompression::Lz4Compression == compression {
        Left(decompress_lz4_data(reader)?)
    } else if let Left(slice_reader) = reader {
        Left(slice_reader)
    } else {
        Right(reader)
    })
}

////////////////////////////////////////////////////////////////////////////////
// CLI input helper
////////////////////////////////////////////////////////////////////////////////

pub fn deserialize_from_slice<T>(slice: &[u8]) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    {
        #[cfg(feature = "serde_path_to_error")]
        {
            serde_path_to_error::deserialize(&mut serde_json::Deserializer::from_slice(slice))
        }
        #[cfg(not(feature = "serde_path_to_error"))]
        {
            serde_json::from_slice(slice)
        }
    }
    .map_err(|e| json_parse_error_context(e, slice))
}
pub fn deserialize_from_reader<T, R>(reader: R) -> Result<T>
where
    T: serde::de::DeserializeOwned,
    R: Read,
{
    #[cfg(feature = "serde_path_to_error")]
    {
        Ok(serde_path_to_error::deserialize(
            &mut serde_json::Deserializer::from_reader(reader),
        )?)
    }
    #[cfg(not(feature = "serde_path_to_error"))]
    {
        Ok(serde_json::from_reader(reader)?)
    }
}

pub enum InputReaderState {
    InputPath(PathBuf),
    Stdin(io::Stdin),
}
/// Represents the input of a CLI command.
pub struct InputReader {
    pub state: InputReaderState,
    pub is_compressed: Option<bool>,
}
impl InputReader {
    /// Read the data this input refers to. The data will usually be stored in memory.
    pub fn get_reader(&self) -> Result<Either<SliceReader, impl Read + '_>> {
        Ok(match &self.state {
            InputReaderState::InputPath(path) => {
                match read_json_file(
                    path,
                    // Read ASAP to RAM:
                    true,
                    match self.is_compressed {
                        Some(true) => JSONCompression::Lz4Compression,
                        Some(false) => JSONCompression::NoCompression,
                        None => JSONCompression::auto_detect_from_path(path),
                    },
                )
                .with_context(|| format!("Failed to get data for file at: {:?}.", &path))?
                {
                    Either::Left(v) => Either::Left(v),
                    Either::Right(v) => Either::Right(Either::Left(v)),
                }
            }
            InputReaderState::Stdin(stdin) => {
                let reader = BufReader::new(stdin.lock());

                if self.is_compressed.unwrap_or(false) {
                    Either::Left(
                        decompress_lz4_data(Either::Right(reader))
                            .context("Failed to decompress data from stdin")?,
                    )
                } else {
                    Either::Right(Either::Right(reader))
                }
            }
        })
    }

    /// Buffer the data this input refers to in memory.
    pub fn create_slice_reader(&self) -> Result<SliceReader> {
        let reader = self.get_reader()?;
        match reader {
            Either::Left(v) => Ok(v),
            Either::Right(mut v) => {
                let mut data = Vec::new();
                v.read_to_end(&mut data).with_context(|| {
                    format!(
                        "Failed to read data into memory from {}.",
                        match &self.state {
                            InputReaderState::InputPath(path) => {
                                Cow::from(format!("file at: \"{}\"", path.display()))
                            }
                            InputReaderState::Stdin(_) => Cow::from("stdin"),
                        }
                    )
                })?;
                Ok(SliceReader::new(data))
            }
        }
    }

    /// Load the input's data into memory and decompress it if it was originally
    /// compressed. Returns a tuple with both the original data and the
    /// decompressed data.
    #[expect(clippy::type_complexity)]
    pub fn get_original_data_and_uncompressed_data(&self) -> Result<(Arc<Vec<u8>>, Arc<Vec<u8>>)> {
        match &self.state {
            InputReaderState::InputPath(path) => {
                let mut original = std::fs::read(path)
                    .with_context(|| format!("Failed to read data from file at: {:?}.", &path))?;
                original.shrink_to_fit();
                let original = Arc::new(original);

                let compression = match self.is_compressed {
                    Some(true) => JSONCompression::Lz4Compression,
                    Some(false) => JSONCompression::NoCompression,
                    None => JSONCompression::auto_detect_from_path(path),
                };
                let uncompressed = if matches!(compression, JSONCompression::Lz4Compression) {
                    let mut uncompressed =
                        crate::compression::decompress(&original, crate::COMPRESSION_LIBRARY)
                            .with_context(|| {
                                format!("Failed to decompress data from file at: {:?}.", &path)
                            })?;
                    uncompressed.shrink_to_fit();
                    Arc::new(uncompressed)
                } else {
                    Arc::clone(&original)
                };

                Ok((original, uncompressed))
            }
            InputReaderState::Stdin(stdin) => {
                let data = Arc::new({
                    let mut reader = BufReader::new(stdin.lock());
                    let mut data = Vec::new();
                    reader
                        .read_to_end(&mut data)
                        .context("Failed to read data from stdin")?;
                    data.shrink_to_fit();
                    data
                });
                let uncompressed = if matches!(self.is_compressed, Some(true)) {
                    let mut uncompressed =
                        crate::compression::decompress(&data, crate::COMPRESSION_LIBRARY)
                            .context("Failed to decompress data from stdin")?;
                    uncompressed.shrink_to_fit();
                    Arc::new(uncompressed)
                } else {
                    Arc::clone(&data)
                };
                Ok((data, uncompressed))
            }
        }
    }

    pub fn deserialize_json_data<T>(&self) -> Result<T>
    where
        T: for<'a> serde::de::Deserialize<'a>,
    {
        Ok(match self.get_reader()? {
            // Using a slice reference will allocate less memory than the reader approach, but it will still allocate some.
            Either::Left(slice_reader) => {
                deserialize_from_slice(&slice_reader.data).with_context(|| {
                    format!(
                        "Failed to parse JSON from cached data that was read from {}",
                        self.reader_info()
                    )
                })?
            }
            // Reader will allocate a buffer to keep all of its data in so it will use about twice the memory of the above method if the original data was already stored in memory.
            Either::Right(reader) => deserialize_from_reader(reader).with_context(|| {
                format!(
                    "Failed to parse JSON or read data from {}",
                    self.reader_info()
                )
            })?,
        })
    }

    pub fn path(&self) -> Option<&Path> {
        if let InputReaderState::InputPath(path) = &self.state {
            Some(path)
        } else {
            None
        }
    }
    pub fn file_stem(&self) -> Option<Cow<str>> {
        let path = self.path()?;
        let stem = path.file_stem()?;
        Some(stem.to_string_lossy())
    }

    pub fn reader_info(&self) -> impl fmt::Display + '_ {
        match self.path() {
            Some(v) => Left(format!(r#""{}""#, v.display())),
            None => Right("stdin"),
        }
    }
}

pub fn json_parse_error_context<E>(error: E, data: &[u8]) -> eyre::Report
where
    E: StdError + Send + Sync + 'static,
{
    let json_error: &serde_json::Error = {
        let mut e: &(dyn StdError + 'static) = &error;
        loop {
            if let Some(e) = e.downcast_ref::<serde_json::Error>() {
                break e;
            } else if let Some(s) = e.source() {
                e = s;
            } else {
                return eyre::Report::new(error);
            }
        }
    };

    const WANTED: usize = 200;
    let mut msg = "Error when parsing JSON. Some of the affected text:\n".to_owned();
    let original_msg = msg.len();
    for line in String::from_utf8_lossy(data)
        .lines()
        .skip(json_error.line() - 1)
    {
        let wanted = WANTED + original_msg - msg.len();

        let mut start_index = (json_error.column() as i64) - 1; // 1 is first char and 0 if first char couldn't be read.
        start_index -= (wanted / 2) as i64;
        let start_index = if start_index < 0 {
            0
        } else {
            start_index as usize
        };

        let end_index = start_index + wanted;
        let end_index = if end_index >= line.len() {
            line.len() - 1
        } else {
            end_index
        };

        if let Some(segment) = line.get(start_index..end_index) {
            msg.push_str(segment);
        }
    }
    eyre::Report::new(error).wrap_err(msg)
}

////////////////////////////////////////////////////////////////////////////////
// CLI output helper
////////////////////////////////////////////////////////////////////////////////

/// Represents the output of a CLI command.
pub enum OutputWriter {
    OutputPath { path: PathBuf, overwrite: bool },
    Stdout(io::Stdout),
}
impl OutputWriter {
    pub fn path(&self) -> Option<&Path> {
        if let OutputWriter::OutputPath { path, .. } = &self {
            Some(path)
        } else {
            None
        }
    }

    pub fn output_info(&self) -> impl fmt::Display + '_ {
        self
    }

    pub fn get_writer(&self) -> io::Result<BufWriter<Either<File, StdoutLock<'_>>>> {
        Ok(BufWriter::new(match &self {
            OutputWriter::OutputPath { path, overwrite } => {
                Left(find::create_file(*overwrite, path).map_err(|e| {
                    io::Error::new(
                        e.kind(),
                        format!(
                            "Failed to create an output file at: {{ path: {path:?}, overwrite: {overwrite}, resolved_path: {:?}, }}",
                            path.canonicalize()
                        ),
                    )
                })?)
            }
            OutputWriter::Stdout(stdout) => Right(stdout.lock()),
        }))
    }

    pub fn generic_error_text(&self) -> String {
        format!("Failed to write output data to {}.", self)
    }

    /// Open the output file with the file's default program. If data was written to stdout then this will do nothing.
    ///
    /// Takes mut reference to ensure that the writer reference is dropped before calling this method (can't open a file that is being written to).
    pub fn open_output_file(&mut self) -> Result<()> {
        if let OutputWriter::OutputPath { path, .. } = &self {
            info!(
                r#"Opening the output file at "{}" with its default program"#,
                path.display()
            );
            // TODO: allow waiting for started program to exit, and maybe delete output file when it does.
            // It can be useful to wait if the started program uses stdin and stderr so that the shell doesn't interfere.
            find::open_file(path, false)?;
        }
        Ok(())
    }
}
impl<'a> WriteBuilderLifetime<'a> for OutputWriter {
    type Writer = BufWriter<Either<File, StdoutLock<'a>>>;
}
impl WriteBuilder for OutputWriter {
    fn get_writer(&mut self) -> io::Result<<Self as WriteBuilderLifetime<'_>>::Writer> {
        OutputWriter::get_writer(&*self)
    }
}
impl fmt::Display for OutputWriter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            OutputWriter::OutputPath { path, overwrite } => {
                if *overwrite {
                    write!(f, "an overwritten ")?;
                } else {
                    write!(f, "a ")?;
                }
                write!(f, "file at \"{}\"", path.display())
            }
            OutputWriter::Stdout(_) => write!(f, "stdout"),
        }
    }
}
impl Clone for OutputWriter {
    fn clone(&self) -> Self {
        match self {
            OutputWriter::OutputPath { path, overwrite } => OutputWriter::OutputPath {
                path: path.clone(),
                overwrite: *overwrite,
            },
            OutputWriter::Stdout(_) => OutputWriter::Stdout(io::stdout()),
        }
    }
}
