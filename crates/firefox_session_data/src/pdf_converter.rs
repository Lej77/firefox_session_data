//! Provides PDF conversions from other formats (currently HTML or Typst documents).
#![cfg_attr(target_family = "wasm", expect(unused_imports, unused_variables))]

use crate::Result;
use eyre::bail;
use firefox_session_store::to_links::ToLinksOptions;
use html_to_pdf::{HtmlSink, HtmlToPdfConverter, PdfScope, WriteBuilder};

pub use html_to_pdf;

/// Configuration for different PDF converters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PdfConversionMethod {
    /// Use a small C# program that calls into the iText .Net Framework library
    /// to generate a PDF from a HTML document, see:
    ///
    /// https://www.nuget.org/packages/itextsharp.xmlworker
    DotNetItextFramework {
        /// The program supports different modes since the C# library it uses
        /// has different ways to handle the conversion.
        mode: DotNetFrameworkItextMode,
    },
    /// Use the iText .Net library via a small C# program. This is slower than
    /// the older .Net Framework iText library but has more accurate results
    /// (for example some japanese characters are only correctly shown with this
    /// option).
    ///
    /// - No PDF Table of Contents.
    DotNetItext,
    /// Use "wkhtmltopdf" to handle the conversion.
    Wkhtml {
        /// Shell out to the "wkhtmltopdf" executable. If this is `false` we will
        /// attempt to link to the "wkhtmltopdf" library instead.
        shelled: bool,
    },
    /// Use the Rust library "chromiumoxide" to control a headless Chrome
    /// browser with the DevTools Protocol in order to load HTML and "print" a
    /// PDF.
    ///
    /// Note: it's important to specify "<meta charset="UTF-8">" in the HTML
    /// file's head section; otherwise it might not handle all characters
    /// correctly.
    Chromiumoxide,
    /// Use Typst as a library to generate a PDF.
    Typst,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DotNetFrameworkItextMode {
    /// A C# HTML to PDF converter using its older legacy implementation.
    ///
    /// - Links will not be colored blue but they can still be clicked.
    /// - No PDF Table of Contents.
    PdfLegacy {
        /// This mode doesn't support page break info from the HTML file. This
        /// argument allows specifying a custom string that should be
        /// interpreted as a page break.
        custom_page_break: Option<String>,
    },
    /// A C# HTML to PDF converter using its older XML implementation in its
    /// simpler mode.
    ///
    /// - More than twice as slow when <a> tags are inside a <div>.
    /// - Supports PDF Table of Contents for easier navigation.
    PdfXmlSimple,
    /// A C# HTML to PDF converter using its older XML implementation in
    /// advanced mode.
    ///
    /// - More than twice as slow when <a> tags are inside a <div>.
    /// - No PDF Table of Contents.
    PdfXmlAdv,
}
#[cfg(all(feature = "to_pdf_dotnet_framework_itext", not(target_family = "wasm")))]
impl DotNetFrameworkItextMode {
    fn mode(&self) -> html_to_pdf_adapter_dotnet_framework_itext::DotNetFrameworkPdfConverterMode {
        use html_to_pdf_adapter_dotnet_framework_itext::DotNetFrameworkPdfConverterMode as Mode;
        use DotNetFrameworkItextMode::*;

        match self {
            PdfLegacy { .. } => Mode::ObsoleteHTMLParser,
            PdfXmlSimple => Mode::XMLWorkerSimple,
            PdfXmlAdv => Mode::XMLWorkerAdvanced,
        }
    }
    fn into_converter(
        self,
    ) -> html_to_pdf_adapter_dotnet_framework_itext::DotNetFrameworkPdfConverter {
        html_to_pdf_adapter_dotnet_framework_itext::DotNetFrameworkPdfConverter {
            mode: self.mode(),
            custom_page_break: if let DotNetFrameworkItextMode::PdfLegacy { custom_page_break } =
                self
            {
                custom_page_break.map(std::ffi::OsString::from)
            } else {
                None
            },
            #[cfg(feature = "to_pdf_dotnet_itext_include_exe")]
            extract_included_exe_at: Some(std::env::temp_dir().join("HtmlToPdf_Framework")),
            #[cfg(not(feature = "to_pdf_dotnet_itext_include_exe"))]
            extract_included_exe_at: None,
        }
    }
}

