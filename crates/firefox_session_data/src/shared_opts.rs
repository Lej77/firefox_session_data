//! Command line options that are shared between multiple commands.

use std::{
    borrow::Cow,
    borrow::{Borrow, BorrowMut},
    convert::AsRef,
    io,
    path::{Path, PathBuf},
};

use clap::{ArgAction, Args};
use color_eyre::Help;
use eyre::{bail, ContextCompat, WrapErr};

use crate::{
    find, init_logger,
    io_utils::{InputReader, InputReaderState, OutputWriter},
    try_, verbosity_level, Result,
};

#[derive(Debug, Args, Clone)]
pub struct CommonOpt {
    /// Provide more verbose logging. Can be specified 1 or 2 times to increase
    /// verbosity level.
    #[clap(short, long, action = ArgAction::Count, help_heading = "LOGGING")]
    pub verbose: u8,
    /// Quiet mode, specify twice to suppress all warnings. Specify 3 times to
    /// suppress logging about errors but note that if the program exits with an
    /// error then info about that will still be written to stderr.
    #[clap(
        short,
        long,
        action = ArgAction::Count,
        conflicts_with = "verbose",
        help_heading = "LOGGING"
    )]
    pub quiet: u8,
}
impl CommonOpt {
    /// Enable logging based on specified verbosity arguments.
    pub fn configure_logging(&self) {
        let verbosity_level_number = 3_i64 - (self.quiet as i64) + (self.verbose as i64);
        let verbosity_level = verbosity_level(verbosity_level_number.max(0) as u64);
        init_logger(verbosity_level);

        if verbosity_level_number < 0 {
            warn!(
                "Specified logging level {} but 0 is the lowest level",
                verbosity_level_number
            )
        }
        if verbosity_level_number > 5 {
            warn!(
                "Specified logging level {} but 5 is the highest level",
                verbosity_level_number
            )
        }
        info!(
            "Logging with verbosity level: {} - {}",
            verbosity_level_number.min(5),
            verbosity_level
                .map(|level| level.to_string())
                .unwrap_or_else(|| "Off".to_string())
        );
    }
}

/// Options needed to read a sessionstore file and generate an output file.
#[derive(Debug, Args, Clone)]
#[clap(rename_all = "kebab-case")]
pub struct SessionstoreOpt {
    #[clap(flatten)]
    pub in_out_info: InOutOpt,

    #[clap(flatten)]
    pub compression: CompressInfoOpt,
}
impl SessionstoreOpt {
    pub fn get_reader_creator(&self) -> Result<InputReader> {
        self.in_out_info.get_reader_creator(
            self.compression.input_is_compressed(),
            &["jsonlz4".into(), "js".into()],
        )
    }
}

/// Extra options for when an input file might be compressed.
#[derive(Debug, Args, Clone, Default)]
#[clap(rename_all = "kebab-case")]
pub struct CompressInfoOpt {
    /// Indicate that the input file is compressed. If the file extension ends
    /// with "lz4" this is automatically detected.
    #[clap(short, long, help_heading = "INPUT")]
    pub compressed: bool,

    /// Indicates that the input file is uncompressed.
    #[clap(short, long, conflicts_with = "compressed", help_heading = "INPUT")]
    pub uncompressed: bool,
}
impl CompressInfoOpt {
    pub fn input_is_compressed(&self) -> Option<bool> {
        if self.compressed {
            Some(true)
        } else if self.uncompressed {
            Some(false)
        } else {
            None
        }
    }
}

/// Option to overwrite the input file.
#[derive(Debug, Args, Clone)]
#[clap(rename_all = "kebab-case")]
pub struct OverwriteInputOpt {
    #[clap(
        long,
        visible_aliases = &["owi", "in-place", "inplace", "replace-input"],
        conflicts_with_all = &["stdin", "open", "output", "overwrite"],
        help_heading = "OUTPUT"
    )]
    /// Overwrite the input file's content with the output data.
    pub overwrite_input: bool,

    #[clap(
        long,
        visible_aliases = &["swap-input-and-output"],
        conflicts_with_all = &["overwrite-input", "stdin"],
        help_heading = "OUTPUT"
    )]
    /// Overwrite the input file with output content and write the input file's
    /// original content to the output file.
    pub swap: bool,
}

