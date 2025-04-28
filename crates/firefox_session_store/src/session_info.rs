//! This module can be used to get tab information about a Firefox sessionstore file.

use either::*;

use super::group_tab::GroupTabInfo;
use crate as session_store;

use std::borrow::Cow;
use std::convert::TryInto;
use std::iter;

#[derive(Debug, Clone)]
pub struct TabGroup<'a> {
    name: Cow<'a, str>,
    tabs: Vec<TabInfo<'a>>,
}
impl<'a> TabGroup<'a> {
    pub fn new(name: impl Into<Cow<'a, str>>, tabs: Vec<TabInfo<'a>>) -> Self {
        Self {
            name: name.into(),
            tabs,
        }
    }
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
    pub fn tabs(&self) -> &[TabInfo<'a>] {
        &self.tabs
    }
}

fn sort_groups(mut groups: Vec<TabGroup<'_>>) -> Vec<TabGroup<'_>> {
    groups.sort_by(|a, b| a.name().cmp(b.name()));
    groups
}

/// Get tabs in groups for a given Firefox session.
pub fn get_groups_from_session(
    session_data: &session_store::FirefoxSessionStore,
    include_open_windows: bool,
    include_closed_windows: bool,
    sort_names: bool,
) -> impl Iterator<Item = TabGroup> {
    let open_windows = session_data
        .windows
        .iter()
        .filter(move |_| include_open_windows)
        .enumerate()
        .map(|(index, window)| WindowInfo::new(window).as_group(format!("Window {}", index + 1)));
    let closed_windows = session_data
        ._closed_windows
        .iter()
        .filter(move |_| include_closed_windows)
        .enumerate()
        .map(|(index, window)| {
            WindowInfo::new(window).as_group(format!("Closed window {}", index + 1))
        });

    if sort_names {
        Left(
            sort_groups(open_windows.collect())
                .into_iter()
                .chain(sort_groups(closed_windows.collect())),
        )
    } else {
        Right(open_windows.chain(closed_windows))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WindowInfo<'a> {
    pub data: &'a session_store::FirefoxWindow,
}
impl<'a> WindowInfo<'a> {
    pub fn new(data: &'a session_store::FirefoxWindow) -> Self {
        Self { data }
    }
    /// The name of the window. This can be provided via some extensions.
    pub fn name(&self) -> Option<Cow<'a, str>> {
        if let Some(name) = &self.data.ext_data.tab_count_in_window_title_name {
            // From "Tab count in window title" extension:
            Some(Cow::from(name))
        } else if let Some(name) = &self.data.ext_data.other_window_name {
            // From "Other window" extension:
            Some(Cow::from(name))
        } else {
            // If the first tab is a pinned Tree Style Tab group tab then use its title:
            let first_tab = self.data.tabs.first()?;
            if first_tab.pinned.unwrap_or(false) {
                let entry = TabInfo::new(first_tab).current_entry()?;
                let group_info = GroupTabInfo::from_url(&entry.url)?;

                if entry.title.is_empty() || entry.title == entry.url {
                    // The stored tab title doesn't represent the group's correct name. This is usually not the case but it can happen.
                    // Attempt to guess the group tab's title from parsed URL info:
                    group_info.name_url_encoded.map(Cow::from)
                } else {
                    Some(Cow::from(&*entry.title))
                }
            } else {
                None
            }
        }
    }

    pub fn as_group(&self, default_name: impl Into<Cow<'a, str>>) -> TabGroup<'a> {
        TabGroup::new(
            self.name().unwrap_or_else(|| default_name.into()),
            self.tabs_iter().collect(),
        )
    }

    /// Iterate over the window's tabs.
    pub fn tabs_iter(&self) -> impl Iterator<Item = TabInfo<'a>> {
        let window = *self;
        self.data.tabs.iter().map(move |tab| TabInfo {
            data: tab,
            window: Some(window),
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TabInfo<'data> {
    pub data: &'data session_store::FirefoxTab,
    pub window: Option<WindowInfo<'data>>,
}
impl<'data> TabInfo<'data> {
    /// Create an info object from some tab data.
    pub fn new(tab_data: &'data session_store::FirefoxTab) -> Self {
        Self {
            data: tab_data,
            window: None,
        }
    }

    /// The index of the current history entry. The other entries represents the tabs history.
    pub fn current_entry_index(&self) -> Option<usize> {
        let index = (self.data.index? - 1).try_into().ok()?;
        if index >= self.data.entries.len() {
            None
        } else {
            Some(index)
        }
    }

    /// The current history entry with the tab's title and URL.
    pub fn current_entry(&self) -> Option<&'data session_store::tab_data::URLEntry> {
        self.data.entries.get(self.current_entry_index()?)
    }

    /// The title for this tab.
    pub fn title(&self) -> &'data str {
        self.current_entry()
            .map(|entry| entry.title.as_str())
            .unwrap_or_default()
    }
    /// The URL for this tab.
    pub fn url(&self) -> &'data str {
        self.current_entry()
            .map(|entry| entry.url.as_str())
            .unwrap_or_default()
    }

    pub fn scroll(&self) -> Option<&'data str> {
        let scroll_info = self.data.scroll.as_ref()?;

        iter::once(scroll_info)
            .chain(
                scroll_info
                    .children
                    .as_ref()
                    .into_iter()
                    .flatten()
                    .flatten(),
            )
            .filter_map(|scroll_info| scroll_info.scroll.as_ref())
            .find(|scroll| !scroll.is_empty())
            .map(String::as_str)
    }

    pub fn tst_id(
        &self,
        tree_sources: &[TreeDataSource],
    ) -> Option<TreeDataOutput<TreeTabId<'data>>> {
        TreeDataAction {
            tst_web_ext: || self.tst_web_ext_id().map(Into::into),
            tst_legacy: || self.tst_legacy_id().map(Into::into),
            sidebery: || Some(self.data.ext_data.sidebery_data.as_ref()?.id.into()),
        }
        .preform(tree_sources)
    }
    pub fn tst_parent_id(
        &self,
        tree_sources: &[TreeDataSource],
    ) -> Option<TreeDataOutput<TreeTabId<'data>>> {
        TreeDataAction {
            tst_web_ext: || self.tst_web_ext_parent_id().map(Into::into),
            tst_legacy: || self.tst_legacy_parent_id().map(Into::into),
            sidebery: || Some(self.data.ext_data.sidebery_data.as_ref()?.parent_id.into()),
        }
        .preform(tree_sources)
    }

    pub fn tst_web_ext_id(&self) -> Option<&'data str> {
        self.data
            .ext_data
            .tree_style_tab_web_extension_id
            .as_ref()
            .map(|id_info| id_info.id.as_str())
    }
    pub fn tst_web_ext_parent_id(&self) -> Option<&'data str> {
        self.data
            .ext_data
            .tree_style_tabs_web_extension_ancestors
            .as_ref()?
            .first()
            .map(String::as_str)
    }

    pub fn tst_legacy_id(&self) -> Option<&'data str> {
        self.data.ext_data.treestyletab_id.as_deref()
    }
    pub fn tst_legacy_parent_id(&self) -> Option<&'data str> {
        self.data.ext_data.treestyletab_parent.as_deref()
    }

    /// Get the ancestor tabs of this tab using Tree Style Tab session data. The first tab in the iterator will be this tab's parent tab.
    pub fn tst_ancestor_tabs<'iter>(
        &'iter self,
        mut tree_sources: &'iter [TreeDataSource],
        window: WindowInfo<'data>,
    ) -> impl Iterator<Item = TreeDataOutput<TabInfo<'data>>> + 'iter {
        let mut current_tab = *self;
        iter::from_fn(move || {
            if tree_sources.is_empty() {
                return None;
            }
            let parent_id = current_tab.tst_parent_id(tree_sources)?;

            if tree_sources.len() > 1 {
                // Only allow fallback when resolving first parent as to prevent infinite loops and other strange behaviors.
                let ix = tree_sources
                    .iter()
                    .position(|&s| s == parent_id.tree_data_source)
                    .expect("can only find tree data from allowed sources");

                tree_sources = std::array::from_ref(&tree_sources[ix]);
            }
            if matches!(current_tab.tst_id(tree_sources), Some(current_id) if current_id.value == parent_id.value)
            {
                // Ignore parent id if it same as current tab:
                return None;
            }

            let parent_tab = window
                .tabs_iter()
                .find(|tab| matches!(tab.tst_id(tree_sources), Some(tab_id) if tab_id.value == parent_id.value))?;

            current_tab = parent_tab;
            Some(TreeDataOutput {
                tree_data_source: tree_sources[0],
                value: current_tab,
            })
        })
    }
}

