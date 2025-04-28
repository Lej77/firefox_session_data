use super::{DynStatistics, JSONStatisticsRef};
use std::borrow::Cow;
use std::fmt;

pub trait StatisticsFormatter {
    /// Format a statistics entry.
    fn format_entry(&mut self, f: &mut fmt::Formatter, stats: JSONStatisticsRef) -> fmt::Result;
}
impl<T> StatisticsFormatter for &mut T
where
    T: StatisticsFormatter,
{
    fn format_entry(&mut self, f: &mut fmt::Formatter, stats: JSONStatisticsRef) -> fmt::Result {
        <T as StatisticsFormatter>::format_entry(self, f, stats)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FMTNumber {
    UInt64(u64),
    Double(f64),
}
impl FMTNumber {
    #[cfg(feature = "with_num_format")]
    pub fn write_formatted(
        mut self,
        f: &mut fmt::Formatter,
        locale: num_format::Locale,
    ) -> fmt::Result {
        if let FMTNumber::Double(v) = self {
            if v < 100_f64 || v > (std::u64::MAX as f64) {
                return fmt::Display::fmt(&self, f);
            } else {
                self = FMTNumber::UInt64(v as u64);
            }
        }

        // Create a stack-allocated buffer...
        let mut buf = num_format::Buffer::default();

        // Write a number into the buffer.
        match self {
            FMTNumber::UInt64(v) => {
                buf.write_formatted(&v, &locale);
            }
            FMTNumber::Double(_) => (),
        };

        // Get a view into the buffer as a &str...
        let s = buf.as_str();

        // Write the str to the formatter.
        write!(f, "{}", s)
    }
}
impl fmt::Display for FMTNumber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FMTNumber::UInt64(v) => write!(f, "{}", v),
            FMTNumber::Double(v) => write!(f, "{}", v),
        }
    }
}

/// Indicates a type of information that statistics might provide.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExtraFMTInfo {
    /// Print the name of the `Statistics` type.
    Name,
    /// Print the number of times that an element of this type was encountered.
    Count,
    /// Print the total size of the data.
    Size,
    /// Print the total size divided by the count.
    AverageSize,
    /// Print the average number of values in each element.
    ///
    /// - For arrays: average number of values in each array.
    /// - For objects: average number of properties in each object.
    AverageLength,
    /// Number of booleans that were `true`.
    TrueCount,
    /// Number of booleans that were `false`.
    FalseCount,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormatInfoWithOptions<'a> {
    pub options: &'a InfoFormattingOptions,
    pub info: &'a FMTInfoValue<'a>,
}
impl fmt::Display for FormatInfoWithOptions<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.info {
            FMTInfoValue::Number(v) => {
                #[cfg(feature = "with_num_format")]
                {
                    if let Some(number_locale) = self.options.number_locale {
                        return v.write_formatted(f, number_locale);
                    }
                }
                write!(f, "{}", v)
            }
            FMTInfoValue::Text(text) => write!(f, "{}", text),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InfoFormattingOptions {
    #[cfg(feature = "with_num_format")]
    pub number_locale: Option<num_format::Locale>,
}
impl InfoFormattingOptions {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn format_with<'a>(&'a self, info: &'a FMTInfoValue<'a>) -> FormatInfoWithOptions<'a> {
        FormatInfoWithOptions {
            options: self,
            info,
        }
    }
}

/// The value for some statistics info.
#[derive(Debug, Clone, PartialEq)]
pub enum FMTInfoValue<'a> {
    Number(FMTNumber),
    Text(Cow<'a, str>),
}
impl From<FMTNumber> for FMTInfoValue<'static> {
    fn from(value: FMTNumber) -> Self {
        FMTInfoValue::Number(value)
    }
}
impl<'a> From<Cow<'a, str>> for FMTInfoValue<'a> {
    fn from(value: Cow<'a, str>) -> Self {
        FMTInfoValue::Text(value)
    }
}

