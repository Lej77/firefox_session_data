//! Firefox sessionstore files contains a JSON Value that can be deserialized to a `FirefoxSessionStore` struct.

pub mod group_tab;
mod serde_as_json_str;
mod serde_as_str;
pub mod session_info;
pub mod to_links;

use serde::{Deserialize, Serialize};

#[cfg(feature = "view")]
pub use serde_unstructured;
#[cfg(feature = "view")]
use serde_unstructured::SerdeView;

#[cfg_attr(feature = "view", derive(SerdeView))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FirefoxSessionStore {
    pub version: Vec<FirefoxVersionInfo>,
    #[serde(default)]
    pub windows: Vec<FirefoxWindow>,
    #[serde(default, rename = "_closedWindows")]
    pub _closed_windows: Vec<FirefoxWindow>,
    pub selected_window: i64,
    pub session: FirefoxSession,
    pub global: FirefoxGlobal,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum FirefoxVersionInfo {
    Text(String),
    Number(i64),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FirefoxGlobal {}

#[cfg_attr(feature = "view", derive(SerdeView))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FirefoxSession {
    pub last_update: i64,
    pub start_time: i64,
    pub recent_crashes: i64,
}

#[cfg_attr(feature = "view", derive(SerdeView))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FirefoxWindow {
    /// Info about all open tabs.
    pub tabs: Vec<FirefoxTab>,
    /// A 1-based index for the currently selected tab.
    pub selected: i64,
    /// Info about recently closed tabs.
    #[serde(default)]
    pub _closed_tabs: Vec<FirefoxTab>,
    pub busy: Option<bool>,
    /// Extension data stored via the
    /// [`browser.sessions.setWindowValue`](https://developer.mozilla.org/docs/Mozilla/Add-ons/WebExtensions/API/sessions/setWindowValue)
    /// API.
    #[serde(default = "window_data::ExtensionData::null")]
    pub ext_data: window_data::ExtensionData,
    pub width: i64,
    pub height: i64,
    pub screen_x: i64,
    pub screen_y: i64,
    pub sizemode: String,
    #[serde(default)]
    pub cookies: Vec<window_data::Cookie>,
    #[serde(default)]
    pub sidebar: SidebarInfo,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum SidebarInfo {
    #[default]
    None,
    /// Firefox version 126 and earlier stored info about the open sidebar as a
    /// string.
    String(String),
    /// Firefox version 127 and later stored info about the open sidebar as an
    /// object/map.
    Map {
        /// This property seems to always be `null` in Firefox version 127.
        position_end: Option<()>,
        /// This string identifies what sidebar panel is open. It is not present
        /// if the sidebar is closed.
        command: Option<String>,
    },
}

#[cfg_attr(feature = "view", derive(SerdeView))]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FirefoxTab {
    /// The history entries for the tab. The current entry can be found via the
    /// `index` field. Note that this can have 0 length in some circumstances.
    pub entries: Vec<tab_data::URLEntry>,
    pub last_accessed: i64,
    pub pinned: Option<bool>,
    pub hidden: bool,
    pub attributes: tab_data::Attributes,
    #[serde(default = "tab_data::ExtensionData::null")]
    pub ext_data: tab_data::ExtensionData,
    pub user_context_id: i64,
    /// The index of the current history entry in the `entries` list. The index
    /// isn't zero based and starts at 1.
    pub index: Option<i64>,
    pub scroll: Option<tab_data::Scroll>,
    pub user_typed_value: Option<String>,
    pub user_typed_clear: Option<i64>,
    pub unloaded_at: Option<i64>,
    pub image: Option<String>,
    pub icon_loading_principal: Option<String>,
}

pub mod window_data {
    //! Types for data that is stored inside [`FirefoxWindow`](super::FirefoxWindow).

    use crate::serde_as_json_str;
    use serde::{Deserialize, Serialize};

    #[cfg(feature = "view")]
    use serde_unstructured::SerdeView;

