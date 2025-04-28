//! Parse and create Tree Style Tab group tab URLs.

use std::fmt;

pub const TST_LEGACY_GROUP_URL: &str = "about:treestyletab-group";

/// Get the URL for a group tab by formatting this struct via `fmt::Display`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GroupTabInfo {
    /// The title of the group tab. This will be encoded.
    ///
    /// In JavaScript encoding can be done through `encodeURIComponent` and decoding through `decodeURIComponent`.
    ///
    /// In Rust the "url" crate can probably be used to decode and encode the name. See the `url::form_urlencoded::parse` function.
    pub name_url_encoded: Option<String>,
    /// Indicates if the group tab is temporary.
    pub temporary: Option<bool>,
    /// The internal id for Tree Style Tab. If this is `null` then the group tab uses the legacy group tab URL.
    pub internal_id: Option<String>,
    /// All arguments that should be suffixed to the group tab URL.
    ///
    /// If this is `Some(_)` then the `temporary` and `name` fields are ignored when formatting the URL.
    pub url_arguments: Option<String>,
}
impl GroupTabInfo {
    /// If the group tab's URL doesn't specify a specific name, then this name will be used instead.
    pub fn default_name() -> &'static str {
        "Group"
    }

    /// Try to parse a URL as a Tree Style Tab group tab and return the parsed information.
    pub fn from_url(mut url: &str) -> Option<Self> {
        let mut info = GroupTabInfo::default();

        if url.starts_with(TST_LEGACY_GROUP_URL) {
            url = &url[TST_LEGACY_GROUP_URL.len()..];
        } else {
            const START: &str = "moz-extension://";
            if !url.starts_with(START) {
                return None;
            }
            url = &url[START.len()..];

            let separator_index = url.find('/')?;
            info.internal_id = Some(url[..separator_index].to_owned());
            url = &url[(separator_index + 1)..];

            const LOCATION: &str = "resources/group-tab.html";
            const SIDEBERY_LOCATION: &str = "sidebery/group.html";
            if url.starts_with(LOCATION) {
                url = &url[LOCATION.len()..];
            } else if url.starts_with(SIDEBERY_LOCATION) {
                url = &url[SIDEBERY_LOCATION.len()..];
                if !url.starts_with('#') {
                    url = "";
                }
                info.url_arguments = Some(url.to_owned());
                info.name_url_encoded = Some(url.to_owned());
                return Some(info);
            } else {
                return None;
            }
        }

        info.url_arguments = Some(url.to_owned());

        if url.starts_with('?') {
            url = &url[1..];

            for query in url.split('&') {
                if let Some(index) = query.find('=') {
                    let key = &query[..index];
                    let value = &query[(index + 1)..];
                    match key {
                        "title" => info.name_url_encoded = Some(value.to_owned()),
                        "temporary" => {
                            if let Ok(value) = value.to_lowercase().parse() {
                                info.temporary = Some(value);
                            }
                        }
                        _ => (),
                    }
                }
            }
        }

        Some(info)
    }
}
impl fmt::Display for GroupTabInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(internal_id) = &self.internal_id {
            write!(
                f,
                "moz-extension://{}/resources/group-tab.html",
                internal_id
            )?;
        } else {
            write!(f, "{}", TST_LEGACY_GROUP_URL)?;
        }
        if let Some(url_args) = &self.url_arguments {
            write!(f, "{}", url_args)?;
        } else {
            let mut first_arg = true;
            let mut prepare_for_arg = |f: &mut fmt::Formatter| -> fmt::Result {
                write!(f, "{}", if first_arg { "?" } else { "&" })?;
                first_arg = false;
                Ok(())
            };
            if let Some(name) = &self.name_url_encoded {
                prepare_for_arg(f)?;
                write!(f, "title={}", name)?;
            }
            if let Some(temporary) = self.temporary {
                prepare_for_arg(f)?;
                write!(f, "temporary={}", if temporary { "true" } else { "false" })?;
            }
        }
        Ok(())
    }
}
