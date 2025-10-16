use either::Either;
use serde_json::{Map, Number, Value};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;

pub mod print;
pub mod type_script;

use print::{StandardStatisticsFormatter, StatisticsFormatter};

macro_rules! define_ref_enum {
    ($(#[$($token:tt)*])* $visible:vis enum $name:ident<$life:lifetime> { $( $(#[$($variant_token:tt)*])* $variant_name:ident($variant_type:ty) ),* $(,)? }) => {
        $(#[$($token)*])*
        $visible enum $name<$life> {
            $(
                $(#[$($variant_token)*])*
                $variant_name($variant_type),
            )*
        }
        impl<$life> $name<$life> {
            pub fn with_formatter<'f>(self, stat_formatter: impl StatisticsFormatter + 'f) -> impl fmt::Display + 'f where $life: 'f {
                struct Helper<'a, F>(RefCell<F>, JSONStatisticsRef<'a>);
                impl<F> fmt::Display for Helper<'_, F> where F: StatisticsFormatter {
                    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        self.0.borrow_mut().format_entry(f, self.1)
                    }
                }
                Helper(RefCell::new(stat_formatter), self)
            }
        }
        impl<$life> DynStatistics for $name<$life> {
            fn boxed_size(&self) -> u64 {
                match self {
                    $($name::$variant_name(v) => v.boxed_size(),)*
                }
            }

            fn boxed_count(&self) -> usize {
                match self {
                    $($name::$variant_name(v) => v.boxed_count(),)*
                }
            }
        }
        $(
            impl<$life> From<$variant_type> for $name<$life> {
                fn from(value: $variant_type) -> Self {
                    $name::$variant_name(value)
                }
            }
        )*
    };
}
define_ref_enum! {
    #[derive(Debug, Clone, Copy)]
    pub enum JSONStatisticsRef<'a> {
        JSONValue(&'a JSONValueStatistics),
        JSONNull(&'a JSONNullStatistics),
        JSONBoolean(&'a JSONBooleanStatistics),
        JSONNumber(&'a JSONNumberStatistics),
        JSONString(&'a JSONStringStatistics),
        JSONArray(&'a JSONArrayStatistics),
        JSONObject(&'a JSONObjectStatistics),
        JSONObjectProperty(&'a JSONObjectPropertyStatistics),
    }
}
impl<'a> fmt::Display for JSONStatisticsRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        StandardStatisticsFormatter::standard().format_entry(f, *self)
    }
}

pub trait Statistics: Clone + fmt::Display {
    /// The total size in number of characters for the data that the statistics were gathered for.
    fn size(&self) -> u64;

    /// The number of times a value was added to the statistics.
    fn count(&self) -> usize;

    /// Merge other statistics data into the current statistics data.
    fn merge(&mut self, data: Cow<Self>);
}
pub trait DynStatistics: fmt::Display {
    /// The total size in number of characters for the data that the statistics were gathered for.
    fn boxed_size(&self) -> u64;

    /// The number of times a value was added to the statistics.
    fn boxed_count(&self) -> usize;
}
impl<S> DynStatistics for S
where
    S: Statistics,
{
    fn boxed_size(&self) -> u64 {
        Statistics::size(self)
    }

    fn boxed_count(&self) -> usize {
        Statistics::count(self)
    }
}

macro_rules! define_union_struct {
    (@zero $($token:tt)*) => {0};
    ($(#[$($token:tt)*])* $struct_vis:vis struct $name:ident { $($(#[$($field_token:tt)*])* $field_vis:vis $field_name:ident: $field_type:ty),* $(,)? }) => {
        $(#[$($token)*])*
        $struct_vis struct $name {
            $(
                $(#[$($field_token)*])*
                $field_vis $field_name: $field_type,
            )*
        }
        impl $name {
            /// An array with all types that this value can contain.
            pub fn all_fields(&self) -> [JSONStatisticsRef<'_>; 0 $(+ 1 + define_union_struct!(@zero $field_name))*] {
                [
                    $(JSONStatisticsRef::from(&self.$field_name)),*
                ]
            }
        }
        impl Statistics for $name {
            fn size(&self) -> u64 {
                $(self.$field_name.size() +)* 0
            }
            fn count(&self) -> usize {
                $(self.$field_name.count() +)* 0
            }
            fn merge(&mut self, data: Cow<Self>) {
                match data {
                    Cow::Borrowed(v) => {
                        $(self.$field_name.merge(Cow::Borrowed(&v.$field_name));)*
                    },
                    Cow::Owned(v) => {
                        $(self.$field_name.merge(Cow::Owned(v.$field_name));)*
                    },
                }
            }
        }
    };
}
define_union_struct! {
    #[derive(Default, Debug, Clone)]
    pub struct JSONValueStatistics {
        pub nulls: JSONNullStatistics,
        pub booleans: JSONBooleanStatistics,
        pub numbers: JSONNumberStatistics,
        pub strings: JSONStringStatistics,
        pub arrays: JSONArrayStatistics,
        pub objects: JSONObjectStatistics,
    }
}
impl JSONValueStatistics {
    pub fn add_value(&mut self, value: &Value) {
        match value {
            Value::Null => self.nulls.add_null(),
            Value::Bool(v) => self.booleans.add_bool(*v),
            Value::Number(v) => self.numbers.add_number(v),
            Value::String(v) => self.strings.add_string(v),
            Value::Array(v) => self.arrays.add_array(v),
            Value::Object(v) => self.objects.add_object(v),
        }
    }
    pub fn with_formatter<'f>(
        &'f self,
        stat_formatter: impl StatisticsFormatter + 'f,
    ) -> impl fmt::Display + 'f {
        JSONStatisticsRef::from(self).with_formatter(stat_formatter)
    }
}
impl fmt::Display for JSONValueStatistics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        StandardStatisticsFormatter::standard().format_entry(f, self.into())
    }
}

#[derive(Default, Debug, Clone)]
pub struct JSONNullStatistics {
    pub count: usize,
}
impl JSONNullStatistics {
    pub fn add_null(&mut self) {
        self.count += 1;
    }
}
impl Statistics for JSONNullStatistics {
    fn size(&self) -> u64 {
        (self.count as u64) * 4
    }
    fn count(&self) -> usize {
        self.count
    }
    fn merge(&mut self, data: Cow<Self>) {
        self.count += data.count;
    }
}
impl fmt::Display for JSONNullStatistics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        StandardStatisticsFormatter::standard().format_entry(f, self.into())
    }
}

#[derive(Default, Debug, Clone)]
pub struct JSONBooleanStatistics {
    pub false_count: usize,
    pub true_count: usize,
}
impl JSONBooleanStatistics {
    pub fn add_bool(&mut self, value: bool) {
        if value {
            self.true_count += 1;
        } else {
            self.false_count += 1;
        }
    }
}
impl Statistics for JSONBooleanStatistics {
    fn size(&self) -> u64 {
        (self.false_count as u64) * 5 + (self.true_count as u64) * 4
    }
    fn count(&self) -> usize {
        self.false_count + self.true_count
    }
    fn merge(&mut self, data: Cow<Self>) {
        self.false_count += data.false_count;
        self.true_count += data.true_count;
    }
}
impl fmt::Display for JSONBooleanStatistics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        StandardStatisticsFormatter::standard().format_entry(f, self.into())
    }
}