    #[cfg_attr(feature = "view", derive(SerdeView))]
    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "kebab-case")]
    pub struct ExtensionData {
        /// If the [`FirefoxWindow`](super::FirefoxWindow) was missing its
        /// `ext_data` field then it is filled in with a default ExtensionData
        /// where this field is `true`. Otherwise if there was an object for
        /// that field (even if that object was empty) then this is `false`.
        #[serde(skip, default = "ExtensionData::had_some_data")]
        pub no_data: bool,

        pub tabview_groups: Option<String>,

        #[cfg_attr(feature = "view", serde_view(skip))]
        #[serde(default, with = "serde_as_json_str")]
        pub tabview_group: Option<TabGroup>,

        /// The "Tree Style Tab" addon's "scroll position" value used to remember
        /// the sidebar's scroll location.
        #[serde(rename = "extension:treestyletab@piro.sakura.ne.jp:scroll-position")]
        pub tree_style_tab_web_extension_scroll_position: Option<String>,

        /// The "Tab Count in Window Title" addon's "window name" value used to give
        /// a window a unique name.
        #[cfg_attr(feature = "view", serde_view(skip))]
        #[serde(rename = "extension:{c28e42b2-28b5-45f0-bdc8-6989ae7e6a7e}:name")]
        #[serde(default, with = "serde_as_json_str")]
        pub tab_count_in_window_title_name: Option<String>,

        /// The "Tab Count in Window Title" addon's "is restored" value which indicates
        /// that this window has been seen by that addon before.
        #[serde(rename = "extension:{c28e42b2-28b5-45f0-bdc8-6989ae7e6a7e}:isRestored")]
        pub tab_count_in_window_title_is_restored: Option<String>,

        /// The "Other Window" addon's "window name" setting.
        #[cfg_attr(feature = "view", serde_view(skip))]
        #[serde(rename = "extension:{5df6e133-f35d-4c62-885a-56387df22f6b}:windowName")]
        #[serde(default, with = "serde_as_json_str")]
        pub other_window_name: Option<String>,

        /// Sidebery groups.
        #[serde(rename = "extension:{3c078156-979c-498b-8990-85f7987dd929}:groups")]
        pub sidebery_groups: Option<String>,
    }
    impl ExtensionData {
        fn had_some_data() -> bool {
            false
        }
        pub(crate) fn null() -> Self {
            Self {
                no_data: true,
                tabview_groups: None,
                tabview_group: None,
                tree_style_tab_web_extension_scroll_position: None,
                tab_count_in_window_title_name: None,
                tab_count_in_window_title_is_restored: None,
                other_window_name: None,
                sidebery_groups: None,
            }
        }
    }

    #[cfg_attr(feature = "view", derive(SerdeView))]
    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct Cookie {
        pub host: String,
        pub value: String,
        pub path: String,
        pub name: String,
        pub origin_attributes: OriginAttributes,
    }

    #[cfg_attr(feature = "view", derive(SerdeView))]
    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct OriginAttributes {
        pub addon_id: String,
        pub app_id: i64,
        pub first_party_domain: String,
        pub in_isolated_moz_browser: bool,
        pub private_browsing_id: i64,
        pub user_context_id: i64,
    }

    #[cfg_attr(feature = "view", derive(SerdeView))]
    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct TabGroup {
        pub slot: Option<i64>,
        pub title: Option<String>,
        pub id: Option<i64>,
    }
}

pub mod tab_data {
    //! Types for data that is stored inside [`FirefoxTab`](super::FirefoxTab).

    use crate::serde_as_json_str;
    use crate::serde_as_str;
    use serde::{Deserialize, Serialize};

    #[cfg(feature = "view")]
    use serde_unstructured::SerdeView;

