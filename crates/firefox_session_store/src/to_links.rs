//! Create text documents with links from Firefox sessionstore files.

#![allow(clippy::write_literal)]

macro_rules! concat_ln {
    ( $( $text:expr ),* ) => {
        concat!($(
            $text, "\n"
        ),*)
    };
}

pub mod simple_html {
    use std::fmt;
    use std::io::{self, Write};
    use std::marker::PhantomData;

    pub trait HTMLWriterState {
        fn write_tag<W: Write>(writer: &mut W, start: bool) -> io::Result<()> {
            write!(writer, "<{}{}>", if start { "" } else { "/" }, Self::tag())
        }
        fn tag() -> &'static str;
    }

    pub struct HTMLWriter<W: Write, M: HTMLWriterState>(Option<W>, PhantomData<M>);
    impl<W: Write, M: HTMLWriterState> HTMLWriter<W, M> {
        fn writer(&mut self) -> &mut W {
            self.0.as_mut().unwrap()
        }
    }

    impl<W: Write, M: HTMLWriterState> Write for HTMLWriter<W, M> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.writer().write(buf)
        }

        fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
            self.writer().write_vectored(bufs)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.writer().flush()
        }
        fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
            self.writer().write_all(buf)
        }
        fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
            self.writer().write_fmt(fmt)
        }
    }

    pub enum Header {}
    impl HTMLWriterState for Header {
        fn tag() -> &'static str {
            "head"
        }
    }
    pub enum Body {}
    impl HTMLWriterState for Body {
        fn tag() -> &'static str {
            "body"
        }
    }

    impl<W: Write> HTMLWriter<W, Header> {
        pub fn start_header(mut writer: W) -> io::Result<HTMLWriter<W, Header>> {
            // write!(writer, "<!DOCTYPE html>")?;
            write!(writer, "{}", "<html>")?;
            Header::write_tag(&mut writer, true)?;
            Ok(Self(Some(writer), Default::default()))
        }
    }
    impl<W: Write> HTMLWriter<W, Header> {
        pub fn start_body(mut self) -> io::Result<HTMLWriter<W, Body>> {
            Header::write_tag(&mut self.writer(), false)?;
            Body::write_tag(&mut self.writer(), true)?;
            Ok(HTMLWriter(self.0.take(), Default::default()))
        }
    }
    impl<W: Write> HTMLWriter<W, Body> {
        pub fn finish(mut self) -> io::Result<W> {
            Body::write_tag(&mut self.writer(), false)?;
            write!(self.writer(), "{}", "</html>")?;
            Ok(self.0.take().unwrap())
        }
    }
    impl<W: Write, M: HTMLWriterState> Drop for HTMLWriter<W, M> {
        fn drop(&mut self) {
            if self.0.is_none() {
                return;
            }

            M::write_tag(&mut self.writer(), false).ok();
            write!(self.writer(), "{}", "</html>").ok();
        }
    }

    pub fn html_escaped_text(text: &str) -> String {
        // https://stackoverflow.com/questions/7381974/which-characters-need-to-be-escaped-in-html
        // Note that the order of replacement is important. You must replace & first.
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }

    pub const fn html_horizontal_line() -> &'static str {
        "<hr />"
    }
}

pub mod simple_rtf {
    use std::fmt;
    use std::io::{self, Write};

