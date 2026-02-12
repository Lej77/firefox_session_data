#![warn(clippy::all)]

#[macro_use]
extern crate log;

pub mod find;
pub mod io_utils;
pub mod pdf_converter;
pub mod shared_opts;
pub mod to_links;
#[cfg(feature = "typst_pdf")]
pub mod typst_world;

pub use firefox_compression as compression;
pub use firefox_session_store as session_store;
use io_utils::InputReader;

pub type Result<T = (), E = Error> = core::result::Result<T, E>;
pub type Error = eyre::Report;

use std::{
    cmp::Reverse,
    collections::HashMap,
    ffi::OsString,
    fs::OpenOptions,
    io::{self, BufRead, BufReader, BufWriter, Read, Write},
    process::{Command, Stdio},
    sync::Arc,
    thread,
    time::Instant,
};

use clap::{Args, Parser};
use color_eyre::Help;
use either::*;
use eyre::WrapErr;
use html_to_pdf::{HtmlSink, HtmlToPdfConverter};
use json_statistics::{collect_statistics, type_script::TypeScriptStatisticsFormatter};

use shared_opts::{CommonOpt, InOutOpt, OverwriteInputOpt, SessionstoreOpt};

/// The compression library that should be used.
const COMPRESSION_LIBRARY: compression::SupportedCompressionLibrary = {
    #[cfg(not(target_family = "wasm"))]
    {
        compression::SupportedCompressionLibrary::Lz4
    }
    #[cfg(target_family = "wasm")]
    'find_lib: {
        let all = compression::CompressionLibrary::get_all();

        let mut i = 0;
        while i < all.len() {
            if !all[i].panic_on_compress() {
                if let Some(lib) = all[i].try_into_supported() {
                    break 'find_lib lib;
                }
            }
            i += 1;
        }

        let mut i = 0;
        while i < all.len() {
            if let Some(lib) = all[i].try_into_supported() {
                break 'find_lib lib;
            }
            i += 1;
        }
        panic!("No compression library was enabled");
    }
};

/// UTF 8 Byte Order Mark. Write to the beginning of a text file to indicate the text encoding of the data.
#[expect(
    dead_code,
    reason = "we are going to write this for some text output formats in the future"
)]
const UTF_8_BOM: &[u8] = b"\xEF\xBB\xBF";

/// Catch any errors inside the block.
///
/// Inspired by how the unstable `try` blocks work:
/// https://doc.rust-lang.org/beta/unstable-book/language-features/try-blocks.html
macro_rules! try_ {
    // Specify error type:
    ($error:ty, $block:expr) => {
        (|| -> Result<_, $error> { Ok($block) })()
    };
    // Use default error type (the current scope should use a Result alias):
    ($block:expr) => {
        (|| -> Result<_> { Ok($block) })()
    };
}
use try_;

use crate::io_utils::{deserialize_from_slice, json_parse_error_context};

