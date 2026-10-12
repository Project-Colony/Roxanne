//! The file tree beside the editor.

use super::style;
use super::{Message, RoxanneApp};
use crate::i18n::t;
use colony_ui::{Palette, Typography};
use iced::widget::{button, column, container, scrollable, text_input};
use iced::{Element, Length};
use std::path::{Path, PathBuf};

const FILE_TREE_MAX_DEPTH: usize = 16;
const FILE_TREE_MAX_ENTRIES: usize = 5000;

#[derive(Debug, Clone)]
pub struct FileTreeEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    depth: usize,
    expanded: bool,
    children: Vec<FileTreeEntry>,
}

/// Load .gitignore patterns from a directory, returning simple name patterns.
fn load_gitignore_patterns(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join(".gitignore"))
        .map(|contents| {
            contents
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(|line| line.trim_end_matches('/').to_string())
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn is_gitignored(name: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| {
        pattern == name
            // Simple wildcard: *.ext
            || pattern
                .strip_prefix("*.")
                .is_some_and(|ext| name.ends_with(&format!(".{ext}")))
    })
}

impl FileTreeEntry {
    fn scan(path: &Path) -> Option<Self> {
        Self::scan_with_ignore(path, 0, &[])
    }

    fn scan_with_ignore(path: &Path, depth: usize, parent_ignores: &[String]) -> Option<Self> {
        let name = path.file_name()?.to_string_lossy().to_string();
        if name.starts_with('.') || is_gitignored(&name, parent_ignores) {
            return None;
        }
        let is_dir = path.is_dir();
        let children = if is_dir && depth < FILE_TREE_MAX_DEPTH {
            let mut ignores = parent_ignores.to_vec();
            ignores.extend(load_gitignore_patterns(path));
            let mut entries: Vec<FileTreeEntry> = std::fs::read_dir(path)
                .ok()?
                .filter_map(Result::ok)
                .filter_map(|entry| Self::scan_with_ignore(&entry.path(), depth + 1, &ignores))
                .collect();
            entries.sort_by(|a, b| {
                b.is_dir
                    .cmp(&a.is_dir)
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
            entries
        } else {
            Vec::new()
        };
        Some(Self {
            name,
            path: path.to_path_buf(),
            is_dir,
            depth,
            expanded: depth == 0,
            children,
        })
    }

    pub fn toggle_dir(&mut self, target: &Path) -> bool {
        if self.path == target {
            self.expanded = !self.expanded;
            return true;
        }
        self.children
            .iter_mut()
            .any(|child| child.toggle_dir(target))
    }

    fn flatten_visible(&self) -> Vec<&FileTreeEntry> {
        let mut result = vec![self];
        if self.is_dir && self.expanded {
            for child in &self.children {
                result.extend(child.flatten_visible());
            }
        }
        result
    }
}

impl RoxanneApp {
    pub(super) fn toggle_file_tree(&mut self) {
        self.file_tree_open = !self.file_tree_open;
        if self.file_tree_open && self.file_tree.is_none() {
            let root = self
                .resolve_workspace_root()
                .ok()
                .or_else(|| std::env::current_dir().ok());
            self.file_tree = root.and_then(|root| FileTreeEntry::scan(&root));
        }
    }

    pub(super) fn file_tree_panel(&self, typo: &Typography) -> Option<Element<'_, Message>> {
        if !self.file_tree_open {
            return None;
        }
        let tree = self.file_tree.as_ref()?;
        let filter = self.file_tree_filter.to_lowercase();

        let mut items = column![
            container(
                text_input(t("tree_filter_placeholder"), &self.file_tree_filter)
                    .on_input(Message::FileTreeFilterChanged)
                    .padding([2, 6])
                    .size(typo.sz(11))
                    .font(typo.regular)
                    .style(style::input),
            )
            .padding([2, 4]),
        ]
        .width(Length::Fill);

        for entry in tree
            .flatten_visible()
            .into_iter()
            .take(FILE_TREE_MAX_ENTRIES)
        {
            if !filter.is_empty() && !entry.is_dir && !entry.name.to_lowercase().contains(&filter) {
                continue;
            }
            // Folders carry the chevron the Preferences sections use.
            let marker = match (entry.is_dir, entry.expanded) {
                (true, true) => colony_ui::widgets::icons::CHEVRON_DOWN,
                (true, false) => colony_ui::widgets::icons::CHEVRON_RIGHT,
                (false, _) => " ",
            };
            let label = iced::widget::row![
                style::button_label(typo, marker, 9).font(colony_ui::fonts::glyph_font()),
                style::button_label(typo, entry.name.as_str(), 12),
            ]
            .spacing(6)
            .padding(iced::Padding::ZERO.left(12.0 * entry.depth as f32));
            let message = if entry.is_dir {
                Message::FileTreeToggleDir(entry.path.clone())
            } else {
                Message::FileTreeFileClicked(entry.path.clone())
            };
            items = items.push(
                button(label)
                    .width(Length::Fill)
                    .padding([1, 4])
                    .style(style::flat)
                    .on_press(message),
            );
        }

        let width = 220.0 * typo.scale;
        Some(
            container(scrollable(items).height(Length::Fill))
                .width(Length::Fixed(width))
                .height(Length::Fill)
                .style(style::surface(Palette::BG_SIDEBAR()))
                .into(),
        )
    }
}