    pub struct RTFWriter<W: Write>(Option<W>);
    impl<W: Write> Write for RTFWriter<W> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.writer().write(buf)
        }

        fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
            self.writer().write_vectored(bufs)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.writer().flush()
        }
        fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
            self.writer().write_all(buf)
        }
        fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
            self.writer().write_fmt(fmt)
        }
    }
    impl<W: Write> RTFWriter<W> {
        pub fn start(mut writer: W) -> io::Result<Self> {
            writeln!(
                writer,
                "{}",
                r"{\rtf1\ansi\ansicpg1252\deff0\nouicompat\deflang1053{\fonttbl{\f0\fnil\fcharset0 Calibri;}}"
            )?;
            writeln!(writer, "{}", r"{\colortbl;\red0\green0\blue255; }")?;
            writeln!(
                writer,
                "{}",
                r"{\*\generator Riched20 10.0.14393}\viewkind4\uc1"
            )?;
            writeln!(writer, "{}", r"\pard\sa200\sl276\slmult1\f0\fs22\lang29")?;

            Ok(Self(Some(writer)))
        }
        fn writer(&mut self) -> &mut W {
            self.0.as_mut().unwrap()
        }
        fn write_end(&mut self) -> io::Result<()> {
            write!(self.writer(), "{}", "}")
        }
        pub fn finish(mut self) -> io::Result<W> {
            self.write_end()?;
            Ok(self.0.take().unwrap())
        }
    }
    impl<W: Write> Drop for RTFWriter<W> {
        fn drop(&mut self) {
            if self.0.is_none() {
                return;
            }

            self.write_end().ok();
        }
    }

    pub fn rtf_horizontal_line(use_picture: bool) -> &'static str {
        if !use_picture {
            r"\par"
        } else {
            concat_ln!(
                r#"{\pict{\*\picprop}\wmetafile8\picw1764\pich882\picwgoal9070\pichgoal30"#,
                "010009000003c902000006000602000000000602000026060f000204574d464301000000000001",
                "00d02e0000000001000000e003000000000000e0030000010000006c000000ffffffffffffffff",
                "c60e0000340000000000000000000000873e0000d400000020454d4600000100e00300001b0000",
                "0003000000000000000000000000000000cc120000ea190000cc00000019010000000000000000",
                "000000000000bc1b030007490400160000000c000000180000000a000000100000000000000000",
                "0000000900000010000000c60e000034000000520000007001000001000000a4ffffff00000000",
                "0000000000000000900100000000000004400022430061006c0069006200720069000000000000",
                "000000000000000000000000000000000000000000000000000000000000000000000000000000",
                "0000000000000000b20023741a51a840c90225000000689fb200ac9fb2001000000010a3b20090",
                "a0b2001a4e5d3210a3b20008a0b2001000000078a1b200f4a2b200ec4d5d3210a3b20008a0b200",
                "20000000b4731a5108a0b20010a3b20020000000fffffffffc021503fb731a51ffffffffffff01",
                "80ffff0180afff0180ffffffff0004010000000000000800009870390301000000250000005802",
                "000025000000372e90010000020f0502020204030204ff2a00e07b2400c00900000000000000ff",
                "01000000000000430061006c0069006200720069000000cca0b200c4281a5130fefb512ca4b200",
                "010000003ca0b200cb311351290000000100000078a0b20078a0b2006476000800000000250000",
                "000c00000001000000250000000c00000001000000250000000c00000001000000120000000c00",
                "000001000000180000000c00000000000002540000005400000000000000000000003500000031",
                "000000010000002da987404e8b87400000000057000000010000004c0000000400000000000000",
                "00000000c50e00003200000050000000200000003600000046000000280000001c000000474449",
                "4302000000ffffffffffffffffc70e000035000000000000002100000008000000620000000c00",
                "00000100000024000000240000000000003e00000000000000000000003e000000000000000002",
                "00000027000000180000000200000000000000a0a0a00000000000250000000c00000002000000",
                "250000000c000000080000805600000030000000ffffffffffffffffc60e000034000000050000",
                "00f9fffcfff9ff9d012d769d012d76fcfff9fffcff250000000c00000007000080250000000c00",
                "000000000080240000002400000000000041000000000000000000000041000000000000000002",
                "000000220000000c000000ffffffff460000001400000008000000474449430300000025000000",
                "0c0000000e000080250000000c0000000e0000800e000000140000000000000010000000140000",
                "000400000003010800050000000b0200000000050000000c0208005102040000002e0118001c00",
                "0000fb02f2ff0000000000009001000000000440002243616c6962726900000000000000000000",
                "000000000000000000000000000000040000002d010000040000002d010000040000002d010000",
                "0400000002010100050000000902000000020d000000320a0e0000000100040000000000520208",
                "0020000800030000001e0007000000fc020000a0a0a0000000040000002d01010008000000fa02",
                "050000000000ffffff00040000002d0102000e0000002403050000000000000008005202080052",
                "0200000000000008000000fa0200000000000000000000040000002d01030007000000fc020000",
                "ffffff000000040000002d010400040000002701ffff1c000000fb020300010000000000bc0200",
                "0000000102022253797374656d00043f3f14000100540000003f3f3f3f00000000000000000000",
                "040000002d010500040000002d010500030000000000",
                "}"
            )
        }
    }
}