/// Helps with managing Firefox session store files.
#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case")]
#[clap(version, author, about)]
pub enum Opt {
    /// Prints information about the contents of a JSON file. The file can be
    /// compressed using firefox `mozLz4` format.
    ///
    /// To create a TypeScript file that exports a JsonData type with info
    /// about the sessionstore's JSON data use:
    ///
    /// firefox-session-data analyze-json --firefox-profile=default-release --type-script
    #[clap(version, author)]
    #[clap(visible_alias = "a")]
    AnalyzeJson {
        #[clap(long, visible_alias = "ts")]
        /// Emit a TypeScript type describing the analyzed JSON.
        type_script: bool,

        #[clap(
            long,
            visible_alias = "max-keys",
            requires = "type-script",
            default_value = "40"
        )]
        /// Max keys inside an object before no specific keys are shown.
        max_object_keys: u32,

        #[clap(flatten)]
        session: SessionstoreOpt,
    },

    /// Compress a file using Firefox's `mozLz4` format. Uncompressed session
    /// store files usually have the `.js` file extensions.
    #[clap(version, author)]
    #[clap(visible_alias = "c")]
    Compress(InOutOpt),

    /// Decompress a file that is using Firefox's `mozLz4` format. Compressed
    /// session store files usually have the `.jsonlz4` file extensions.
    #[clap(version, author)]
    #[clap(visible_alias = "d")]
    Decompress(InOutOpt),

    /// Copy a sessionstore file to an output location.
    ///
    /// Allows making use of this program's ability to find sessionstore files.
    #[clap(version, author)]
    Copy(SessionstoreOpt),

    /// Remove tabs that are marked via a special Firefox extension from a
    /// sessionstore file.
    #[clap(version, author)]
    #[clap(visible_alias = "rmt")]
    RemoveMarkedTabs {
        #[clap(flatten)]
        remove_options: RemoveMarkedTabsOptions,

        #[clap(flatten)]
        overwrite_input: OverwriteInputOpt,

        #[clap(flatten)]
        session: SessionstoreOpt,
    },

    /// Remove tree data kept by a specific browser extension.
    ///
    /// This can help when switching between different extensions that use tree
    /// data by clearing any old data left over by the wanted extension before
    /// installing it again. Then it should reload all the tree data from the
    /// currently installed extension.
    ///
    /// 1. Remove any old tree data for the wanted extension that might have
    ///    been left over from when you previously used it.
    ///
    /// 2. Install the new tree extension that you want to use.
    ///
    /// 3. Uninstall the tree extension that was used previously.
    #[clap(version, author)]
    #[clap(visible_alias = "rtd")]
    RemoveTreeData {
        #[clap(flatten)]
        remove_options: RemoveTreeDataOptions,

        #[clap(flatten)]
        overwrite_input: OverwriteInputOpt,

        #[clap(flatten)]
        session: SessionstoreOpt,
    },

    /// Modify a Firefox sessionstore file using another program/command
    ///
    /// For example, to modify the sessionstore of the Firefox profile
    /// `default-release` using JavaScript executed by the Deno runtime you can
    /// use the following invocation:
    ///
    /// firefox-session-data modify --firefox-profile=default-release --overwrite-input -- deno run modify-sessionstore.ts
    ///
    /// Which will run a JavaScript file named "modify-sessionstore.ts" which
    /// could look like:
    ///
    /// import type { JsonData } from './sessionstore-type.ts';
    /// const json: JsonData = await new Response(Deno.stdin.readable).json();
    /// // Insert some code here, can print using console.error() if needed
    /// console.log(JSON.stringify(json));
    ///
    /// The "sessionstore-type.ts" file can be created using:
    ///
    /// firefox-session-data analyze-json --firefox-profile=default-release --type-script --output "sessionstore-type.ts"
    ///
    /// The types are of course optional and can easily be skipped but they can
    /// be quite helpful by providing auto completion when writing the script.
    ///
    /// (If you care about backups then take a look at the "--swap" option.)
    #[clap(version, author, verbatim_doc_comment)]
    #[clap(visible_alias = "m")]
    Modify {
        #[clap(
            long,
            action = clap::ArgAction::Append,
            use_value_delimiter = true,
            help_heading = "MODIFY"
        )]
        /// If the command exits with this error code then don't report an error
        /// but still don't emit any output.
        stop_exit_code: Option<i64>,

        #[clap(required = true, help_heading = "MODIFY")]
        /// The command to run that will modify the sessionstore's JSON content.
        ///
        /// The first argument will be the program to run, the rest will be
        /// given as arguments to that program.
        ///
        /// The started program can read the original sessionstore JSON from its
        /// stdin and write the new content to stdout. If it exits with a
        /// non-zero exit code then the new content won't be written anywhere.
        /// If the error code isn't listed using `--stop-exit-code` then this
        /// program will also exit with a non-zero exit code.
        command: Vec<OsString>,

        #[clap(
            long,
            visible_aliases = &["skip-json-verify"],
            help_heading = "MODIFY"
        )]
        /// Don't verify the JSON before passing it to the command and after the
        /// command has printed JSON to its stdout.
        skip_json_verification: bool,

        #[clap(flatten)]
        overwrite_input: OverwriteInputOpt,

        #[clap(flatten)]
        session: SessionstoreOpt,
    },

    /// Get a list of tab groups from a sessionstore.
    #[clap(version, author)]
    #[clap(visible_alias = "gg")]
    GetGroups {
        #[clap(flatten)]
        session: SessionstoreOpt,

        #[clap(flatten)]
        tab_group_options: to_links::TabGroupOptions,

        /// Output the information as JSON.
        #[clap(long)]
        json: bool,
    },

    /// Get URLs for tabs in a sessionstore file.
    #[clap(version, author)]
    #[clap(visible_alias = "ttl")]
    TabsToLinks(to_links::TabsToLinksOpt),

    /// Analyze the domains of a session's open tabs.
    #[clap(version, author)]
    Domains(SessionstoreOpt),

    /// Print info about the different output formats that are supported by the
    /// `tabs-to-links` command.
    #[clap(version, author)]
    #[clap(visible_alias = "ttlf")]
    TabsToLinksFormats {
        /// Output the information as JSON.
        #[clap(long)]
        json: bool,
    },
}
impl Opt {
    pub fn common(&self) -> &CommonOpt {
        match self {
            Opt::AnalyzeJson { session, .. } => &session.in_out_info.common,
            Opt::Copy(opt) => &opt.in_out_info.common,
            Opt::Compress(opt) => &opt.common,
            Opt::Decompress(opt) => &opt.common,
            Opt::RemoveMarkedTabs { session, .. } => &session.in_out_info.common,
            Opt::RemoveTreeData { session, .. } => &session.in_out_info.common,
            Opt::Modify { session, .. } => &session.in_out_info.common,
            Opt::GetGroups { session, .. } => &session.in_out_info.common,
            Opt::TabsToLinks(opt) => &opt.session_store_opt.in_out_info.common,
            Opt::Domains(opt) => &opt.in_out_info.common,
            Opt::TabsToLinksFormats { .. } => panic!("this command doesn't have any arguments"),
        }
    }
}

/// Specify what type of extension stored the tree data that should be removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum RemovableTreeData {
    /// The modern Tree Style Tab web extension's data.
    Tst,
    /// The tree data from the old Tree Style Tab addon, the one from before
    /// Firefox had WebExtensions.
    TstLegacy,
    /// The tree data from Sidebery.
    Sidebery,
}

#[derive(Debug, Args, Clone, Default)]
#[clap(rename_all = "kebab-case")]
pub struct RemoveTreeDataOptions {
    #[clap(
        long,
        value_enum,
        action = clap::ArgAction::Append,
        use_value_delimiter = true,
        required_unless_present = "all",
        conflicts_with = "all",
        help_heading = "Remove Tree Data"
    )]
    /// Specifies the extensions to remove tree data from, for example Tree
    /// Style Tab.
    ///
    /// Multiple extensions can be specified by separating them with
    /// commas (,). The tree data for all listed extensions will be removed.
    pub addon: Vec<RemovableTreeData>,
    #[clap(long, help_heading = "Remove Tree Data")]
    /// Remove tree data from all extensions that this program knows about.
    pub all: bool,
}