/// An id for a tab used by Tree Style Tab like extensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TreeTabId<'a> {
    /// Used by Tree Style Tab (both legacy and WebExtension versions.)
    Text(&'a str),
    /// Used by Sidebery.
    Number(i64),
}
impl<'a> From<&'a str> for TreeTabId<'a> {
    fn from(value: &'a str) -> Self {
        TreeTabId::Text(value)
    }
}
impl From<i64> for TreeTabId<'_> {
    fn from(value: i64) -> Self {
        TreeTabId::Number(value)
    }
}

pub struct TreeDataOutput<T: ?Sized> {
    /// What source the tree data was gathered from.
    pub tree_data_source: TreeDataSource,
    /// The result of the operation.
    pub value: T,
}
impl<T> TreeDataOutput<T> {
    pub fn into_tuple(self) -> (TreeDataSource, T) {
        (self.tree_data_source, self.value)
    }
}
impl<T> From<(TreeDataSource, T)> for TreeDataOutput<T> {
    fn from(value: (TreeDataSource, T)) -> Self {
        TreeDataOutput {
            tree_data_source: value.0,
            value: value.1,
        }
    }
}

pub struct TreeDataAction<F1, F2, F3> {
    pub tst_web_ext: F1,
    pub tst_legacy: F2,
    pub sidebery: F3,
}
impl<F1, F2, F3> TreeDataAction<F1, F2, F3> {
    pub fn preform<T>(mut self, sources: &[TreeDataSource]) -> Option<TreeDataOutput<T>>
    where
        F1: FnMut() -> Option<T>,
        F2: FnMut() -> Option<T>,
        F3: FnMut() -> Option<T>,
    {
        for source in sources {
            let result = match source {
                TreeDataSource::TstWebExtension => (self.tst_web_ext)(),
                TreeDataSource::TstLegacy => (self.tst_legacy)(),
                TreeDataSource::Sidebery => (self.sidebery)(),
            };
            if let Some(value) = result {
                return Some(TreeDataOutput {
                    tree_data_source: *source,
                    value,
                });
            }
        }
        None
    }
}

/// Specify where to load tab tree data from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TreeDataSource {
    /// Use the modern Tree Style Tab web extension's data.
    TstWebExtension,
    /// Use the tree data from the old Tree Style Tab addon, the one from before
    /// Firefox had WebExtensions.
    TstLegacy,
    /// Use the tree data from Sidebery.
    Sidebery,
}
impl TreeDataSource {
    /// Search tabs to see if any of them has tree data from the specified
    /// source.
    pub fn has_any_data<'a>(
        &self,
        tabs: impl IntoIterator<Item = &'a session_store::FirefoxTab>,
    ) -> bool {
        for tab in tabs {
            if match self {
                TreeDataSource::TstWebExtension => {
                    tab.ext_data.tree_style_tab_web_extension_id.is_some()
                }
                TreeDataSource::TstLegacy => tab.ext_data.treestyletab_id.is_some(),
                TreeDataSource::Sidebery => tab.ext_data.sidebery_data.is_some(),
            } {
                return true;
            }
        }
        false
    }
}