mod simple_typst {

    pub fn typst_escaped_text(text: &str) -> String {
        // https://typst.app/docs/reference/foundations/str/
        text.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }
}

use super::session_info::{TabGroup, TreeDataSource};
use either::*;
use simple_html::{html_escaped_text, html_horizontal_line, HTMLWriter};
use simple_rtf::{rtf_horizontal_line, RTFWriter};
use simple_typst::typst_escaped_text;
use std::{
    borrow::Cow,
    io::{self, Write},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinkFormat {
    #[default]
    TXT,
    RTF {
        picture_horizontal_line: bool,
    },
    HTML,
    Markdown,
    Typst,
}
impl LinkFormat {
    #[must_use]
    pub fn is_html(self) -> bool {
        self == LinkFormat::HTML
    }
    #[must_use]
    pub fn is_rtf(self) -> bool {
        matches!(self, LinkFormat::RTF { .. })
    }
    #[must_use]
    pub fn rtf_picture_horizontal_line(self) -> bool {
        if let LinkFormat::RTF {
            picture_horizontal_line,
        } = self
        {
            picture_horizontal_line
        } else {
            false
        }
    }
    #[must_use]
    pub fn is_txt(self) -> bool {
        self == LinkFormat::TXT
    }
    #[must_use]
    pub fn is_markdown(self) -> bool {
        self == LinkFormat::Markdown
    }
    #[must_use]
    pub fn is_typst(&self) -> bool {
        matches!(self, Self::Typst)
    }

    pub fn line_break(self) -> &'static str {
        match self {
            LinkFormat::TXT | LinkFormat::Markdown => "\n",
            LinkFormat::RTF { .. } => concat!(r#"\line"#, "\n"),
            LinkFormat::HTML => concat!("<br />", "\n"),
            LinkFormat::Typst => "\n",
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ToLinksOptions<'a> {
    pub format: LinkFormat,
    /// Make page breaks between different windows' tabs. This is not supported
    /// for all formats (in which case the windows' tabs will be appended after
    /// each other without separation).
    pub page_breaks_after_group: bool,
    pub skip_page_break_after_last_group: bool,
    pub table_of_contents: bool,
    pub indent_all_links: bool,
    pub custom_page_break: Cow<'a, str>,
    pub tree_sources: Cow<'a, [TreeDataSource]>,
}
impl ToLinksOptions<'_> {
    #[allow(clippy::cognitive_complexity)]
    pub fn write_links<W: Write>(&self, groups: &[TabGroup<'_>], writer: &mut W) -> io::Result<()> {
        const HTML_GROUP_TAG: &str = "p";

        // -------------------------------------
        //            Format header
        // -------------------------------------

        let mut writer = match self.format {
            LinkFormat::TXT | LinkFormat::Markdown => Left(writer),
            LinkFormat::RTF { .. } => Right(Left(RTFWriter::start(writer)?)),
            LinkFormat::HTML => {
                let mut writer = HTMLWriter::start_header(writer)?;
                writeln!(writer, r#"<meta charset="UTF-8" />"#)?; // <-- Specify that the page is UTF-8 encoded

                if self.page_breaks_after_group {
                    writeln!(writer, "{}", "<STYLE TYPE=\"text/css\">")?;
                    write!(writer, "{}", HTML_GROUP_TAG)?;
                    writeln!(writer, "{}", " {page-break-after: always}")?;
                    writeln!(writer, "{}", "</STYLE>")?;
                }

                Right(Right(writer.start_body()?))
            }
            LinkFormat::Typst => {
                writeln!(writer, "#show link: underline")?;
                writeln!(writer, "#show link: set text(blue)")?;
                writeln!(writer, "\n")?;
                Left(writer)
            }
        };

        // -------------------------------------
        //             Helper Macro
        // -------------------------------------

        let line_break = self.format.line_break();

        macro_rules! writer {
            ("") => {
                write!(writer, "{}", line_break)?;
            };
            ( $( $token:tt )* ) => {
                write!(writer, $( $token )* )?;
                writer!("");
            };
        }

        // -------------------------------------
        //          Table of contents
        // -------------------------------------

        if self.table_of_contents {
            match self.format {
                LinkFormat::HTML => {
                    writer!("<h2>{}</h2>", html_escaped_text("Contents"));

                    for (index, group) in groups.iter().enumerate() {
                        writer!(r##"<a href="#group{}">{}</a>"##, index + 1, group.name());
                    }
                    writeln!(writer, "<{}>", HTML_GROUP_TAG)?;
                    writeln!(writer, "</{}>", HTML_GROUP_TAG)?;
                }
                LinkFormat::Markdown => {
                    writer!("");
                    writer!("# Contents");
                    writer!("");

                    for group in groups {
                        writer!("{}", group.name());
                        writer!("");
                    }

                    writer!("");
                }
                LinkFormat::Typst => {
                    writer!("#outline()");
                    writer!("");
                }
                LinkFormat::TXT | LinkFormat::RTF { .. } => {
                    writer!("Contents");
                    writer!("");
                    writer!("");

                    for group in groups {
                        writer!("{}", group.name());
                    }

                    writer!("");
                    if self.format.is_rtf() {
                        writer!(
                            "{}",
                            rtf_horizontal_line(self.format.rtf_picture_horizontal_line())
                        );
                    }
                    writer!("");
                    writer!("");
                    writer!("");
                }
            }

            // Page break:

            if !self.custom_page_break.is_empty() {
                // This will produce a custom page break:
                writer!("{}", self.custom_page_break);
            }
            if self.page_breaks_after_group {
                if self.format.is_typst() {
                    writer!("#pagebreak()");
                    writer!("");
                }
            } else {
                // If we aren't doing page breaks after group then add some empty lines and possibly horizontal lines:
                writer!("");
                writer!("");
                if self.format.is_rtf() {
                    writer!(
                        "{}",
                        rtf_horizontal_line(self.format.rtf_picture_horizontal_line())
                    );
                };
                writer!("");

                if self.format.is_html() {
                    writer!("{}", html_horizontal_line());
                } else if self.format.is_typst() {
                    writer!("#line(length: 100%)");
                }

                writer!("");
                writer!("");
            }
        }

        // -------------------------------------
        //                Links
        // -------------------------------------

        let tree_source = self
            .tree_sources
            .iter()
            .find(|s| {
                s.has_any_data(
                    groups
                        .iter()
                        .flat_map(|group| group.tabs().iter())
                        .map(|tab_info| tab_info.data),
                )
            })
            .map(|source| std::array::from_ref(source) as &[_])
            .unwrap_or(&[]);

        for (group_index, group) in groups.iter().enumerate() {
            match self.format {
                LinkFormat::TXT | LinkFormat::RTF { .. } => {
                    writer!("{}", group.name());
                    if self.format.is_rtf() {
                        writer!("");
                    }
                }
                LinkFormat::HTML => {
                    writer!(
                        r#"<a name="group{}"></a><h2>{}</h2>"#,
                        group_index + 1,
                        html_escaped_text(group.name())
                    );
                }
                LinkFormat::Markdown => {
                    writer!("# {}", group.name());
                }
                LinkFormat::Typst => {
                    writer!("= {}\n", group.name());
                }
            }

            for tab in group.tabs() {
                if tab.data.entries.is_empty() {
                    // Can have 0 entries! Why?
                    continue;
                }
                let url = tab.url();
                let mut title = tab.title();
                if title.is_empty() {
                    title = "No title";
                }

                let number_of_tree_style_tab_parents = tab
                    .tst_ancestor_tabs(
                        tree_source,
                        tab.window.expect("tab should have an associated window"),
                    )
                    .count();

                let mut tab_tree_indention = "".to_owned();

                if self.indent_all_links {
                    tab_tree_indention += match self.format {
                        LinkFormat::HTML => "&nbsp;&nbsp;&nbsp;&nbsp;",
                        LinkFormat::RTF { .. } => "  ",
                        LinkFormat::TXT => "    ",
                        LinkFormat::Markdown => "  ",
                        LinkFormat::Typst => "",
                    };
                }

                let mut tab_tree_indention_main = tab_tree_indention.clone();

                for index in 0..number_of_tree_style_tab_parents {
                    if index + 1 == number_of_tree_style_tab_parents {
                        // Last indentation:
                        let extra = match self.format {
                            LinkFormat::Markdown => "",
                            LinkFormat::RTF { .. } | LinkFormat::HTML => "|---",
                            LinkFormat::TXT => "|--- ",
                            LinkFormat::Typst => "- ",
                        };
                        tab_tree_indention_main = tab_tree_indention.clone() + extra;
                    }

                    tab_tree_indention += match self.format {
                        LinkFormat::Markdown => "  ",
                        LinkFormat::HTML => "|&nbsp;&nbsp;&nbsp;&nbsp;",
                        LinkFormat::RTF { .. } => "|  ",
                        LinkFormat::TXT => "|    ",
                        LinkFormat::Typst => "  ",
                    };
                }

                let mut scroll = tab.scroll().unwrap_or_default().to_owned();
                if !scroll.is_empty() {
                    scroll = format!(" (scroll: {})", scroll);
                }

                if url.starts_with("about:") {
                    writer!("{}", tab_tree_indention);
                    match self.format {
                        LinkFormat::HTML => {
                            // writer!("{}", html_horizontal_line());
                        }
                        LinkFormat::RTF { .. } => {
                            // writer!("{}", rtf_horizontal_line(self.format.rtf_picture_horizontal_line()));
                        }
                        LinkFormat::TXT => {
                            writer!(
                                "{}{}",
                                tab_tree_indention_main,
                                "--------------------------------------------------------------"
                            );
                        }
                        LinkFormat::Markdown => {}
                        LinkFormat::Typst => {}
                    }
                }

                if url != "about:newtab" {
                    match self.format {
                        LinkFormat::HTML => {
                            writer!(
                                r#"{}<a href="{}">{}</a>{}"#,
                                tab_tree_indention_main,
                                html_escaped_text(url),
                                html_escaped_text(title),
                                scroll
                            );
                        }
                        LinkFormat::RTF { .. } => {
                            writer!(
                                "{}{}{}{}{}{}{}",
                                tab_tree_indention_main,
                                r#"{\field{\*\fldinst HYPERLINK ""#,
                                url,
                                r#""}{\fldrslt "#,
                                title,
                                "}}",
                                scroll
                            );
                        }
                        LinkFormat::TXT => {
                            writer!("{}", tab_tree_indention);
                            writer!("{}{}{}", tab_tree_indention_main, title, scroll);
                            writer!("{}{}", tab_tree_indention, url);
                        }
                        LinkFormat::Markdown => {
                            writer!(
                                "{}- [{}]({}){}",
                                tab_tree_indention_main,
                                // TODO: escape markdown link TITLE:
                                title,
                                // TODO: escape markdown URL:
                                url,
                                scroll
                            );
                        }
                        LinkFormat::Typst => {
                            // https://typst.app/docs/reference/model/link/
                            writer!(
                                "{}#link(\"{}\", \"{}\"){}\n",
                                tab_tree_indention_main,
                                typst_escaped_text(url),
                                typst_escaped_text(title),
                                scroll
                            );
                        }
                    }
                }
            }

            let skip_page_break =
                self.skip_page_break_after_last_group && group_index + 1 == groups.len();

            if !skip_page_break && self.custom_page_break.is_empty() {
                if self.page_breaks_after_group {
                    if self.format.is_typst() {
                        writer!("#pagebreak()\n\n");
                    }
                } else {
                    // If we aren't doing page breaks after group then add some empty lines and possibly horizontal lines:
                    writer!("");
                    writer!("");
                    if self.format.is_rtf() {
                        writer!(
                            "{}",
                            rtf_horizontal_line(self.format.rtf_picture_horizontal_line())
                        );
                    };
                    writer!("");

                    if self.format.is_html() {
                        writer!("{}", html_horizontal_line());
                    } else if self.format.is_typst() {
                        writer!("#line(length: 100%)");
                    }

                    writer!("");
                    writer!("");
                }
            }

            if self.format.is_html() {
                write!(writer, "<{}>", HTML_GROUP_TAG)?;
                write!(writer, "</{}>", HTML_GROUP_TAG)?;
            }
            if !self.custom_page_break.is_empty() && !skip_page_break {
                // This will produce a custom page break:
                writer!("{}", self.custom_page_break);
            }
        }

        // -------------------------------------
        //             Format footer
        // -------------------------------------

        // Write end tabs for some formats (this will otherwise be done when the writer is dropped but that will silently ignore any errors):
        match writer {
            Left(v) => v,
            Right(Left(v)) => v.finish()?,
            Right(Right(v)) => v.finish()?,
        };

        Ok(())
    }
}