/// Modify Firefox session data so that tree data for specific extensions are
/// cleared/removed.
///
/// The `session_data` argument should be the complete JSON structure that is
/// deserialized from the sessionstore file.
pub fn remove_tree_data(
    session_data: &mut serde_json::Value,
    options: &RemoveTreeDataOptions,
) -> Result<()> {
    let mut total_remove_count = 0;
    let session = session_store::serde_unstructured::view(session_data)
        .cast::<session_store::FirefoxSessionStore>();

    #[derive(Debug, Default)]
    struct DataToClear {
        tst_legacy: bool,
        tst_modern: bool,
        sidebery: bool,
    }
    impl std::ops::Index<RemovableTreeData> for DataToClear {
        type Output = bool;

        fn index(&self, index: RemovableTreeData) -> &Self::Output {
            match index {
                RemovableTreeData::Tst => &self.tst_modern,
                RemovableTreeData::TstLegacy => &self.tst_legacy,
                RemovableTreeData::Sidebery => &self.sidebery,
            }
        }
    }
    impl std::ops::IndexMut<RemovableTreeData> for DataToClear {
        fn index_mut(&mut self, index: RemovableTreeData) -> &mut Self::Output {
            match index {
                RemovableTreeData::Tst => &mut self.tst_modern,
                RemovableTreeData::TstLegacy => &mut self.tst_legacy,
                RemovableTreeData::Sidebery => &mut self.sidebery,
            }
        }
    }
    let data_to_clear = if options.all {
        DataToClear {
            tst_legacy: true,
            tst_modern: true,
            sidebery: true,
        }
    } else {
        let mut data_to_clear = DataToClear::default();
        for &addon in &options.addon {
            data_to_clear[addon] = true;
        }
        data_to_clear
    };

    debug!("Removing tree data from the following extensions: {data_to_clear:?}");

    let windows = session.project(|p| p.windows())?;
    for mut window in windows.try_array_iter()? {
        let window_result = (|| -> Result<_> {
            let mut window_remove_count = 0;

            let tabs = window.as_mut().project(|p| p.tabs())?;

            for tab in tabs.try_array_iter()? {
                let Ok(ext_data) = tab.project(|p| p.ext_data()) else {
                    // No ext data:
                    continue;
                };
                let Some(ext_data) = ext_data.data.as_object_mut() else {
                    // Ext data was not an object.
                    warn!(
                        "A tab's ext_data was not an object (tab was skipped): {}",
                        ext_data.tracker
                    );
                    continue;
                };

                let mut was_affected = false;

                ext_data.retain(|k, _| {
                    let remove = (data_to_clear[RemovableTreeData::TstLegacy]
                        && k.starts_with("treestyletab_"))
                        || (data_to_clear[RemovableTreeData::Tst]
                            && k.starts_with("extension:treestyletab@piro.sakura.ne.jp"))
                        || (data_to_clear[RemovableTreeData::Sidebery]
                            && k.starts_with("extension:{3c078156-979c-498b-8990-85f7987dd929}"));
                    if remove {
                        was_affected = true;
                    }
                    !remove
                });

                if was_affected {
                    window_remove_count += 1;
                }
            }

            total_remove_count += window_remove_count;
            Ok(())
        })();
        if let Err(e) = window_result {
            warn!(
                "failed to remove tree data from a window: {e} (affected json data: {})",
                window.tracker
            );
        }
    }

    info!(
        "Removed tree data from {} tabs in the sessionstore file",
        total_remove_count
    );

    Ok(())
}

#[derive(Debug, Args, Clone, Default)]
#[clap(rename_all = "kebab-case")]
pub struct RemoveMarkedTabsOptions {
    #[clap(
        long,
        action = clap::ArgAction::Append,
        use_value_delimiter = true,
        help_heading = "Remove Marked Tabs"
    )]
    /// Remove tabs that are marked with a specific color in the extension
    /// Sidebery. For example: "red".
    ///
    /// Multiple values can be specified by separating them with commas (,)
    /// in which case a tab will be removed if it is marked with any of the
    /// colors.
    sidebery_colors: Vec<String>,
}