/// Values for some statistics info.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatisticsInfoValues<'a> {
    pub name: Option<Cow<'a, str>>,
    pub count: Option<u64>,
    pub size: Option<u64>,
    pub true_count: Option<u64>,
    pub average_length: Option<u64>,
    pub all_elements_have_same_size: bool,
}
impl<'a> StatisticsInfoValues<'a> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn set_name(&mut self, name: impl Into<Cow<'a, str>>) -> &mut Self {
        self.name = Some(name.into());
        self
    }

    pub fn get_info_value(&self, info_type: ExtraFMTInfo) -> Option<FMTInfoValue> {
        use ExtraFMTInfo::*;
        match info_type {
            Name => self
                .name
                .as_ref()
                .map(|v| Cow::Borrowed(&**v))
                .map(Into::into),
            Count => self.count.map(FMTNumber::UInt64).map(Into::into),
            Size => self.size.map(FMTNumber::UInt64).map(Into::into),
            AverageSize => self.average_size().map(FMTNumber::Double).map(Into::into),
            AverageLength => self.average_length.map(FMTNumber::UInt64).map(Into::into),
            TrueCount => self.true_count.map(FMTNumber::UInt64).map(Into::into),
            FalseCount => self.false_count().map(FMTNumber::UInt64).map(Into::into),
        }
    }

    pub fn get_from_boxed_stats(&mut self, stats: &impl DynStatistics) -> &mut Self {
        self.count = Some(stats.boxed_count() as u64);
        self.size = Some(stats.boxed_size());
        self
    }
    pub fn get_from_stats(&mut self, stats: &JSONStatisticsRef) -> &mut Self {
        self.get_from_boxed_stats(stats);

        use JSONStatisticsRef::*;
        match stats {
            JSONValue(_) => (),
            JSONNull(_) => {
                self.set_name("null");
                self.all_elements_have_same_size = true;
            }
            JSONBoolean(stats) => {
                self.set_name("bool");
                self.true_count = Some(stats.true_count as u64);
            }
            JSONNumber(_) => {
                self.set_name("number");
            }
            JSONString(_) => {
                self.set_name("string");
            }
            JSONArray(stats) => {
                self.set_name("array");
                self.average_length =
                    Some((stats.lengths.iter().sum::<usize>() / stats.lengths.len()) as u64);
            }
            JSONObject(stats) => {
                self.set_name("object");
                self.average_length = Some(
                    (stats.properties_count.iter().sum::<usize>() / stats.properties_count.len())
                        as u64,
                );
            }
            JSONObjectProperty(_) => (),
        }
        self
    }
    pub fn average_size(&self) -> Option<f64> {
        if self.all_elements_have_same_size {
            None
        } else {
            Some((self.size? as f64) / (self.count? as f64))
        }
    }
    pub fn false_count(&self) -> Option<u64> {
        Some(self.count? - self.true_count?)
    }
}

pub struct StatisticsInfoTypeNames {
    pub is_object: bool,
}
impl StatisticsInfoTypeNames {
    pub fn get_name(&self, info_type: ExtraFMTInfo) -> Cow<'static, str> {
        match info_type {
            ExtraFMTInfo::Name => "name".into(),
            ExtraFMTInfo::Count => "count".into(),
            ExtraFMTInfo::AverageLength => {
                if self.is_object {
                    "average properties".into()
                } else {
                    "average length".into()
                }
            }
            ExtraFMTInfo::AverageSize => "average size".into(),
            ExtraFMTInfo::Size => "size".into(),
            ExtraFMTInfo::TrueCount => "true".into(),
            ExtraFMTInfo::FalseCount => "false".into(),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct StandardStatisticsFormatter<'a> {
    pub surround_infos: Option<(Cow<'a, str>, Cow<'a, str>)>,
    pub indents: u32,
    pub indent_text: Cow<'a, str>,
    pub infos_to_print: Option<Cow<'a, [ExtraFMTInfo]>>,
    pub format_options: InfoFormattingOptions,
}
impl<'a> StandardStatisticsFormatter<'a> {
    /// Create the simplest possible config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a good standard config for printing.
    #[allow(unused_mut)]
    pub fn standard() -> Self {
        let mut format_options = InfoFormattingOptions::new();
        #[cfg(feature = "with_num_format")]
        {
            format_options.number_locale = Some(num_format::Locale::en);
        }
        Self {
            surround_infos: Some(("(".into(), ")".into())),
            indents: 0,
            indent_text: "  ".into(),
            infos_to_print: Some(Cow::Borrowed(&[
                ExtraFMTInfo::Name,
                ExtraFMTInfo::Count,
                ExtraFMTInfo::Size,
                ExtraFMTInfo::AverageSize,
                ExtraFMTInfo::AverageLength,
                ExtraFMTInfo::TrueCount,
                ExtraFMTInfo::FalseCount,
            ])),
            format_options,
        }
    }

