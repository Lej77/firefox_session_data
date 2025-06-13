//! Print tabs stored in sessionstore file as links.

use crate::{
    pdf_converter::{self, DotNetFrameworkItextMode},
    session_store, Result, SessionstoreOpt,
};
use clap::{Parser, ValueEnum};
use eyre::anyhow;
use session_store::{
    session_info::TreeDataSource,
    to_links::{LinkFormat, ToLinksOptions},
};

pub mod ttl_formats {
    //! Info and CLI definitions for the output formats that are supported by the
    //! `tabs-to-links` command.
    #![allow(non_camel_case_types)]
    #![allow(clippy::upper_case_acronyms)]

    use clap::Args;
    use std::fmt;
    use std::str::FromStr;

    macro_rules! define {
        ($(
            $( #[ $( $meta_token:tt )* ] )*
            $([extra_info(
                $extra_info:literal
                $(,$extra_arg:expr)*
                $(,)?
            )])?
            $([supported(
                $supported:expr
            )])?
            $name:ident = $value:expr
            $( => $alias:ident $($_alias:ident)?)?
        ),* $(,)? ) => {
            $(
                $( #[ $( $meta_token )* ] )*
                pub const $name: &str = $value;
            )*

            #[derive(Debug, Args, Clone)]
            #[clap(rename_all = "kebab-case")]
            pub struct FormatOpt {
                #[clap(
                    long,
                    visible_alias = "fmt",
                    default_value = "pdf",
                    value_parser = [$($name,)*],
                )]
                /// Specify the format of the output file.
                ///
                /// Use the `tabs-to-links-formats` command to get more information
                /// about the different formats that are supported.
                pub format: String,
            }

            /// An enum with all formats that the user can specify to allow
            /// for exhaustive match.
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub enum Format {
                $(
                    $(#[cfg(any())] $($_alias)? )?
                    $( #[ $( $meta_token )* ] )*
                    $name,
                )*
            }
            impl Format {
                // Fallbacks for aliases which aren't present in the actual enum:
                $($(
                    #[doc = core::concat!("Alias for ", core::stringify!($alias))]
                    pub const $name: Format = Format::$alias;
                )?)*

                /// Get information about the format that can be printed to
                /// inform the user about it.
                pub fn as_info(self) -> FormatInfo {
                    match self {$(
                        $(#[cfg(any())] $($_alias)? )?
                        Format::$name => FormatInfo::$name,
                    )*}
                }

                pub fn is_supported(self) -> bool {
                    match self {$(
                        $(#[cfg(any())] $($_alias)? )?
                        Format::$name => true $(&& $supported)?,
                    )*}
                }
            }
            /// Parses from CLI argument name to format.
            impl FromStr for Format {
                type Err = ();

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    match s {
                        $(
                            $name => {
                                $(#[cfg(any())] $($_alias)? )?
                                {
                                    Ok(Format::$name)
                                }
                                $(Ok(Format::$alias) $($_alias)? )?
                            },
                        )*
                        _ => Err(())
                    }
                }
            }
            /// Prints the CLI argument name for this format.
            impl fmt::Display for Format {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    match self {
                        $(
                            $(#[cfg(any())] $($_alias)? )?
                            Format::$name => write!(f, "{}", $name),
                        )*
                    }
                }
            }

            /// Used to print information about the supported output formats.
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub enum FormatInfo {
                $(
                    $( #[ $( $meta_token )* ] )*
                    $name,
                )*
            }
            impl FormatInfo {
                /// Info about all formats and aliases.
                pub fn all() -> &'static [FormatInfo] {
                    &[$(FormatInfo::$name,)*]
                }
                /// `true` if this variant is an alias for another format.
                pub fn is_alias(self) -> bool {
                    match self {$(
                        FormatInfo::$name => {
                            $(
                                $($_alias)?
                                return true;
                            )?
                        }
                    )*}
                    false
                }
                /// Forget alias information.
                pub fn as_format(self) -> Format {
                    match self {$(
                        FormatInfo::$name => Format::$name,
                    )*}
                }
                /// Get the format that this is an alias for.
                pub fn follow_alias(self) -> FormatInfo {
                    match self {
                        $(
                            FormatInfo::$name => {
                                $(#[cfg(any())] $($_alias)? )?
                                {
                                    FormatInfo::$name
                                }
                                $(FormatInfo::$alias $($_alias)? )?
                            },
                        )*
                    }
                }
                /// The name of the format in the CLI option.
                pub fn as_str(self) -> &'static str {
                    match self {$(
                        FormatInfo::$name => $name,
                    )*}
                }
            }
            impl FromStr for FormatInfo {
                type Err = ();

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    match s {
                        $(
                            $name => Ok(FormatInfo::$name),
                        )*
                        _ => Err(())
                    }
                }
            }
            impl fmt::Display for FormatInfo {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    match self {$(
                        FormatInfo::$name => {
                            write!(f, "`{}`:", $name)?;
                            writeln!(f)?;
                            $(
                                // Extract the content of a document comment:
                                define!(@get-doc-text #[$($meta_token)*] as text => {
                                    writeln!(f, "{}", text)?;
                                });
                            )*
                            $(
                                writeln!(f, $extra_info $(,$extra_arg)*)?;
                            )?
                            $(
                                // This is simply an alias for another format, write info about which:
                                writeln!(f)?;
                                writeln!(f, " (alias for '{}')", $alias)?;
                                $($_alias)?
                            )?
                        }
                    )*}
                    Ok(())
                }
            }

            /// Used to print information about the supported output formats.
            #[derive(Debug)]
            pub struct InfoAboutFormats;
            impl fmt::Display for InfoAboutFormats {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    write!(f, "These are the formats supported by the `tabs-to-links` command:")?;
                    writeln!(f)?;

                    for format in FormatInfo::all() {
                        writeln!(f)?;
                        <FormatInfo as fmt::Display>::fmt(format, f)?;
                        writeln!(f)?;
                    }

                    writeln!(f)?;
                    Ok(())
                }
            }
        };
        (@hidden $($token:tt)*) => {};
        (@get-doc-text #[doc = $text:literal] as $name:ident => $($token:tt)*) => {
            let $name = $text;
            $($token)*
        };
        (@get-doc-text #[ $( $meta_token:tt )* ] as $name:ident => $($token:tt)*) => {};
    }

    define! {
        /// Write the links to a text file (".txt" file extension). The output
        /// file can be opened with any Text Editor, for example Notepad should
        /// work fine.
        TEXT = "text",
        /// Write the links in the RTF format (".rtf" file extension). The output
        /// file can be opened in WordPad or Word.
        RTF = "rtf",
        /// Write the links in the RTF format (".rtf" file extension). The output
        /// file can be opened in WordPad or Word. This format tries to keep the
        /// output formatting simpler to decrease file size and to hopefully limit
        /// issues that can occur.
        RTF_SIMPLE = "rtf-simple",
        /// Write the links in the HTML format (".html" file extension). The
        /// output file can be opened in a web browser.
        HTML = "html",
        /// Write the links in the PDF format (".pdf" file extension). The output
        /// file can be opened in a web browser or with a PDF viewer such as Adobe
        /// Reader.
        ///
        /// This will use one of the more specific PDF formats, but which one might
        /// change in the future.
        PDF = "pdf" => PDF_TYPST,
        /// Write the links in the Markdown format (".md" file extension). This
        /// format is supported on several web pages such as when making comments
        /// on Reddit or GitHub.
        MARKDOWN = "markdown",
        /// Write the links in the Typst document format (".typ" file extension).
        /// Typst is a modern alternative to LaTeX and can easily be converted to
        /// a PDF.
        TYPST = "typst",

        /// Use Typst as a library (not an external program) to generate a PDF file.
        [extra_info(
            "{}",
            if cfg!(not(feature = "typst_pdf")) {" [Note: Typst was not included when this version of the program was compiled and so this format will fail.]"} else {""},
        )]
        [supported(cfg!(feature = "typst_pdf"))]
        PDF_TYPST = "pdf-typst",

        /// A C# HTML to PDF converter using its more modern implementation.
        /// - Slowest of the alternatives in the C# program.
        /// - Most accurate of the C# program's converters (for example some
        ///   japanese characters are only correctly shown with this option).
        /// - No PDF Table of Contents.
        [supported(cfg!(all(feature = "to_pdf_dotnet_itext", not(target_family = "wasm"))))]
        PDF_MODERN = "pdf-modern",
        /// A C# HTML to PDF converter using its older legacy implementation.
        /// - Links will not be colored blue but they can still be clicked.
        /// - No PDF Table of Contents.
        [supported(cfg!(all(feature = "to_pdf_dotnet_framework_itext", not(target_family = "wasm"))))]
        PDF_LEGACY = "pdf-legacy",
        /// A C# HTML to PDF converter using its older XML implementation in its
        /// simpler mode.
        /// - Supports PDF Table of Contents for easier navigation.
        [supported(cfg!(all(feature = "to_pdf_dotnet_framework_itext", not(target_family = "wasm"))))]
        PDF_XML_SIMPLE = "pdf-xml-simple",
        /// A C# HTML to PDF converter using its older XML implementation in
        /// advanced mode. This uses simpler default CSS.
        /// - Supports PDF Table of Contents for easier navigation.
        [supported(cfg!(all(feature = "to_pdf_dotnet_framework_itext", not(target_family = "wasm"))))]
        PDF_XML_ADV = "pdf-xml-adv",

        /// Use the "wkhtmltopdf" project to convert HTML to PDF. This uses the
        /// "QT Webkit" web browser to render HTML for PDF generation. Installing
        /// "wkhtmltopdf" is required before this can be used unless the necessary
        /// files was compiled into this program (if the files weren't included
        /// and they couldn't be found then you will get an error).
        [extra_info(
            " [Note: in this version of the program the files are {}included]",
            if cfg!(feature = "wk_html_to_pdf_include_dll") {""} else {"NOT "},
        )]
        PDF_WK_HTML = "pdf-wk-html" => PDF_WK_HTML_LINKED,
        /// Use the "wkhtmltopdf" project to convert HTML to PDF. This uses the
        /// "QT Webkit" web browser to render HTML for PDF generation. The
        /// "wkhtmltopdf.dll" library file with a certain version is required and
        /// must be in PATH or in the Current Directory before this can be used
        /// unless the necessary files was compiled into this program (if the
        /// files weren't included and they couldn't be found then you will get
        /// an error).
        [extra_info(
            " [Note: in this version of the program the files are {}included]",
            if cfg!(feature = "wk_html_to_pdf_include_dll") {""} else {"NOT "},
        )]
        [supported(cfg!(all(feature = "wk_html_to_pdf", not(target_family = "wasm"))))]
        PDF_WK_HTML_LINKED = "pdf-wk-html-linked",
        // Uses QT Webkit to render HTML for PDF generation. "wkhtmltopdf.exe"
        // must be in PATH or Current Directory if it isn't included in this binary.
        // PDF_WK_HTML_SHELLED = "pdf-wk-html-shelled",

        /// Use the Rust library "chromiumoxide" to control a headless Chrome
        /// browser with the DevTools Protocol in order to load HTML and "print"
        /// a PDF. Requires that Chrome is installed.
        [supported(cfg!(all(feature = "chromiumoxide_conversion", not(target_family = "wasm"))))]
        PDF_CHROMIUM_OXIDE = "pdf-chromium-oxide",
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum TreeData {
    /// Get tree data from Tree Style Tabs.
    TST,
    /// Get tree data from Sidebery.
    Sidebery,
    /// Don't use tree data.
    None,
}
impl TreeData {
    pub fn to_tree_sources(wanted: &[TreeData]) -> Vec<TreeDataSource> {
        wanted
            .iter()
            .flat_map(|v| match v {
                TreeData::TST => {
                    &[TreeDataSource::TstWebExtension, TreeDataSource::TstLegacy] as &[_]
                }
                TreeData::Sidebery => &[TreeDataSource::Sidebery],
                TreeData::None => &[],
            })
            .copied()
            .collect::<Vec<_>>()
    }
}

impl ttl_formats::Format {
    pub fn to_link_format(self) -> (LinkFormat, Option<pdf_converter::PdfConversionMethod>) {
        use pdf_converter::PdfConversionMethod as PdfMode;
        use ttl_formats::Format;
        use LinkFormat::*;

        match self {
            Format::TEXT => (TXT, None),
            Format::RTF => (
                RTF {
                    picture_horizontal_line: true,
                },
                None,
            ),
            Format::RTF_SIMPLE => (
                RTF {
                    picture_horizontal_line: false,
                },
                None,
            ),
            Format::MARKDOWN => (Markdown, None),
            Format::HTML => (HTML, None),
            Format::TYPST => (Typst, None),
            Format::PDF_TYPST => (Typst, Some(PdfMode::Typst)),
            Format::PDF_LEGACY => (
                HTML,
                Some(PdfMode::DotNetItextFramework {
                    mode: DotNetFrameworkItextMode::PdfLegacy {
                        custom_page_break: None,
                    },
                }),
            ),
            Format::PDF_XML_SIMPLE => (
                HTML,
                Some(PdfMode::DotNetItextFramework {
                    mode: DotNetFrameworkItextMode::PdfXmlSimple,
                }),
            ),
            Format::PDF_XML_ADV => (
                HTML,
                Some(PdfMode::DotNetItextFramework {
                    mode: DotNetFrameworkItextMode::PdfXmlAdv,
                }),
            ),
            Format::PDF_MODERN => (HTML, Some(PdfMode::DotNetItext)),
            Format::PDF_WK_HTML_LINKED => (HTML, Some(PdfMode::Wkhtml { shelled: false })),
            Format::PDF_CHROMIUM_OXIDE => (HTML, Some(PdfMode::Chromiumoxide)),
        }
    }
}

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case")]
pub struct TabGroupOptions {
    #[clap(long, visible_alias = "no_sort")]
    /// Don't sort windows or tab groups after their names.
    pub no_sorting: bool,

    #[clap(long, requires = "closed-windows")]
    /// Only include info from recently closed windows and ignore all open
    /// windows.
    pub only_closed_windows: bool,

    #[clap(long)]
    /// Include info from recently closed windows as well as open windows.
    pub closed_windows: bool,
}

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case")]
pub struct TabsToLinksOpt {
    #[clap(flatten)]
    pub session_store_opt: SessionstoreOpt,

    #[clap(flatten)]
    pub format: ttl_formats::FormatOpt,

    #[clap(short, long)]
    /// Make page breaks between different windows' tabs. This is not supported
    /// for all formats (in which case the windows' tabs will be appended after
    /// each other without separation).
    pub page_breaks: bool,

    #[clap(long, visible_alias = "no_toc")]
    /// Don't include a table of contents in the beginning of the output file.
    pub no_table_of_contents: bool,

    #[clap(long, visible_alias = "ial")]
    /// Indent all links so that word wrap doesn't make it hard to determine
    /// where a new link starts.
    pub indent_all_links: bool,

    #[clap(flatten)]
    pub tab_group_options: TabGroupOptions,

    #[clap(long, visible_alias = "tgi", value_delimiter = ',')]
    /// Only generate links for the tab groups specified by these indexes.
    /// Multiple indexes can be specified by separating them with commas (,).
    pub tab_group_indexes: Vec<u64>,

    #[clap(long, visible_alias = "tgi")]
    /// Only generate links for the tab groups specified by these names.
    pub tab_group_names: Vec<String>,

    #[clap(
        long,
        value_enum,
        action = clap::ArgAction::Append,
        value_delimiter = ',',
    )]
    /// Visualize tab trees from addons like Tree Style Tab.
    ///
    /// Multiple tree data sources can be specified by separating them with
    /// commas (,). The first data source that exists in the session file will
    /// be used. (So if you ever installed Tree Style Tab and haven't closed all
    /// tabs that existed last it was installed then its data will exist.)
    pub tree_data: Vec<TreeData>,
}
impl TabsToLinksOpt {
    pub fn get_options_for_format(&self, format: ttl_formats::Format) -> TabsToLinksOutput {
        let (format, as_pdf) = format.to_link_format();

        let tree_sources = TreeData::to_tree_sources(self.tree_data.as_slice());
        let conversion_options = session_store::to_links::ToLinksOptions {
            format,
            page_breaks_after_group: self.page_breaks,
            skip_page_break_after_last_group: (format.is_html() || format.is_typst())
                && self.page_breaks,
            table_of_contents: !self.no_table_of_contents,
            indent_all_links: self.indent_all_links,
            custom_page_break: "".into(),
            tree_sources: tree_sources.into(),
        };
        TabsToLinksOutput {
            format,
            as_pdf,
            conversion_options,
        }
    }

    pub fn parse_format(&self) -> Result<ttl_formats::Format> {
        self.format
            .format
            .to_lowercase()
            .as_str()
            .parse::<ttl_formats::Format>()
            .map_err(|_| anyhow!("Incorrect format argument: \"{}\"", self.format.format))
    }
    /// Parse "tabs to links" options and return the info together with the
    /// normal file extension for the produced format.
    pub fn parse_options(&self) -> Result<TabsToLinksOutput> {
        let format = self.parse_format()?;
        Ok(self.get_options_for_format(format))
    }
}

pub struct TabsToLinksOutput {
    pub format: LinkFormat,
    pub as_pdf: Option<pdf_converter::PdfConversionMethod>,
    pub conversion_options: ToLinksOptions<'static>,
}
impl TabsToLinksOutput {
    /// The file extension for the produced format.
    pub fn file_extension(&self) -> &'static str {
        use LinkFormat::*;

        if self.as_pdf.is_some() {
            return "pdf";
        }
        match self.format {
            TXT => "txt",
            RTF { .. } => "rtf",
            HTML => "html",
            Markdown => "md",
            Typst => "typ",
        }
    }
}