#[derive(Default, Debug, Clone)]
pub struct JSONNumberStatistics {
    /// The sizes in characters of the encountered values.
    pub sizes: Vec<usize>,
}
impl JSONNumberStatistics {
    pub fn add_number(&mut self, value: &Number) {
        // TODO: better precision.
        self.sizes.push(ToString::to_string(value).len())
    }
}
impl Statistics for JSONNumberStatistics {
    fn size(&self) -> u64 {
        let value: usize = self.sizes.iter().sum();
        value as u64
    }
    fn count(&self) -> usize {
        self.sizes.len()
    }
    fn merge(&mut self, data: Cow<Self>) {
        self.sizes.extend_from_slice(&data.sizes);
    }
}
impl fmt::Display for JSONNumberStatistics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        StandardStatisticsFormatter::standard().format_entry(f, self.into())
    }
}

#[derive(Default, Debug, Clone)]
pub struct JSONStringStatistics {
    /// The sizes in characters of the encountered values.
    pub sizes: Vec<usize>,
}
impl JSONStringStatistics {
    pub fn add_string(&mut self, value: &str) {
        self.sizes.push(value.len());
    }
}
impl Statistics for JSONStringStatistics {
    fn size(&self) -> u64 {
        let mut value: usize = self.sizes.iter().sum();
        value += 2 * self.sizes.len(); // start and end quotes.
        value as u64
    }
    fn count(&self) -> usize {
        self.sizes.len()
    }
    fn merge(&mut self, data: Cow<Self>) {
        self.sizes.extend_from_slice(&data.sizes);
    }
}
impl fmt::Display for JSONStringStatistics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        StandardStatisticsFormatter::standard().format_entry(f, self.into())
    }
}

