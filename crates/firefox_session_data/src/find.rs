//! Handle path related work.

use crate::Result;
use color_eyre::Section;
use either::*;
use eyre::{bail, ContextCompat, WrapErr};

use std::borrow::Cow;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::SystemTime;
use std::{io, iter};

/// Get the path to the Firefox profiles directory.
pub fn firefox_profile_dir() -> Result<PathBuf> {
    // Firefox profiles are located at: "C:\Users\%UserName%\AppData\Roaming\Mozilla\Firefox\Profiles" on windows.
    // The environment variable %AppData% will return the path to: "C:\Users\%UserName%\AppData\Roaming".
    let mut app_data = match std::env::var("APPDATA") {
        Ok(v) => PathBuf::from(v),
        Err(_) => {
            let user_name = std::env::var_os("USERNAME")
                .context("Failed to get %APPDATA% or %USERNAME% environment variables.")?;
            #[cfg(target_family = "wasm")]
            {
                // doesn't handle non-UTF8 user names
                PathBuf::from(format!(
                    r"C:\Users\{}\AppData\Roaming",
                    user_name.to_str().context("User name was invalid UTF8")?
                ))
            }

            #[cfg(not(target_family = "wasm"))]
            {
                let mut path = PathBuf::from(r"C:\Users");
                path.push(&user_name);
                path.push(r"AppData\Roaming");
                path
            }
        }
    };
    app_data.push(r"Mozilla\Firefox\Profiles");
    #[cfg(target_family = "wasm")]
    {
        // Only use backslashes to be consistent (otherwise we will fail to "find a pre-opened file descriptor"):
        app_data = PathBuf::from(
            app_data
                .to_str()
                .context("Path is not valid UTF8")?
                .replace('/', "\\"),
        );
    }
    Ok(app_data)
}