/// Modify Firefox session data so that marked tabs are removed.
///
/// The `session_data` argument should be the complete JSON structure that
/// is deserialized from the sessionstore file.
pub fn remove_marked_tabs(
    session_data: &mut serde_json::Value,
    options: &RemoveMarkedTabsOptions,
) -> Result<()> {
    let mut total_remove_count = 0;
    let session = session_store::serde_unstructured::view(session_data)
        .cast::<session_store::FirefoxSessionStore>();

    let windows = session.project(|p| p.windows())?;
    for window in windows.try_array_iter()? {
        let window_result = (|| -> Result<_> {
            let mut window_remove_count = 0;

            let (tabs, selected) = window.project(|p| (p.tabs(), p.selected()));
            let tabs = tabs?;
            let mut selected_tab = try_!({
                let selected = selected?;
                let value = selected.as_ref().deserialize()?;
                (selected, value)
            })
            .map_err(|e| {
                error!(
                    "could not get selected tab info for a window, \
                    so can't update it if any tabs are removed: {e}"
                );
            })
            .ok();

            // Remove unwanted tabs from the array:
            let mut idx = 0;
            tabs.try_retain(|tab| {
                // Deserialize the tab to get structured access to its data:
                let keep_tab = match tab.as_ref().deserialize() {
                    Ok(structured_tab) => {
                        let removed_sidebery_color = matches!(
                            &structured_tab.ext_data.sidebery_data,
                            Some(data) if matches!(&data.custom_color,
                                Some(color) if options.sidebery_colors.contains(color)
                            )
                        );

                        if removed_sidebery_color
                            || structured_tab.ext_data.marked_for_removal.is_some()
                        {
                            let info = session_store::session_info::TabInfo::new(&structured_tab);
                            trace!(
                                r#"Removing tab with title "{}" and the URL "{}""#,
                                info.title(),
                                info.url()
                            );
                            window_remove_count += 1;
                            false
                        } else {
                            // Not marked:
                            true
                        }
                    }
                    Err(e) => {
                        error!("Failed to deserialize tab data (tab was skipped): {}", e);
                        true
                    }
                };

                // Ensure active tab index is updated so that the active tab remains
                // selected after we have removed the marked tabs.
                if let Some((_, selected_tab)) = &mut selected_tab {
                    // If old selected index == current tab
                    if *selected_tab == idx + 1 {
                        // Decrement selected_tab with the number of removed tabs.
                        // If the selected tab was also removed then the next tab
                        // will be selected.
                        let new_tab = idx.saturating_sub(window_remove_count) + 1;
                        debug!(
                            "Changed selected tab index from {} to {}.",
                            *selected_tab, new_tab
                        );
                        *selected_tab = new_tab;
                    }
                }
                idx += 1;

                keep_tab
            })?;

            total_remove_count += window_remove_count;

            if window_remove_count > 0 {
                if let Some((slot, selected_tab)) = selected_tab {
                    // Replace old selected tab value with the updated one.
                    *slot.data = selected_tab.into();
                }
            }
            Ok(())
        })();
        if let Err(e) = window_result {
            warn!("failed to remove marked tabs from a window: {e}");
        }
    }

    info!(
        "Removed {} tabs from the sessionstore file",
        total_remove_count
    );

    Ok(())
}