#[derive(Default, Debug, Clone)]
pub struct JSONArrayStatistics {
    /// The lengths of arrays.
    pub lengths: Vec<usize>,
    /// The sizes in characters of the encountered arrays.
    pub sizes: Vec<u64>,
    /// Info about the values that occurred in this array.
    pub values: Option<Box<JSONValueStatistics>>,
}
impl JSONArrayStatistics {
    pub fn get_values(&mut self) -> &mut JSONValueStatistics {
        if self.values.is_none() {
            self.values = Some(Default::default());
        }
        self.values.as_mut().unwrap()
    }
    pub fn add_array(&mut self, array: &[Value]) {
        let mut stats = JSONValueStatistics::default();
        for value in array {
            stats.add_value(value);
        }
        self.sizes.push(stats.size());
        self.lengths.push(array.len());
        self.get_values().merge(Cow::Owned(stats));
    }
}
impl Statistics for JSONArrayStatistics {
    fn size(&self) -> u64 {
        // Only content of the array not separators or [] at start and end of array.
        let inner_size: u64 = self.sizes.iter().sum();

        let separators: usize = self
            .lengths
            .iter()
            // ',' separator between each item + '[' + ']'
            .map(|items| (if *items > 0 { items - 1 } else { 0 }) + 2)
            .sum();

        inner_size + (separators as u64)
    }
    fn count(&self) -> usize {
        self.sizes.len()
    }
    fn merge(&mut self, data: Cow<Self>) {
        self.lengths.extend_from_slice(&data.lengths);
        self.sizes.extend_from_slice(&data.sizes);
        if data.values.is_some() {
            self.get_values().merge(match data {
                Cow::Borrowed(v) => Cow::Borrowed(v.values.as_ref().unwrap()),
                Cow::Owned(v) => Cow::Owned(*v.values.unwrap()),
            });
        }
    }
}
impl fmt::Display for JSONArrayStatistics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        StandardStatisticsFormatter::standard().format_entry(f, self.into())
    }
}

#[derive(Default, Debug, Clone)]
pub struct JSONObjectStatistics {
    /// The number of properties in each object.
    pub properties_count: Vec<usize>,
    /// The sizes in characters of the encountered objects.
    ///
    /// This includes property keys but not their surrounding quotes and any characters used to describe values
    /// but not separators between values and between keys and values like (`:` and `,`).
    pub sizes: Vec<u64>,
    /// Info about the properties that existed for this object.
    /// The key is the properties' names.
    pub properties: BTreeMap<String, JSONObjectPropertyStatistics>,
}
impl JSONObjectStatistics {
    pub fn add_object(&mut self, object: &Map<String, Value>) {
        let mut size = 0;
        for (key, value) in object.iter() {
            let mut data = JSONObjectPropertyStatistics::default();
            data.add_value(value);

            size += key.len() as u64;
            size += data.size();

            self.add_property(Cow::Borrowed(key), Cow::Owned(data));
        }

        self.properties_count.push(object.len());
        self.sizes.push(size);
    }