    /// Get a new options struct that borrows the content of this one.
    pub fn as_borrowed<'b>(&'b self) -> StandardStatisticsFormatter<'b>
    where
        'b: 'a,
    {
        StandardStatisticsFormatter {
            surround_infos: self
                .surround_infos
                .as_ref()
                .map(|(a, b)| (Cow::Borrowed(&**a), Cow::Borrowed(&**b))),
            indents: self.indents,
            indent_text: Cow::Borrowed(&*self.indent_text),
            infos_to_print: self.infos_to_print.as_ref().map(|v| Cow::Borrowed(&**v)),
            format_options: self.format_options.clone(),
        }
    }
    /// Set the text that should be written before and after info entries.
    pub fn set_surround_info(
        &mut self,
        before: impl Into<Cow<'a, str>>,
        after: impl Into<Cow<'a, str>>,
    ) {
        self.surround_infos = Some((before.into(), after.into()));
    }

    fn write_indents(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for _ in 0..self.indents {
            write!(f, "{}", self.indent_text)?;
        }
        Ok(())
    }
    fn write_new_line(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f)?;
        self.write_indents(f)
    }
    fn write_property_name(&self, f: &mut fmt::Formatter, name: &str) -> fmt::Result {
        write!(f, "\"{}\": ", name)
    }
}
impl StatisticsFormatter for StandardStatisticsFormatter<'_> {
    fn format_entry(&mut self, f: &mut fmt::Formatter, stats: JSONStatisticsRef) -> fmt::Result {
        if let Some(infos_to_print) = &self.infos_to_print {
            let mut info = StatisticsInfoValues::new();
            info.get_from_stats(&stats);

            if info.name.is_some() {
                // Should probably print info:
                for info_type in infos_to_print.iter().cloned() {
                    if let Some(info_value) = info.get_info_value(info_type) {
                        let printable_info_value = self.format_options.format_with(&info_value);
                        if let ExtraFMTInfo::Name = info_type {
                            write!(f, "{} ", printable_info_value)?;
                        } else {
                            let info_type_name = StatisticsInfoTypeNames {
                                is_object: matches!(stats, JSONStatisticsRef::JSONObject(_)),
                            }
                            .get_name(info_type);
                            write!(f, "({}: {}) ", info_type_name, printable_info_value)?;
                        }
                    }
                }
            }
        }

        use JSONStatisticsRef::*;
        match stats {
            JSONValue(stats) => {
                // Don't print info for types that never occurred (count === 0):
                for (printed_lines, field) in stats
                    .all_fields()
                    .iter()
                    .filter(|field| field.boxed_count() > 0)
                    .enumerate()
                {
                    if printed_lines > 0 {
                        self.write_new_line(f)?;
                    }
                    self.format_entry(f, *field)?;
                }
            }
            JSONNull(_) => (),
            JSONBoolean(_) => (),
            JSONNumber(_) => (),
            JSONString(_) => (),
            JSONArray(stats) => {
                if let Some(values) = &stats.values {
                    let mut nested_options = self.as_borrowed();
                    nested_options.indents += 1;

                    nested_options.write_new_line(f)?;

                    nested_options.format_entry(f, (&**values).into())?;
                }
            }
            JSONObject(stats) => {
                let mut nested_options = self.as_borrowed();
                nested_options.indents += 1;

                for (key, value) in stats.properties.iter() {
                    nested_options.write_new_line(f)?;
                    nested_options.write_property_name(f, key)?;

                    nested_options.indents += 1;
                    nested_options.write_new_line(f)?;
                    nested_options.format_entry(f, (value).into())?;
                    nested_options.indents -= 1;
                }
            }
            JSONObjectProperty(stats) => {
                self.format_entry(f, (&stats.value_info).into())?;
            }
        }
        Ok(())
    }
}