pub struct FirefoxProfileFinder {
    pub profile_root: PathBuf,
    profiles: OnceLock<Vec<(PathBuf, io::Result<SystemTime>)>>,
}
impl FirefoxProfileFinder {
    pub fn new() -> Result<Self> {
        Ok(Self {
            profile_root: firefox_profile_dir()
                .context("Failed to get the path to the Firefox profiles directory.")?,
            profiles: OnceLock::new(),
        })
    }
    pub fn all_profiles(&self) -> Result<&[(PathBuf, io::Result<SystemTime>)]> {
        if let Some(v) = self.profiles.get() {
            return Ok(v);
        }
        log::debug!(
            "Finding Firefox profiles at {}",
            self.profile_root.display()
        );
        let profile_paths = self.profile_root
            .read_dir()
            .with_context(|| {
                format!(
                    "Failed to list current Firefox profiles at: \"{}\"",
                    self.profile_root.display()
                )
            })?
            // Skip entry errors (Skips: io::Error):
            .filter_map(|err| {
                match err {
                    Ok(v) => Some(v),
                    Err(e) => { error!("Failed to get info about entry in firefox profiles directory: {}", e); None},
                }
            })
            // Skip all files and any entries that gave io::error when getting file type info:
            .filter(|entry| {
                match entry.file_type() {
                    Ok(info) if info.is_dir() => true,
                    Ok(_) => {
                        log::trace!("Skipping possible Firefox profile because it is not a directory (name: {})", entry.file_name().to_string_lossy());
                        false
                    }
                    Err(e) => {
                        error!(r#"Failed to get file type info for the "{}" entry in the firefox profiles directory: {}"#, entry.path().display(), e);
                        false
                    },
                }
            })
            .map(|entry| {
                let time = entry.metadata().and_then(|data| data.modified());
                if let Err(e) = &time {
                    debug!(r#"Couldn't gather extra error info due to failure to get last modified time for the "{}" entry in the firefox profiles directory: {}"#, entry.path().display(), e);
                }
                (entry.path(), time)
            })
            .collect::<Vec<_>>();

        let _ = self.profiles.set(profile_paths);
        Ok(self.profiles.get().unwrap())
    }
    /// Find a specific Firefox profile. Returns `None` if the specific
    /// profile could not be found. Returns an error if multiple
    /// profiles match the queried name.
    pub fn find_profile(&self, name: &str) -> Result<Option<PathBuf>> {
        if name.contains(['.', '/', '\\']) {
            // Full profile directory name specified:
            let dir = self.profile_root.join(name);
            return Ok(dir.is_dir().then_some(dir));
        }

        let profiles = self.all_profiles()?;

        let mut profile_paths = profiles
            .iter()
            // Get profiles with the correct names:
            .filter(|(entry, _)| {
                log::trace!("Checking profile folder at {}", entry.display());
                entry
                    .file_name()
                    .and_then(|end| Some(end.to_string_lossy().split_once('.')?.1 == name))
                    .unwrap_or(false)
            })
            .peekable();

        let Some(first) = profile_paths.next() else {
            log::debug!(
                "No profile folders ends with {name:?} (possible_profiles: {})",
                profiles.len()
            );
            return Ok(None);
        };

        if profile_paths.peek().is_some() {
            // List possible profiles (with a max count if there are too many):

            let possible_profiles = iter::once(first)
                .chain(&mut profile_paths)
                .take(5)
                .map(|(path, _)| path)
                // Make string that can be displayed:
                .map(path_to_file_name)
                .collect::<Vec<_>>()
                .join("\n");

            let more_count = if profile_paths.peek().is_some() {
                Cow::from(format!("\n...and {} more", profile_paths.count()))
            } else {
                Cow::from("")
            };

            let mut error: Result<_> = Err(eyre::eyre!(
                "More than one Firefox profile was found with the specified name.\n\nPossible profile directories:\n{}{}\n\n",
                possible_profiles,
                more_count
            ));
            if let Some((path, _)) = profiles
                .iter()
                // Ignore profile directories with unknown modification time:
                .filter_map(|(p, time)| Some((p, time.as_ref().ok()?)))
                // Then find the latest modified one:
                .max_by_key(|(_, &time)| time)
            {
                let path = path_to_file_name(path);
                error = error.suggestion(format!(r#"of the found Firefox profiles the "{path}" profile is the latest modified, maybe that is the one you want?"#));
            }
            return error;
        }

        Ok(Some(first.0.clone()))
    }
}

/// Convert a path to a filename. Useful for logging.
pub fn path_to_file_name(path: impl AsRef<Path>) -> String {
    path.as_ref()
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.as_ref().display().to_string())
}

/// Get all files in a folder sorted so that the last modified ones are first.
pub fn get_latest_files_in_dir(
    folder_path: impl AsRef<Path>,
) -> Result<impl Iterator<Item = PathBuf>> {
    let folder_path = folder_path.as_ref();
    Ok(sort_last_modified(
        folder_path
            // Get all files in the current folder together with their modified time.
            .read_dir()
            .with_context(|| {
                format!(
                    "Failed to get contents from directory.\nDirectory path: {}",
                    folder_path.to_string_lossy()
                )
            })?
            // Skip entry errors (Skips: io::Error):
            .filter_map(Result::ok)
            // Skip all folders and any entries that gave io::error when getting file type info:
            .filter(|entry| {
                entry
                    .file_type()
                    .map(|info| info.is_file())
                    .unwrap_or(false)
            }),
    )
    // Get file paths:
    .map(|entry| entry.path()))
}

/// Sort some folder entries so that the last modified ones are first.
pub fn sort_last_modified(
    entries: impl IntoIterator<Item = fs::DirEntry>,
) -> impl Iterator<Item = fs::DirEntry> {
    let mut timed_entries = Vec::new();
    // entries: all files in the current folder together with their modified time.
    for entry in entries.into_iter() {
        // Skip files that gave io::error when getting metadata:
        if let Ok(meta) = entry.metadata() {
            // Skip files that gave io::error when getting modified time:
            if let Ok(modified) = meta.modified() {
                timed_entries.push((modified, entry));
            }
        }
    }

    // Sort newest files first:
    timed_entries.sort_by(|b, a| a.0.cmp(&b.0));

    timed_entries
        .into_iter()
        // Strip away time:
        .map(|(_, entry)| entry)
}

/// Open a file with the file's default program.
///
/// `wait_for_program` indicates if the thread should block until the started program exits.
pub fn open_file(path: impl AsRef<std::ffi::OsStr>, wait_for_program: bool) -> Result<()> {
    let mut command = std::process::Command::new("cmd");
    command.arg("/c").arg(path.as_ref());

    let process = if wait_for_program {
        command.status().map(Left)
    } else {
        command.spawn().map(Right)
    }
    .with_context(|| format!("Failed to open \"{}\".", path.as_ref().to_string_lossy()))?;

    if let Left(status) = process {
        // If we are waiting for the external program then check its error code to see if it was successful:
        if !status.success() {
            bail!(
                "External program exited with {} when opening \"{}\".",
                if let Some(code) = status.code() {
                    Cow::from(format!("the error code {}", code))
                } else {
                    Cow::from("an error")
                },
                path.as_ref().to_string_lossy()
            );
        }
    }
    Ok(())
}

/// Create a new file.
///
/// If `overwrite` is `false` then an atomic operation is used to ensure that no other file existed where the new file is created.
pub fn create_file(overwrite: bool, path: impl AsRef<Path>) -> io::Result<File> {
    // Configure how new files are crated:
    let mut new_file_options = fs::OpenOptions::new();
    new_file_options.write(true);
    if overwrite {
        new_file_options
            // If file exists then remove all of its content:
            .truncate(true)
            // If not file exists then create one:
            .create(true);
    } else {
        // Ensure we don't overwrite anything (atomic operation that ensures that we are creating a new file):
        new_file_options.create_new(true);
    }

    new_file_options.open(path)
}

/// Resolve a path to an unused file path (relative paths will be joined with the current working directory to become absolute paths).
///
/// * If path is `None` or empty then use `default_name`.
/// * If path is a folder (ends with `/` or `\`) then append `default_name`.
/// * If path's file extension is empty then append `default_extension`.
/// * If `overwrite` is `false` then attempt to find an unused path.
///
/// The `default_extension` should be without a leading dot.
pub fn resolve_to_unused_path(
    path: Option<String>,
    overwrite: bool,
    default_name: Cow<str>,
    default_extension: Cow<str>,
) -> Result<String> {
    let mut path_str = if let Some(mut path) = path {
        if path.is_empty() {
            path = default_name.into_owned();
        } else if path.ends_with(['\\', '/']) {
            path.push_str(default_name.as_ref());
        }
        path
    } else {
        default_name.into_owned()
    };

    let mut path = PathBuf::from(&path_str);

    let extension = path.extension().map(std::ffi::OsStr::to_string_lossy);

    if overwrite {
        if extension.is_none() {
            // Ensure file has an extension:
            path_str.push('.');
            path_str.push_str(default_extension.as_ref());
        }

        Ok(path_str)
    } else {
        // Ensure file name doesn't already exist

        let file_name = path
            .file_stem()
            .context("Couldn't get the part of the output path that specified the file name.")?
            .to_string_lossy()
            .into_owned();

        let extension = extension.unwrap_or(default_extension).into_owned();

        path.pop();
        let mut dir_nav = path.to_string_lossy().into_owned().replace('/', "\\");
        if !dir_nav.is_empty() {
            dir_nav.push('\\');
        }

        let dir = std::env::current_dir()
            .context("Couldn't get the current working directory.")?
            .join(path);

        Ok(generate_file_names(dir, move |index| {
            format!(
                "{}{}.{}",
                file_name,
                if index == 0 {
                    "".into()
                } else {
                    format!(" ({})", index)
                },
                extension
            )
        })
        // Check if path is used:
        .filter(|path| !path.exists())
        .filter_map(|allowed_path| {
            allowed_path
                .file_name()
                .map(|v| v.to_string_lossy().into_owned())
        })
        .map(|file_name| {
            let mut dir = dir_nav.clone();
            dir.push_str(&file_name);
            dir
        })
        // Attempt to find an unused path:
        .next()
        .context("Couldn't find an unused path to use for the output file.")?)
    }
}

/// Create an iterator that generates file names.
pub fn generate_file_names<R>(
    dir: impl Into<PathBuf>,
    mut file_generator: impl FnMut(u32) -> R,
) -> impl Iterator<Item = PathBuf>
where
    R: AsRef<Path>,
{
    let dir = dir.into();
    (0..).map(move |current| {
        let file_name = file_generator(current);
        let mut target = dir.clone();
        target.push(file_name);
        target
    })
}