/// Describes a Pdf conversion that is supported by this program.
pub struct SupportedPdfConversion<'a, 'b> {
    pub method: PdfConversionMethod,
    pub link_options: &'a mut ToLinksOptions<'b>,
}
impl<'scope, W> HtmlToPdfConverter<'scope, W> for SupportedPdfConversion<'_, '_>
where
    W: WriteBuilder + Send + 'scope,
{
    type HtmlSink = Box<dyn HtmlSink<W, Self::Error> + 'scope>;
    type Error = eyre::Error;

    fn start(self, scope: PdfScope<'scope, '_>, output: W) -> Result<Self::HtmlSink> {
        let SupportedPdfConversion {
            method: pdf_method,
            link_options: _options,
        } = self;

        Ok(match pdf_method {
            #[cfg(not(target_family = "wasm"))]
            #[cfg_attr(not(feature = "to_pdf_dotnet_framework_itext"), expect(unused))]
            PdfConversionMethod::DotNetItextFramework { mut mode } => {
                #[cfg(feature = "to_pdf_dotnet_framework_itext")]
                {
                    if let DotNetFrameworkItextMode::PdfLegacy { custom_page_break } = &mut mode {
                        // Handle page breaks manually in this mode by inserting magic string:
                        _options.page_breaks_after_group = false;
                        _options.custom_page_break = std::borrow::Cow::Owned(
                                custom_page_break.get_or_insert_with(||
                                    html_to_pdf_adapter_dotnet_framework_itext::RECOMMENDED_PAGE_BREAK.to_owned()
                                ).clone()
                            );
                    }
                    Box::new(mode.into_converter().start(scope, output)?)
                }
                #[cfg(not(feature = "to_pdf_dotnet_framework_itext"))]
                {
                    bail!(
                        r#"The C# .Net Framework PDF conversion program wasn't included when this program was created."#
                    );
                }
            }
            #[cfg(not(target_family = "wasm"))]
            PdfConversionMethod::DotNetItext => {
                #[cfg(feature = "to_pdf_dotnet_itext")]
                {
                    Box::new(
                        html_to_pdf_adapter_dotnet_itext::DotNetPdfConverter {
                            #[cfg(feature = "to_pdf_dotnet_itext_include_exe")]
                            extract_included_exe_at: Some(std::env::temp_dir().join("HtmlToPdf")),
                            #[cfg(not(feature = "to_pdf_dotnet_itext_include_exe"))]
                            extract_included_exe_at: None,
                        }
                        .start(scope, output)?,
                    )
                }
                #[cfg(not(feature = "to_pdf_dotnet_itext"))]
                {
                    bail!(
                        r#"The C# .Net PDF conversion program wasn't included when this program was created."#
                    );
                }
            }
            #[cfg(not(target_family = "wasm"))]
            PdfConversionMethod::Wkhtml { shelled } => {
                if shelled {
                    bail!("Shell out to prince for PDF conversion is not supported yet.");
                }
                #[cfg(feature = "wk_html_to_pdf")]
                {
                    Box::new(html_to_pdf_adapter_wkhtml::WkHtmlPdfConverter.start(scope, output)?)
                }
                #[cfg(not(feature = "wk_html_to_pdf"))]
                {
                    bail!(
                        r#"The WKHtmlToPdf PDF conversion program wasn't included when this program was created."#
                    );
                }
            }
            #[cfg(not(target_family = "wasm"))]
            PdfConversionMethod::Chromiumoxide => {
                #[cfg(not(feature = "chromiumoxide_conversion"))]
                {
                    bail!(
                        r#"The "chromiumoxide" Rust library wasn't built when this program was created."#
                    );
                }
                #[cfg(feature = "chromiumoxide_conversion")]
                {
                    Box::new(
                        html_to_pdf_adapter_chromiumoxide::ChromiumoxideConverter {
                            pdf_options: Default::default(),
                        }
                        .start(scope, output)
                        .map_err(|e| eyre::eyre!(e))?
                        .map_completion_err(|e| eyre::eyre!(e)),
                    )
                }
            }
            PdfConversionMethod::Typst => {
                #[cfg(not(feature = "typst_pdf"))]
                {
                    bail!(r#"The "Typst" library wasn't built when this program was created."#);
                }
                #[cfg(feature = "typst_pdf")]
                {
                    use eyre::Context;
                    use std::io;

                    struct TypstConverter<W>(Vec<u8>, W);
                    impl<W> io::Write for TypstConverter<W> {
                        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                            self.0.extend_from_slice(buf);
                            Ok(buf.len())
                        }

                        fn flush(&mut self) -> std::io::Result<()> {
                            Ok(())
                        }
                    }
                    impl<W> HtmlSink<W, eyre::Error> for TypstConverter<W>
                    where
                        W: WriteBuilder + Send,
                    {
                        fn complete(mut self) -> Result<W, eyre::Error> {
                            let typst_doc = String::from_utf8(self.0)
                                .wrap_err("Invalid UTF8 in typst document")?;

                            log::debug!("Generated Typst document, converting it to a PDF...");

                            let world = crate::typst_world::TypstWrapperWorld::new(
                                "../".to_owned(),
                                typst_doc,
                            );

                            // Render document
                            let document = typst::compile(&world)
                                .output
                                .map_err(|e| eyre::eyre!("{e:?}"))
                                .wrap_err("Error compiling typst")?;

                            // Output to pdf
                            let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
                                .map_err(|e| eyre::eyre!("{e:?}"))
                                .wrap_err("Error exporting Typst document to PDF")?;
                            std::io::copy(&mut pdf.as_slice(), &mut self.1.get_writer()?)
                                .wrap_err("Error writing PDF to output")?;

                            Ok(self.1)
                        }
                    }

                    Box::new(TypstConverter(Vec::new(), output))
                }
            }
            #[cfg(target_family = "wasm")]
            _ => {
                bail!("HTML to PDF converters are not available in WebAssembly");
            }
        })
    }
}