pub fn tabs_to_links<W>(
    groups: &[session_store::session_info::TabGroup<'_>],
    mut options: to_links::TabsToLinksOutput,
    mut writer_creator: W,
) -> Result<()>
where
    W: html_to_pdf::WriteBuilder + Send,
{
    thread::scope(|s| -> Result<_> {
        trace!("Conversion options:\n{:#?}\n", options.conversion_options);

        let mut writer = if let Some(pdf_mode) = options.as_pdf {
            Left(
                pdf_converter::SupportedPdfConversion {
                    method: pdf_mode,
                    link_options: &mut options.conversion_options,
                }
                .start(html_to_pdf::PdfScope::scoped(s), &mut writer_creator)?,
            )
        } else {
            Right(writer_creator.get_writer()?)
        };

        // TODO: only write utf8 BOM for some file formats (maybe only for Text or Markdown?).
        // writer.write_all(UTF_8_BOM).context("Failed to write UTF8 Byte Order Mark.")?;

        options
            .conversion_options
            .write_links(groups, &mut writer)?;

        if let Left(pdf_writer) = writer {
            pdf_writer.complete().context("PDF conversion failed")?;
        }

        Ok(())
    })
}

fn modify_sessionstore(
    session_opt: &SessionstoreOpt,
    overwrite_opt: &OverwriteInputOpt,
    output_postfix: &str,
    modify: impl FnOnce(Arc<Vec<u8>>, &InputReader) -> Result<Vec<u8>>,
) -> Result<()> {
    let reader_creator = session_opt.get_reader_creator()?;
    let mut input_data;
    let mut encoder = {
        let modified_json_data = {
            info!("Reading data from {}", reader_creator.reader_info());

            // Store data in Arc so we can drop it ASAP when not using "--swap" flag.
            let (original, decompressed) =
                reader_creator.get_original_data_and_uncompressed_data()?;
            input_data = overwrite_opt.swap.then_some(original);

            modify(decompressed, &reader_creator)?
        };

        info!("Compressing modified JSON data");

        // TODO: Allow writing uncompressed sessionstore files.
        compression::Encoder::compress(&modified_json_data, None, COMPRESSION_LIBRARY)
            .context("Failed to compress modified sessionstore data.")?
        // Drop modified_json_data here.
    };

    if overwrite_opt.overwrite_input || overwrite_opt.swap {
        let io_utils::InputReaderState::InputPath(input_path) = &reader_creator.state else {
            unreachable!("argument parser should ensure we don't read from stdin when overwriting input file");
        };

        let writer_creator = if overwrite_opt.swap {
            let writer_creator = session_opt
                .in_out_info
                .get_writer_creator_from_reader_creator(
                    &reader_creator,
                    "sessionstore",
                    "-",
                    format!("{output_postfix}-backup").as_str(),
                    input_path
                        .extension()
                        .map(|s| s.to_str().expect("UTF8 file extension"))
                        .unwrap_or("jsonlz4"),
                )?;

            info!(
                "Writing original input data to {}",
                writer_creator.output_info()
            );

            let Some(input_data) = input_data.take() else {
                unreachable!(
                    "We should always remember the input data when run with the --swap flag"
                );
            };

            io::copy(&mut &**input_data, &mut writer_creator.get_writer()?).with_context(|| {
                format!("Failed to write original input data to {}.", writer_creator)
            })?;
            drop(input_data);
            Some(writer_creator)
        } else {
            None
        };

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(input_path)
            .with_context(|| {
                format!(
                    "failed to open input file again to overwrite its content, file was at: {}",
                    input_path.display()
                )
            })?;

        info!(
            "Writing modified sessionstore data to re-opened input file at {}",
            input_path.display()
        );

        io::copy(&mut encoder, &mut file)
            .and_then(|_| file.flush())
            .with_context(|| {
                format!(
                    "Failed to write modified sessionstore data to re-opened input file at {}.",
                    input_path.display()
                )
            })?;
        drop(encoder);
        drop(file);

        if let Some(writer_creator) = writer_creator {
            session_opt.in_out_info.handle_output(writer_creator)?;
        }
    } else {
        let writer_creator = session_opt
            .in_out_info
            .get_writer_creator_from_reader_creator(
                &reader_creator,
                "sessionstore",
                "-",
                output_postfix,
                "jsonlz4",
            )?;

        info!(
            "Writing compressed data to {}",
            writer_creator.output_info()
        );

        io::copy(&mut encoder, &mut writer_creator.get_writer()?).with_context(|| {
            format!(
                "Failed to write modified sessionstore data to {}.",
                writer_creator
            )
        })?;
        drop(encoder);

        session_opt.in_out_info.handle_output(writer_creator)?;
    }
    Ok(())
}

pub fn run() -> Result<()> {
    color_eyre::install()?;

    let result = try_!({
        let opt = Opt::parse();

        if let Opt::TabsToLinksFormats { json } = opt {
            if json {
                #[derive(serde::Serialize)]
                struct JsonInfo<'a> {
                    name: &'a str,
                    alias_for: Option<&'a str>,
                    is_supported: bool,
                    description: String,
                    file_extension: &'a str,
                }
                let formats = to_links::ttl_formats::FormatInfo::all()
                    .iter()
                    .map(|format| {
                        let (link_format, as_pdf) = format.as_format().to_link_format();
                        JsonInfo {
                            name: format.as_str(),
                            alias_for: Some(format.follow_alias().as_str())
                                .filter(|&alias| alias != format.as_str()),
                            is_supported: format.as_format().is_supported(),
                            description: format.to_string(),
                            file_extension: to_links::TabsToLinksOutput {
                                format: link_format,
                                as_pdf,
                                conversion_options: Default::default(),
                            }
                            .file_extension(),
                        }
                    })
                    .collect::<Vec<_>>();
                serde_json::to_writer_pretty(io::stdout().lock(), &formats)
                    .context("Failed to serialize format info to stdout")?;
            } else {
                write!(
                    io::stdout().lock(),
                    "{}",
                    to_links::ttl_formats::InfoAboutFormats
                )
                .context("Failed to write info to stdout.")?;
            }
            return Ok(());
        }

        opt.common().configure_logging();

        trace!("Parsed arguments:\n{:#?}\n", opt);

        #[cfg(all(target_os = "wasi", target_env = "p1"))]
        {
            // Note: wasi preview 2 has a special method to get the current
            // directory so this should not be needed...

            if let Some(cd) = std::env::var_os("CD") {
                if let Err(e) = std::env::set_current_dir(&*cd) {
                    log::error!(
                        "Failed to set current working directory to {}: {e}",
                        cd.to_string_lossy()
                    );
                }
            }
        }

        match opt {
            Opt::AnalyzeJson {
                session,
                type_script,
                max_object_keys,
            } => {
                debug!("Executing: Analyze command");
                let reader_creator = session.get_reader_creator()?;

                info!("Analyzing JSON data");
                let stats = collect_statistics(
                    &reader_creator.deserialize_json_data::<serde_json::Value>()?,
                );

                let writer_creator = session.in_out_info.get_writer_creator_from_reader_creator(
                    &reader_creator,
                    "",
                    "-",
                    "json-analysis",
                    if type_script { "ts" } else { "txt" },
                )?;

                info!(
                    "Writing analyze results to {}",
                    writer_creator.output_info()
                );

                {
                    let mut writer = writer_creator.get_writer()?;

                    (if type_script {
                        write!(
                            writer,
                            "{}",
                            stats.with_formatter(TypeScriptStatisticsFormatter {
                                exported_type_name: Some("JsonData".into()),
                                indents: 0,
                                indent_text: "  ".into(),
                                parent_count: None,
                                max_object_keys,
                            })
                        )
                    } else {
                        write!(writer, "{}", stats)
                    })
                    .with_context(|| {
                        format!(
                            "Failed to write analytics information to {}.",
                            writer_creator
                        )
                    })?;
                }

                drop(stats);

                session.in_out_info.handle_output(writer_creator)?;
            }
            Opt::Copy(command) => {
                debug!("Executing: Copy command");
                let reader_creator = command.get_reader_creator()?;

                info!("Reading data from {}", reader_creator.reader_info());
                let mut reader = reader_creator.create_slice_reader()?;

                let writer_creator = command.in_out_info.get_writer_creator_from_reader_creator(
                    &reader_creator,
                    "sessionstore",
                    "-",
                    "copy",
                    reader_creator
                        .path()
                        .and_then(|p| p.extension())
                        .map(|s| s.to_str().expect("UTF8 file extension"))
                        .unwrap_or(if command.compression.uncompressed {
                            "js"
                        } else {
                            "jsonlz4"
                        }),
                )?;

                info!("Writing input data to {}", writer_creator.output_info());

                io::copy(&mut reader, &mut writer_creator.get_writer()?).with_context(|| {
                    format!("Failed to write input data to {}.", writer_creator)
                })?;
                drop(reader);

                command.in_out_info.handle_output(writer_creator)?;
            }
            Opt::Compress(command) => {
                debug!("Executing: Compress command");
                let mut encoder = {
                    let reader_creator = command.get_reader_creator(Some(false), &["js".into()])?;
                    let data = reader_creator.create_slice_reader()?.data;

                    info!("Compressing data from {}", reader_creator.reader_info());

                    compression::Encoder::compress(&data, None, COMPRESSION_LIBRARY)
                        .context("Failed to compress data.")?
                };

                let writer_creator = command.get_writer_creator("sessionstore", "jsonlz4")?;

                info!(
                    "Writing compressed data to {}",
                    writer_creator.output_info()
                );

                io::copy(&mut encoder, &mut writer_creator.get_writer()?).with_context(|| {
                    format!("Failed to write compressed data to {}.", writer_creator)
                })?;
                drop(encoder);

                command.handle_output(writer_creator)?;
            }
            Opt::Decompress(command) => {
                debug!("Executing: Decompress command");
                let decompressed = {
                    let reader_creator =
                        command.get_reader_creator(Some(false), &["jsonlz4".into()])?;
                    let data = reader_creator.create_slice_reader()?.data;

                    info!("Decompressing data from {}", reader_creator.reader_info());

                    compression::decompress(&data, COMPRESSION_LIBRARY)
                        .context("Failed to decompress data.")?
                };

                let writer_creator = command.get_writer_creator("sessionstore", "js")?;

                info!(
                    "Writing decompressed data to {}",
                    writer_creator.output_info()
                );

                writer_creator
                    .get_writer()?
                    .write_all(&decompressed)
                    .with_context(|| {
                        format!("Failed to write decompressed data to {}.", writer_creator)
                    })?;
                drop(decompressed);

                command.handle_output(writer_creator)?;
            }
            Opt::RemoveMarkedTabs {
                remove_options,
                overwrite_input,
                session,
            } => {
                debug!("Executing: RemoveMarkedTabs command");
                modify_sessionstore(
                    &session,
                    &overwrite_input,
                    "removed-tabs",
                    |input, input_info| {
                        info!("Deserializing JSON data from {}", input_info.reader_info());
                        let mut session = deserialize_from_slice(&input).with_context(|| {
                            format!("Failed to parse JSON from {}", input_info.reader_info())
                        })?;

                        remove_marked_tabs(&mut session, &remove_options)?;

                        info!("Serializing modified data to JSON");

                        serde_json::to_vec(&session).context(
                            "Failed to serialize modified sessionstore data to a JSON object.",
                        )
                    },
                )?;
            }
            Opt::RemoveTreeData {
                remove_options,
                overwrite_input,
                session,
            } => {
                debug!("Executing: RemoveTreeData command");
                modify_sessionstore(
                    &session,
                    &overwrite_input,
                    "removed-tree-data",
                    |input, input_info| {
                        info!("Deserializing JSON data from {}", input_info.reader_info());
                        let mut session = deserialize_from_slice(&input).with_context(|| {
                            format!("Failed to parse JSON from {}", input_info.reader_info())
                        })?;

                        remove_tree_data(&mut session, &remove_options)?;

                        info!("Serializing modified data to JSON");

                        serde_json::to_vec(&session).context(
                            "Failed to serialize modified sessionstore data to a JSON object.",
                        )
                    },
                )?;
            }
            Opt::Modify {
                overwrite_input,
                session,
                command,
                stop_exit_code,
                skip_json_verification,
            } => {
                debug!("Executing: Modify command");

                let Some(first) = command.first() else {
                    eyre::bail!("No command specified");
                };

                #[derive(Debug)]
                struct StopCode;
                impl std::error::Error for StopCode {}
                impl std::fmt::Display for StopCode {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(f, "External command exited with a known non-zero exit code")
                    }
                }

                let start = Instant::now();
                let res = modify_sessionstore(
                    &session,
                    &overwrite_input,
                    "modified",
                    |input, input_info| {
                        debug!(
                            "It took {:?} to read and decompress the sessionstore JSON data",
                            start.elapsed()
                        );
                        if !skip_json_verification {
                            let deserialize_start = Instant::now();
                            info!("Deserializing JSON data from {}", input_info.reader_info());
                            drop(
                                serde_json::from_slice::<serde_json::Value>(&input)
                                    .map_err(|e| json_parse_error_context(e, &input))
                                    .with_context(|| {
                                        format!(
                                            "Failed to parse JSON from {}",
                                            input_info.reader_info()
                                        )
                                    })?,
                            );
                            debug!(
                                "Validation of original firefox sessionstore JSON data finished after {:?}",
                                deserialize_start.elapsed()
                            );
                        }

                        let mut process = Command::new(first)
                            .args(command.iter().skip(1))
                            .stdin(Stdio::piped())
                            .stdout(Stdio::piped())
                            .stderr(Stdio::inherit())
                            .spawn()
                            .with_context(|| {
                                format!(
                                    "Failed to spawn process for command: {}",
                                    first.to_string_lossy()
                                )
                            })?;
                        info!("Started command \"{}\"", first.to_string_lossy());
                        let after_spawn = Instant::now();

                        let (read_res, write_res, command_writing_after) = thread::scope(|s| {
                            let (tx, rx) = std::sync::mpsc::sync_channel::<()>(1);
                            let reader = s.spawn(|| {
                                let mut stdout = BufReader::new(process.stdout.as_mut().unwrap());
                                stdout.fill_buf().context(
                                    "failed to wait for first byte from command's stdout",
                                )?;
                                drop(tx);
                                debug!(
                                    "Command started writing to its stdout after {:?}",
                                    after_spawn.elapsed()
                                );
                                let read_start = Instant::now();
                                let res = {
                                    let mut data = Vec::new();
                                    stdout
                                        .read_to_end(&mut data)
                                        .context("failed to read from command's stdout")
                                        .map(|_| data)
                                };
                                debug!(
                                    "Finished reading JSON from command's stdout, it took {:?}",
                                    read_start.elapsed()
                                );
                                res
                            });
                            let mut input_ref = input.as_slice();
                            let write_res = std::io::copy(
                                &mut input_ref,
                                // Take stdin so its closed when we have
                                // written all data:
                                &mut BufWriter::new(process.stdin.take().unwrap()),
                            )
                            .context("failed to write sessionstore JSON data to command's stdin");
                            let write_end = Instant::now();
                            debug!(
                                "Finished writing to command's stdin after {:?}",
                                after_spawn.elapsed()
                            );
                            drop(input); // Free memory!

                            let _ = rx.recv();
                            let command_writing_after = write_end.elapsed();

                            let read_res = reader.join().unwrap();

                            (read_res, write_res, command_writing_after)
                        });
                        debug!("Waiting for command to exit");
                        let status = process
                            .wait()
                            .context("failed to wait for command to exit")?;
                        let elapsed = after_spawn.elapsed();
                        info!("Command exited after {elapsed:?} (Excluding reading and writing the command took {command_writing_after:?})");
                        if !status.success() {
                            if let Some(code) = status.code() {
                                if stop_exit_code.iter().any(|&stop| stop == i64::from(code)) {
                                    info!("The command's exit code was {code} and so the command's output was ignored.");
                                    return Err(StopCode.into());
                                }
                            }

                            eyre::bail!(
                                "Command exited with an error {}",
                                if let Some(code) = status.code() {
                                    format!("(exit code: {code})")
                                } else {
                                    "".to_string()
                                }
                            );
                        }
                        let modified_data = read_res?;
                        write_res?;

                        if skip_json_verification {
                            Ok(modified_data)
                        } else {
                            info!("Validating modified sessionstore JSON from command");
                            let start = Instant::now();
                            let json =  serde_json::from_slice::<serde_json::Value>(&modified_data)
                                .context("The data written to the commands stdout could not be parsed as JSON")?;
                            let data = serde_json::to_vec(&json)
                                .context("Failed to serialize modified sessionstore data");
                            debug!("Validation finished after {:?}", start.elapsed());
                            data
                        }
                    },
                );
                debug!("Execution completed after {:?}", start.elapsed());

                // Ignore stop because of known exit code.
                let known_stop =
                    matches!(&res, Err(e) if e.root_cause().downcast_ref::<StopCode>().is_some());
                if !known_stop {
                    res?;
                }
            }
            Opt::Domains(command) => {
                debug!("Executing: Domains command");
                let reader_creator = command.get_reader_creator()?;

                info!(
                    "Deserializing JSON data from {}",
                    reader_creator.reader_info()
                );

                let session =
                    reader_creator.deserialize_json_data::<session_store::FirefoxSessionStore>()?;

                // Code inspired by blog post at:
                // https://blog.dend.ro/decoding-firefox-session-store-data/
                let domains = {
                    let mut domains = HashMap::<String, u32>::new();
                    for window in &session.windows {
                        for tab in &window.tabs {
                            let tab = session_store::session_info::TabInfo::new(tab);
                            match url::Url::parse(tab.url()) {
                                Ok(url) => {
                                    // skip about:blank, about:reader etc.
                                    if let Some(host) = url.host_str() {
                                        *domains.entry(host.to_string()).or_default() += 1;
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to parse the tab URL {:?} because: {}",
                                        tab.url(),
                                        e
                                    );
                                }
                            }
                        }
                    }
                    let mut domains = domains.into_iter().collect::<Vec<_>>();
                    domains.sort_unstable_by_key(|&(_, count): &(_, u32)| Reverse(count));
                    domains
                };

                let writer_creator = command.in_out_info.get_writer_creator_from_reader_creator(
                    &reader_creator,
                    "",
                    "-",
                    "open-domains",
                    "txt",
                )?;

                info!("Writing domains info to {}", writer_creator.output_info());

                {
                    let mut writer = writer_creator.get_writer()?;
                    try_!({
                        for (domain, count) in domains.into_iter() {
                            writeln!(writer, "{} {}", domain, count)?;
                        }
                    })
                    .with_context(|| {
                        format!("Failed to write domains information to {}.", writer_creator)
                    })?;
                }

                drop(session);

                command.in_out_info.handle_output(writer_creator)?;
            }
            Opt::GetGroups {
                session: session_store_opt,
                tab_group_options,
                json,
            } => {
                debug!("Executing: GetGroups command");
                let reader_creator = session_store_opt.get_reader_creator()?;

                info!(
                    "Deserializing JSON data from {}",
                    reader_creator.reader_info()
                );

                let session =
                    reader_creator.deserialize_json_data::<session_store::FirefoxSessionStore>()?;

                let groups = session_store::session_info::get_groups_from_session(
                    &session,
                    !tab_group_options.only_closed_windows,
                    tab_group_options.closed_windows || tab_group_options.only_closed_windows,
                    !tab_group_options.no_sorting,
                )
                .collect::<Vec<_>>();

                let writer_creator = session_store_opt
                    .in_out_info
                    .get_writer_creator("tab-groups", if json { "json" } else { "txt" })?;
                {
                    let mut writer = writer_creator.get_writer()?;

                    if json {
                        #[derive(serde::Serialize)]
                        struct JsonGroup<'a> {
                            name: &'a str,
                            tab_count: u64,
                            is_closed: bool,
                        }
                        let json_groups = groups
                            .iter()
                            .map(|group| JsonGroup {
                                name: group.name(),
                                tab_count: u64::try_from(group.tabs().len()).unwrap(),
                                is_closed: group.is_closed(),
                            })
                            .collect::<Vec<_>>();
                        serde_json::to_writer_pretty(writer, &json_groups).with_context(|| {
                            format!(
                                "Failed to serialize tab group info as JSON to {}",
                                writer_creator
                            )
                        })?;
                    } else {
                        try_!({
                            let mut is_closed = false;
                            for group in groups {
                                if is_closed != group.is_closed() {
                                    // Closed windows come after open ones.
                                    writeln!(writer)?;
                                    is_closed = true;
                                }
                                writeln!(writer, "{}", group.name())?;
                            }
                        })
                        .with_context(|| {
                            format!(
                                "Failed to write tab group information to {}.",
                                writer_creator
                            )
                        })?;
                    }
                    drop(session);
                }

                session_store_opt
                    .in_out_info
                    .handle_output(writer_creator)?;
            }
            Opt::TabsToLinks(command) => {
                debug!("Executing: TabsToLinks command");
                let options = command.parse_options()?;

                let session_store_opt = &command.session_store_opt;
                let reader_creator = session_store_opt.get_reader_creator()?;

                info!(
                    "Deserializing JSON data from {}",
                    reader_creator.reader_info()
                );

                let session =
                    reader_creator.deserialize_json_data::<session_store::FirefoxSessionStore>()?;

                let mut writer_creator = session_store_opt
                    .in_out_info
                    .get_writer_creator("Links", options.file_extension())?;

                let writer_info = writer_creator.output_info().to_string();

                info!("Writing links to {}", writer_info);

                // Select windows/groups:
                let groups = session_store::session_info::get_groups_from_session(
                    &session,
                    !command.tab_group_options.only_closed_windows,
                    command.tab_group_options.closed_windows
                        || command.tab_group_options.only_closed_windows,
                    !command.tab_group_options.no_sorting,
                );
                let groups = if !command.tab_group_indexes.is_empty()
                    || !command.tab_group_names.is_empty()
                {
                    groups
                        .enumerate()
                        .filter(|(index, group)| {
                            command.tab_group_indexes.contains(&(*index as u64))
                                || command
                                    .tab_group_names
                                    .iter()
                                    .any(|name| name == group.name())
                        })
                        .map(|(_, group)| group)
                        .collect::<Vec<_>>()
                } else {
                    groups.collect::<Vec<_>>()
                };

                tabs_to_links(&groups, options, &mut writer_creator)
                    .with_context(|| format!("Failed to write links to {}.", writer_info))?;
                drop(session);

                session_store_opt
                    .in_out_info
                    .handle_output(writer_creator)?;
            }
            Opt::TabsToLinksFormats { .. } => {
                unreachable!("We handled this earlier");
            }
        }

        info!("Finished");
    });
    add_backtrace_note_to_error(result)
}

