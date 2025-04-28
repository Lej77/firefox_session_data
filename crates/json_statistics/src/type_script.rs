use std::{borrow::Cow, fmt};

use crate::{
    print::StatisticsFormatter, DynStatistics, JSONStatisticsRef, JSONValueStatistics, Statistics,
};

#[derive(Default, Debug, Clone)]
pub struct TypeScriptStatisticsFormatter<'a> {
    pub exported_type_name: Option<Cow<'a, str>>,
    pub indents: u32,
    pub indent_text: Cow<'a, str>,
    /// The number of times the parent object existed.
    pub parent_count: Option<u32>,
    pub max_object_keys: u32,
}
impl<'a> TypeScriptStatisticsFormatter<'a> {
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
    /// Get a new options struct that borrows the content of this one.
    fn as_borrowed(&self) -> TypeScriptStatisticsFormatter<'_> {
        TypeScriptStatisticsFormatter {
            exported_type_name: None,
            indents: self.indents,
            indent_text: Cow::Borrowed(&self.indent_text),
            parent_count: self.parent_count,
            max_object_keys: self.max_object_keys,
        }
    }
}
impl<'a> StatisticsFormatter for TypeScriptStatisticsFormatter<'a> {
    fn format_entry(&mut self, f: &mut fmt::Formatter, stats: JSONStatisticsRef) -> fmt::Result {
        if let Some(name) = &self.exported_type_name {
            write!(f, "export type {name} = ")?;
        }
        match stats {
            JSONStatisticsRef::JSONNull(_) => write!(f, "null")?,
            JSONStatisticsRef::JSONBoolean(_) => write!(f, "boolean")?,
            JSONStatisticsRef::JSONNumber(_) => write!(f, "number")?,
            JSONStatisticsRef::JSONString(_) => write!(f, "string")?,
            JSONStatisticsRef::JSONArray(info) => {
                if let Some(values) = info.values.as_deref() {
                    if values.count() == 0 {
                        write!(f, "[]")?;
                    } else {
                        write!(f, "(")?;
                        self.as_borrowed().format_entry(f, values.into())?;
                        write!(f, ")[]")?;
                    }
                } else {
                    write!(f, "any[]")?;
                }
            }
            JSONStatisticsRef::JSONObject(obj_info) => {
                write!(f, "{{")?;
                if obj_info.properties.len() > self.max_object_keys as usize {
                    write!(f, " [key: string]: ")?;
                    let mut stats = JSONValueStatistics::default();
                    for prop in obj_info.properties.values() {
                        stats.merge(Cow::Borrowed(&prop.value_info));
                    }
                    self.as_borrowed()
                        .format_entry(f, JSONStatisticsRef::from(&stats))?;
                    write!(f, " ")?;
                } else {
                    self.indents += 1;
                    for (name, prop) in &obj_info.properties {
                        let optional = prop.count() < obj_info.count();
                        self.write_new_line(f)?;
                        let surround = if name
                            .as_bytes()
                            .first()
                            .map_or(false, |&c| c.is_ascii_alphabetic() || c == b'_')
                            && name
                                .as_bytes()
                                .iter()
                                .all(|&c| c.is_ascii_alphanumeric() || c == b'_')
                        {
                            ""
                        } else {
                            "\""
                        };
                        let optional = if optional { "?" } else { "" };
                        write!(
                            f,
                            "{surround}{}{surround}{optional}: ",
                            name.replace('"', "\\\"")
                        )?;
                        self.as_borrowed()
                            .format_entry(f, From::from(&prop.value_info))?;
                        write!(f, ";")?;
                    }
                    self.indents -= 1;
                    self.write_new_line(f)?;
                }
                write!(f, "}}")?;
            }
            JSONStatisticsRef::JSONObjectProperty(info) => {
                self.as_borrowed()
                    .format_entry(f, From::from(&info.value_info))?;
            }
            // Can have different types:
            JSONStatisticsRef::JSONValue(info) => {
                for (i, variant) in info
                    .all_fields()
                    .into_iter()
                    .filter(|info| info.boxed_count() != 0)
                    .enumerate()
                {
                    if i != 0 {
                        write!(f, " | ")?;
                    }
                    self.as_borrowed().format_entry(f, variant)?;
                }
            }
        }
        if self.exported_type_name.is_some() {
            write!(f, ";")?;
        }
        Ok(())
    }
}