    /// Add more data about a property. This will not update `sizes` or `properties_count`.
    fn add_property(&mut self, name: Cow<String>, value: Cow<JSONObjectPropertyStatistics>) {
        match self.properties.get_mut(name.as_str()) {
            Some(info) => info.merge(value),
            None => {
                self.properties
                    .insert(name.into_owned(), value.into_owned());
            }
        }
    }
}
impl Statistics for JSONObjectStatistics {
    fn size(&self) -> u64 {
        let inner_size: u64 = self.sizes.iter().sum();
        // Data layout: {"KEY":VALUE,"KEY":VALUE}
        // KEY and VALUE text is included in `inner_size`.
        let quotes_and_separators: usize = self
            .properties_count
            .iter()
            .map(|count| {
                // For each object:
                // Quotes (") and colons (:):
                count * 3 +
                // commas (,):
                if *count > 0 { count - 1 } else { 0 } +
                // Also start `{` and end `}` brackets:
                2
            })
            .sum();

        inner_size + (quotes_and_separators as u64)
    }
    fn count(&self) -> usize {
        self.sizes.len()
    }
    fn merge(&mut self, data: Cow<Self>) {
        self.sizes.extend_from_slice(&data.sizes);
        self.properties_count
            .extend_from_slice(&data.properties_count);
        let properties = match data {
            Cow::Borrowed(v) => Either::Left(
                v.properties
                    .iter()
                    .map(|(key, value)| (Cow::Borrowed(key), Cow::Borrowed(value))),
            ),
            Cow::Owned(v) => Either::Right(
                v.properties
                    .into_iter()
                    .map(|(key, value)| (Cow::Owned(key), Cow::Owned(value))),
            ),
        };
        properties.for_each(|(key, value)| self.add_property(key, value));
    }
}
impl fmt::Display for JSONObjectStatistics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        StandardStatisticsFormatter::standard().format_entry(f, self.into())
    }
}

#[derive(Default, Debug, Clone)]
pub struct JSONObjectPropertyStatistics {
    /// The sizes in characters of the encountered properties that has this properties name.
    pub sizes: Vec<u64>,
    /// Info about the values that properties with this name had.
    pub value_info: JSONValueStatistics,
}
impl JSONObjectPropertyStatistics {
    pub fn add_value(&mut self, value: &Value) {
        let mut stats = JSONValueStatistics::default();
        stats.add_value(value);
        self.sizes.push(stats.size());
        self.value_info.merge(Cow::Owned(stats));
    }
}
impl Statistics for JSONObjectPropertyStatistics {
    fn size(&self) -> u64 {
        self.sizes.iter().sum()
    }
    fn count(&self) -> usize {
        self.sizes.len()
    }
    fn merge(&mut self, data: Cow<Self>) {
        self.sizes.extend_from_slice(&data.sizes);
        self.value_info.merge(match data {
            Cow::Borrowed(v) => Cow::Borrowed(&v.value_info),
            Cow::Owned(v) => Cow::Owned(v.value_info),
        });
    }
}
impl fmt::Display for JSONObjectPropertyStatistics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        StandardStatisticsFormatter::standard().format_entry(f, self.into())
    }
}

pub fn collect_statistics(json_value: &Value) -> JSONValueStatistics {
    let mut stats = JSONValueStatistics::default();
    stats.add_value(json_value);

    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statistics_start_at_zero_size() {
        assert_eq!(Statistics::size(&super::JSONValueStatistics::default()), 0)
    }
}