/// Options to select an input file that is a firefox sessionstore file and also
/// options to select an output location.
#[derive(Debug, Args, Clone)]
#[clap(rename_all = "kebab-case")]
pub struct InOutOpt {
    #[clap(flatten)]
    pub common: CommonOpt,

    /// Path to the input file. If not provided then attempts to find the last
    /// modified file with the correct file extension. If the path ends with
    /// "\" or "/" then attempts to find the last modified file in the specified
    /// directory.
    #[clap(short, long, value_parser, help_heading = "INPUT")]
    pub input: Option<PathBuf>,

    /// Firefox profile name. Specify this to make input paths relative to a
    /// Firefox profile directory instead of the current working directory.
    ///
    /// This can be something like "default" or "wscs2ifj.default". If the
    /// provided profile name doesn't include a dot then anything before the
    /// first dot in the Firefox profile directories' names will be ignored.
    ///
    /// If the input file can't be found directly in the Firefox profile's
    /// directory then the input path will also be checked relative to the
    /// "\sessionstore-backups" sub directory inside the Firefox profile
    /// directory.
    ///
    /// If no input file is specified then this will check for the
    /// "sessionstore" file and then the "sessionstore-backups/recovery" file
    /// instead of the latest modified file with the correct file extension
    /// which is what is usually used.
    ///
    /// This option can be specified multiple times or multiple profiles can be
    /// separated by commas (,) in which case the first existing profile will be
    /// used.
    #[clap(
        short,
        long,
        action = ArgAction::Append,
        use_value_delimiter = true,
        help_heading = "INPUT"
    )]
    pub firefox_profile: Vec<String>,

    /// Read input from stdin instead of from a file.
    #[clap(
        long,
        visible_alias = "si",
        conflicts_with = "input",
        conflicts_with = "firefox_profile",
        help_heading = "INPUT"
    )]
    pub stdin: bool,

    /// Path to the output file. If not provided then guess from the input path
    /// or if that isn't provided then use a default name and place the file in
    /// the current working directory.
    #[clap(short, long, value_parser, help_heading = "OUTPUT")]
    pub output: Option<PathBuf>,

    /// Overwrite the output file if necessary.
    #[clap(long, visible_alias = "ow", help_heading = "OUTPUT")]
    pub overwrite: bool,

    /// Write the output to stdout instead of a file.
    #[clap(
        long,
        visible_alias = "so",
        conflicts_with_all = &["output", "overwrite"],
        help_heading = "OUTPUT"
    )]
    pub stdout: bool,

    /// Open the output file.
    #[clap(long, conflicts_with = "stdout", help_heading = "OUTPUT")]
    pub open: bool,
}
impl InOutOpt {
    fn get_latest_modified_file_in_dir(
        dir_path: impl AsRef<Path>,
        file_extensions: &[Cow<'static, str>],
    ) -> Result<PathBuf> {
        let allowed_extensions_info = {
            let mut all = file_extensions
                .iter()
                .map(|ext| format!(r#""{}""#, ext))
                .collect::<Vec<_>>();
            if let Some(last) = all.pop() {
                format!("{} or {}", all.join(", "), last)
            } else {
                String::new()
            }
        };
        info!(
            r#"Searching for the latest modified file with a {} extension in "{}" to use as input file"#,
            allowed_extensions_info,
            dir_path.as_ref().display()
        );
        find::get_latest_files_in_dir(dir_path.as_ref())
            .with_context(|| {
                format!(
                    "Failed to get the latest modified files in: \"{}\".",
                    dir_path.as_ref().display()
                )
            })?
            .find(|path| {
                let allowed = path
                    .extension()
                    .map(|ext| file_extensions.iter().any(|allowed| &**allowed == ext))
                    .unwrap_or(false);
                trace!(
                    r#"skipping the file at "{}" because of its file extension"#,
                    path.display()
                );
                allowed
            })
            .with_context(|| {
                format!(
                    "Failed to find a file with a {} extension in: \"{}\".",
                    allowed_extensions_info,
                    dir_path.as_ref().display()
                )
            })
    }

    /// Get the path that an input path argument specifies given a specific
    /// current directory. Will return `None` if the input path is empty.
    ///
    /// The second value in the tuple specifies if the input path is a directory.
    /// Otherwise the path is for a file.
    fn resolve_input_path(&self, current_dir: impl AsRef<Path>) -> (Option<PathBuf>, bool) {
        let current_dir = current_dir.as_ref();
        let input = match &self.input {
            Some(v) => v,
            None => return (None, true),
        };

        let is_dir = {
            let input_str = input.to_string_lossy();
            input_str.ends_with('\\') || input_str.ends_with('/')
        };

        let input = current_dir.join(input);
        if is_dir || current_dir == input {
            // Use current_dir
            (None, true)
        } else {
            (Some(input), is_dir)
        }
    }

    /// Resolve an input path. Returns `None` if stdin should be used.
    ///
    /// `file_extensions` is the file extensions that should be used for the
    /// default file. ("jsonlz4" for compressed files and "js" for uncompressed
    /// files.)
    pub fn get_input_path(&self, file_extensions: &[Cow<'static, str>]) -> Result<Option<PathBuf>> {
        if self.stdin {
            trace!("Use stdin as input source");
            return Ok(None);
        }
        if self.firefox_profile.is_empty() {
            // Input path is relative to the current working directory.
            trace!("Finding input source relative to the current working directory.");

            let current_dir =
                std::env::current_dir().context("Failed to get the current working directory.")?;

            let (path, is_dir) = self.resolve_input_path(&current_dir);

            if !is_dir {
                if let Some(path) = path {
                    return Ok(Some(path));
                }
            }

            // We should find the latest modified file in the specified directory.
            return Self::get_latest_modified_file_in_dir(
                path.unwrap_or(current_dir),
                file_extensions,
            )
            .map(Some);
        }

        trace!("Finding input source in a Firefox profile directory");
        let all_firefox_names = self
            .firefox_profile
            .iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ");

        let mut result = try_!({
            // Input path is relative to a Firefox profile directory.
            let finder = find::FirefoxProfileFinder::new()?;
            info!(r#"Searching for one of the Firefox profiles {all_firefox_names} in "{}""#, finder.profile_root.display());

            // Find the correct Firefox profile:
            let profile_dir = self.firefox_profile
                .iter()
                // Ignore names that could not be found (but not errors)
                .find_map(|name| finder.find_profile(name).transpose())
                .with_context(|| format!("Failed to find one of specified Firefox profile directories: {all_firefox_names}"))??;

            // Find the specified input file inside the Firefox profile:
            let backup_dir_name = "sessionstore-backups";

            let (path, is_dir) = self.resolve_input_path(&profile_dir);

            if let Some(path) = path {
                if is_dir {
                    Self::get_latest_modified_file_in_dir(path, file_extensions)
                        .map(Some)?
                } else if path.is_file() {
                    Some(path)
                } else {
                    info!(
                        r#"The input file "{}" couldn't be found in the root of the firefox profile's directory so searching in the profile's backup sub-folder ("/{}")."#,
                        path.display(),
                        backup_dir_name
                    );
                    // Try to find the file in the sessionstore backup directory:
                    let backup_dir = profile_dir.join(backup_dir_name);
                    let path = self.resolve_input_path(backup_dir).0.unwrap();
                    if !path.is_file() {
                        bail!("Failed to find input file at: \"{}\"", path.display());
                    }
                    Some(path)
                }
            } else {
                info!("No input path was specified so checking default sessionstore file names");
                let backup_dir = profile_dir.join(backup_dir_name);

                for extension in file_extensions.iter() {
                    // Check if `sessionstore.` exists.

                    let mut path = profile_dir.join("sessionstore");
                    path.set_extension(&**extension);
                    info!(r#"Checking for input file at: "{}""#, path.display());
                    if path.is_file() {
                        return Ok(Some(path));
                    }

                    let mut path = backup_dir.join("recovery");
                    path.set_extension(&**extension);
                    info!(r#"Checking for input file at: "{}""#, path.display());
                    if path.is_file() {
                        return Ok(Some(path));
                    }
                }
                bail!(
                    "Failed to find an input file for the Firefox profile at: \"{}\"",
                    profile_dir.display()
                );
            }
        })
        .with_context(|| {
            format!(
                r#"Failed to find an input file for the Firefox profile with one of the names: {all_firefox_names}."#,
            )
        });

        if result.is_err() {
            // Suggest using the latest modified Firefox profile:
            match try_!({
                let mut latest = None;
                for entry in find::firefox_profile_dir()?.read_dir()? {
                    match try_!(io::Error, {
                        let entry = entry?;
                        let time = entry.metadata()?.modified()?;
                        if let Some((_, latest_time)) = latest {
                            if latest_time > time {
                                return Ok(());
                            }
                        }
                        latest = Some((entry, time));
                    }) {
                        Ok(()) => {}
                        Err(e) => {
                            debug!("Couldn't gather extra error info: no info about directory entry in firefox profile (ignoring entry): {}", e);
                        }
                    }
                }
                latest
            }) {
                Ok(Some((latest_entry, _))) => {
                    let file_name = find::path_to_file_name(latest_entry.path());
                    result = result.suggestion(format!(r#"of all Firefox profiles the "{}" profile is the latest modified, maybe that is the one you want?"#, file_name));
                }
                Ok(None) => {}
                Err(e) => debug!(r#"Couldn't gather extra error info: {}"#, e),
            }
        }
        result
    }

    /// `input_is_compressed` indicates if the input data is compressed, if it
    /// is then it will be decompressed. Specify `None` to auto detect compression
    /// from file extension.
    pub fn get_reader_creator(
        &self,
        input_is_compressed: Option<bool>,
        file_extensions: &[Cow<'static, str>],
    ) -> Result<InputReader> {
        trace!("Determining input source");
        let state = if let Some(input_path) = self
            .get_input_path(file_extensions)
            .context("Failed to find input path.")?
        {
            info!(r#"Reading input from file at: "{}""#, input_path.display());
            InputReaderState::InputPath(input_path)
        } else {
            info!("Reading input data from stdin");
            InputReaderState::Stdin(io::stdin())
        };

        Ok(InputReader {
            state,
            is_compressed: input_is_compressed,
        })
    }

    pub fn get_writer_creator<'a>(
        &self,
        default_name: impl Into<Cow<'a, str>>,
        default_extension: impl Into<Cow<'a, str>>,
    ) -> Result<OutputWriter> {
        let default_name = default_name.into();
        let default_extension = default_extension.into();
        trace!(
            r#"Determining output location using the default name "{}" and the default file extension "{}" "#,
            default_name,
            default_extension
        );
        if self.stdout {
            trace!("Writing to stdout");
            Ok(OutputWriter::Stdout(io::stdout()))
        } else {
            trace!("Resolving output path.");
            Ok(OutputWriter::OutputPath {
                path: find::resolve_to_unused_path(
                    self.output
                        .as_ref()
                        .map(|v| v.to_string_lossy().into_owned()),
                    self.overwrite,
                    default_name,
                    default_extension,
                )
                .with_context(|| {
                    format!(
                        "Failed to resolve output path from: {{ path: {:?}, overwrite: {} }}",
                        self.output, self.overwrite
                    )
                })?
                .into(),
                overwrite: self.overwrite,
            })
        }
    }

    /// Wraps [`get_writer_creator`] but first tries to use the input's filename
    /// without extension to determine the output path.
    pub fn get_writer_creator_from_reader_creator<'a>(
        &self,
        reader_creator: &InputReader,
        default_name: impl Into<Cow<'a, str>>,
        separator: impl Borrow<&'a str>,
        post_fix: impl Borrow<&'a str>,
        default_extension: impl Into<Cow<'a, str>>,
    ) -> Result<OutputWriter> {
        let default_extension = default_extension.into();

        let mut input_stem = reader_creator
            .file_stem()
            .unwrap_or_else(|| default_name.into())
            .into_owned();
        if !input_stem.is_empty() {
            input_stem.push_str(separator.borrow());
        }
        input_stem.push_str(post_fix.borrow());
        if !default_extension.is_empty() {
            input_stem.push('.');
            input_stem.push_str(default_extension.as_ref());
        }

        trace!(
            r#"Determining output file path using the default path "{}" derived from the input path"#,
            input_stem
        );

        self.get_writer_creator(input_stem, default_extension)
    }

    pub fn handle_output(&self, mut writer_creator: impl BorrowMut<OutputWriter>) -> Result<()> {
        if self.open {
            // TODO: Allow deleting the output file after a certain time has passed or when the started external program exits.
            writer_creator.borrow_mut().open_output_file()
        } else {
            Ok(())
        }
    }
}