/// Add a note in the error about how to enable backtraces via environment variables.
pub fn add_backtrace_note_to_error<T>(result: Result<T>) -> Result<T> {
    result.note(
        "backtraces are controlled via environment variables:\n\
            If you want panics and errors to both have backtraces, set RUST_BACKTRACE=1.\n\
            If you want only errors to have backtraces, set RUST_LIB_BACKTRACE=1.\n\
            If you want only panics to have backtraces, set RUST_BACKTRACE=1 and RUST_LIB_BACKTRACE=0.\n\
            If you want backtraces to be printed with source locations, set RUST_LIB_BACKTRACE=full.\n\
        ",
    )
}

fn verbosity_level(verbose: u64) -> Option<log::Level> {
    use log::Level::*;
    Some(match verbose {
        0 => return None,
        1 => Error,
        2 => Warn,
        3 => Info,
        4 => Debug,
        _ => Trace,
    })
}

fn init_logger(default_level: Option<log::Level>) {
    use chrono::Local;
    use env_logger::{Builder, Env};
    use log::Level;

    let mut builder = Builder::from_env(Env::new().default_filter_or(
        if let Some(default_level) = default_level {
            format!("{:?}", default_level).to_lowercase()
        } else {
            "off".to_string()
        },
    ));

    builder.format(|formatter, record| {
        writeln!(
            formatter,
            " {} {} {}: {}",
            // formatter.timestamp(),
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            formatter
                .default_level_style(record.level())
                .value(format_args!(
                    "[{}]{}",
                    record.level(),
                    match record.level() {
                        Level::Debug | Level::Error | Level::Trace => "",
                        Level::Info | Level::Warn => " ",
                    }
                )),
            formatter
                .style()
                .set_bold(true)
                .value(format_args!("({})", record.target())),
            record.args()
        )
    });

    builder.init();
}