    #[cfg_attr(feature = "view", derive(SerdeView))]
    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct URLEntry {
        pub url: String,
        pub title: String,
        pub charset: Option<String>,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct Attributes {}

    #[cfg_attr(feature = "view", derive(SerdeView))]
    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct Scroll {
        pub scroll: Option<String>,
        pub children: Option<Vec<Option<Scroll>>>,
    }

    #[cfg_attr(feature = "view", derive(SerdeView))]
    #[derive(Deserialize, Serialize, Debug, Clone, Default)]
    #[serde(rename_all = "kebab-case")]
    pub struct ExtensionData {
        /// If the [`FirefoxTab`](super::FirefoxTab) was missing its `ext_data`
        /// field then it is filled in with a default `ExtensionData` where this
        /// field is `true`. Otherwise if there was an object for the `ext_data`
        /// field (even if that object was empty) then this is `false`.
        #[serde(skip, default = "ExtensionData::had_some_data")]
        pub no_data: bool,

        #[cfg_attr(feature = "view", serde_view(skip))]
        #[serde(default, with = "serde_as_json_str")]
        pub tabview_tab: Option<TabView>,

        pub treestyletab_id: Option<String>,
        pub treestyletab_subtree_collapsed: Option<String>,
        pub treestyletab_insert_after: Option<String>,
        pub treestyletab_insert_before: Option<String>,
        pub treestyletab_parent: Option<String>,

        #[cfg_attr(feature = "view", serde_view(skip))]
        #[serde(default, with = "serde_as_json_str")]
        #[serde(rename = "extension:treestyletab@piro.sakura.ne.jp:data-persistent-id")]
        pub tree_style_tab_web_extension_id: Option<TreeStyleTabsWebExtensionId>,

        #[serde(rename = "extension:treestyletab@piro.sakura.ne.jp:insert-before")]
        pub tree_style_tab_web_extension_insert_before: Option<String>,

        #[serde(rename = "extension:treestyletab@piro.sakura.ne.jp:insert-after")]
        pub tree_style_tab_web_extension_insert_after: Option<String>,

        #[cfg_attr(feature = "view", serde_view(skip))]
        #[serde(rename = "extension:treestyletab@piro.sakura.ne.jp:subtree-collapsed")]
        #[serde(default, with = "serde_as_str")]
        pub tree_style_tabs_web_extension_subtree_collapsed: Option<bool>,

        #[cfg_attr(feature = "view", serde_view(skip))]
        #[serde(rename = "extension:treestyletab@piro.sakura.ne.jp:ancestors")]
        #[serde(default, with = "serde_as_json_str")]
        pub tree_style_tabs_web_extension_ancestors: Option<Vec<String>>,

        #[cfg_attr(feature = "view", serde_view(skip))]
        #[serde(rename = "extension:treestyletab@piro.sakura.ne.jp:children")]
        #[serde(default, with = "serde_as_json_str")]
        pub tree_style_tabs_web_extension_children: Option<Vec<String>>,

        #[serde(rename = "extension:{dab33964-ee66-494e-a816-b064ca5518c4}:marked")]
        pub marked_for_removal: Option<String>,

        /// Used by Sidebery to store data. The id can be seen by extracting the
        /// addon installation file (`.xpi`) from the [addon store] and reading
        /// the "manifest.json" file.
        ///
        /// You can see what data it stores at:
        /// https://github.com/mbnuqw/sidebery/blob/3933196225ed0b2f713cbb7831c81a7023b579ed/src/services/tabs.fg.actions.ts#L595-L607
        ///
        /// [addon store]: https://addons.mozilla.org/firefox/addon/sidebery/first
        #[cfg_attr(feature = "view", serde_view(skip))]
        #[serde(default, with = "serde_as_json_str")]
        #[serde(rename = "extension:{3c078156-979c-498b-8990-85f7987dd929}:data")]
        pub sidebery_data: Option<SideberyData>,
    }
    impl ExtensionData {
        fn had_some_data() -> bool {
            false
        }
        pub(crate) fn null() -> Self {
            Self {
                no_data: true,
                ..Default::default()
            }
        }
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct TabView {
        #[serde(rename = "groupID")]
        pub group_id: i64,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct TreeStyleTabsWebExtensionId {
        pub id: String,
        pub tab_id: Option<i64>,
    }

    /// Tab data stored by Sidebery.
    #[derive(Deserialize, Serialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct SideberyData {
        pub id: i64,
        pub panel_id: String,
        pub parent_id: i64,
        pub folded: bool,
        pub custom_title: Option<String>,
        pub custom_color: Option<String>,
    }
}
