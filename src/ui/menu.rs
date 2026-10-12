//! The menu bar with its drop-down menus, the Roxanne button that opens
//! Preferences, and the command palette.

use super::style;
use super::{Message, RoxanneApp, preferences};
use crate::i18n::t;
use crate::keymap::KeyAction;
use crate::search::SearchScope;
use colony_ui::widgets::ABOUT_CATEGORY;
use colony_ui::{Palette, Typography};
use iced::advanced::widget::{self, Tree, Widget};
use iced::advanced::{Clipboard, Layout, Shell, layout, mouse, overlay, renderer};
use iced::widget::{button, column, container, row, scrollable, space, text_input};
use iced::{
    Alignment, Border, Element, Event, Length, Point, Rectangle, Renderer, Size, Task, Theme,
    Vector,
};

const MENU_BUTTON_PADDING_X: u16 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    File,
    Edit,
    Selection,
    View,
    Goto,
    Tools,
    Project,
    Help,
}

impl Menu {
    const ALL: [Menu; 8] = [
        Menu::File,
        Menu::Edit,
        Menu::Selection,
        Menu::View,
        Menu::Goto,
        Menu::Tools,
        Menu::Project,
        Menu::Help,
    ];

    fn label(self) -> &'static str {
        match self {
            Menu::File => t("menu_file"),
            Menu::Edit => t("menu_edit"),
            Menu::Selection => t("menu_selection"),
            Menu::View => t("menu_view"),
            Menu::Goto => t("menu_goto"),
            Menu::Tools => t("menu_tools"),
            Menu::Project => t("menu_project"),
            Menu::Help => t("menu_help"),
        }
    }

    fn entries(self) -> &'static [MenuEntry] {
        use MenuAction::*;
        use MenuEntry::{Action as A, Separator};
        match self {
            Menu::File => &[
                A(NewFile),
                A(Open),
                Separator,
                A(Save),
                A(SaveAs),
                A(CloseFile),
            ],
            Menu::Edit => &[
                A(Undo),
                A(Redo),
                Separator,
                A(Cut),
                A(Copy),
                A(Paste),
                Separator,
                A(Find),
                A(FindNext),
                A(FindPrevious),
                A(FindInFiles),
                Separator,
                A(ToggleSearchPanel),
                A(ToggleDiagnosticsPanel),
            ],
            Menu::Selection => &[
                A(SelectAll),
                A(AddCursorNextMatch),
                A(AddCursorsAllMatches),
                A(ClearMultiCursors),
            ],
            Menu::View => &[
                A(CommandPalette),
                A(ToggleFileTree),
                Separator,
                A(ToggleLineNumbers),
                A(ToggleLineWrap),
                A(ToggleMinimap),
            ],
            Menu::Goto => &[A(GoToLine)],
            Menu::Tools => &[
                A(LspHover),
                A(LspGotoDefinition),
                Separator,
                A(ReloadConfig),
                A(PerformanceReport),
            ],
            Menu::Project => &[A(FindInFiles), A(ToggleFileTree)],
            Menu::Help => &[A(Preferences), A(About)],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    Open,
    Save,
    SaveAs,
    Find,
    FindNext,
    FindPrevious,
    FindInFiles,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    AddCursorNextMatch,
    AddCursorsAllMatches,
    ClearMultiCursors,
    ToggleDiagnosticsPanel,
    ToggleSearchPanel,
    ToggleLineNumbers,
    ToggleLineWrap,
    ToggleMinimap,
    CommandPalette,
    GoToLine,
    ReloadConfig,
    PerformanceReport,
    ToggleFileTree,
    NewFile,
    CloseFile,
    Preferences,
    About,
    LspHover,
    LspGotoDefinition,
}

impl MenuAction {
    /// Every action, in the order the command palette lists them.
    const ALL: [MenuAction; 32] = [
        MenuAction::NewFile,
        MenuAction::Open,
        MenuAction::Save,
        MenuAction::SaveAs,
        MenuAction::CloseFile,
        MenuAction::Undo,
        MenuAction::Redo,
        MenuAction::Cut,
        MenuAction::Copy,
        MenuAction::Paste,
        MenuAction::Find,
        MenuAction::FindNext,
        MenuAction::FindPrevious,
        MenuAction::FindInFiles,
        MenuAction::SelectAll,
        MenuAction::AddCursorNextMatch,
        MenuAction::AddCursorsAllMatches,
        MenuAction::ClearMultiCursors,
        MenuAction::ToggleDiagnosticsPanel,
        MenuAction::ToggleSearchPanel,
        MenuAction::ToggleLineNumbers,
        MenuAction::ToggleLineWrap,
        MenuAction::ToggleMinimap,
        MenuAction::CommandPalette,
        MenuAction::ToggleFileTree,
        MenuAction::GoToLine,
        MenuAction::LspHover,
        MenuAction::LspGotoDefinition,
        MenuAction::ReloadConfig,
        MenuAction::PerformanceReport,
        MenuAction::Preferences,
        MenuAction::About,
    ];

    pub fn label(self) -> &'static str {
        match self {
            MenuAction::NewFile => t("action_new_file"),
            MenuAction::Open => t("action_open"),
            MenuAction::Save => t("action_save"),
            MenuAction::SaveAs => t("action_save_as"),
            MenuAction::CloseFile => t("action_close_file"),
            MenuAction::Undo => t("action_undo"),
            MenuAction::Redo => t("action_redo"),
            MenuAction::Cut => t("action_cut"),
            MenuAction::Copy => t("action_copy"),
            MenuAction::Paste => t("action_paste"),
            MenuAction::Find => t("action_find"),
            MenuAction::FindNext => t("action_find_next"),
            MenuAction::FindPrevious => t("action_find_previous"),
            MenuAction::FindInFiles => t("action_find_in_files"),
            MenuAction::SelectAll => t("action_select_all"),
            MenuAction::AddCursorNextMatch => t("action_add_cursor_next"),
            MenuAction::AddCursorsAllMatches => t("action_add_cursors_all"),
            MenuAction::ClearMultiCursors => t("action_clear_cursors"),
            MenuAction::ToggleDiagnosticsPanel => t("action_diagnostics_panel"),
            MenuAction::ToggleSearchPanel => t("action_search_panel"),
            MenuAction::ToggleLineNumbers => t("action_line_numbers"),
            MenuAction::ToggleLineWrap => t("action_line_wrap"),
            MenuAction::ToggleMinimap => t("action_minimap"),
            MenuAction::CommandPalette => t("action_command_palette"),
            MenuAction::ToggleFileTree => t("action_file_tree"),
            MenuAction::GoToLine => t("action_go_to_line"),
            MenuAction::LspHover => t("action_lsp_hover"),
            MenuAction::LspGotoDefinition => t("action_lsp_definition"),
            MenuAction::ReloadConfig => t("action_reload_config"),
            MenuAction::PerformanceReport => t("action_performance"),
            MenuAction::Preferences => t("settings_title"),
            MenuAction::About => t("settings_cat_about"),
        }
    }

    /// The keymap action with the same effect, whose shortcut the menus show.
    fn key_action(self) -> Option<KeyAction> {
        Some(match self {
            MenuAction::NewFile => KeyAction::NewTab,
            MenuAction::Open => KeyAction::Open,
            MenuAction::Save => KeyAction::Save,
            MenuAction::CloseFile => KeyAction::CloseTab,
            MenuAction::Undo => KeyAction::Undo,
            MenuAction::Redo => KeyAction::Redo,
            MenuAction::Cut => KeyAction::Cut,
            MenuAction::Copy => KeyAction::Copy,
            MenuAction::Paste => KeyAction::Paste,
            MenuAction::Find => KeyAction::Find,
            MenuAction::FindNext => KeyAction::FindNext,
            MenuAction::FindPrevious => KeyAction::FindPrevious,
            MenuAction::SelectAll => KeyAction::SelectAll,
            MenuAction::CommandPalette => KeyAction::CommandPalette,
            MenuAction::ToggleFileTree => KeyAction::ToggleFileTree,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum MenuEntry {
    Action(MenuAction),
    Separator,
}

impl RoxanneApp {
    pub(super) fn menu_action(&mut self, action: MenuAction) -> Task<Message> {
        // The result of an action shows in the editor, not in Preferences.
        if !matches!(action, MenuAction::Preferences | MenuAction::About) {
            self.preferences.open = false;
        }
        match action {
            MenuAction::NewFile => self.new_tab(),
            MenuAction::Open => return self.open_file(),
            MenuAction::Save => return self.save_file(),
            MenuAction::SaveAs => return self.open_save_as(),
            MenuAction::CloseFile => self.close_tab(self.active_tab),
            MenuAction::Undo => self.apply_undo(),
            MenuAction::Redo => self.apply_redo(),
            MenuAction::Cut => return self.cut(),
            MenuAction::Copy => return self.copy(),
            MenuAction::Paste => return self.handle_key_action(KeyAction::Paste),
            MenuAction::Find => return self.focus_search(),
            MenuAction::FindNext => self.find_next_match(true),
            MenuAction::FindPrevious => self.find_next_match(false),
            MenuAction::FindInFiles => {
                self.search_panel_open = true;
                self.search_scope = SearchScope::Workspace;
                return self.search_in_files();
            }
            MenuAction::SelectAll => self.select_all(),
            MenuAction::AddCursorNextMatch => self.add_cursor_next_match(),
            MenuAction::AddCursorsAllMatches => self.add_cursors_all_matches(),
            MenuAction::ClearMultiCursors => self.clear_cursors(),
            MenuAction::ToggleDiagnosticsPanel => {
                self.diagnostics_panel_open = !self.diagnostics_panel_open;
            }
            MenuAction::ToggleSearchPanel => self.search_panel_open = !self.search_panel_open,
            MenuAction::ToggleLineNumbers => {
                return self.preferences_update(preferences::Message::LineNumbers(
                    !self.config.editor.line_numbers,
                ));
            }
            MenuAction::ToggleLineWrap => {
                return self.preferences_update(preferences::Message::WordWrap(
                    !self.config.editor.word_wrap,
                ));
            }
            MenuAction::ToggleMinimap => {
                return self.preferences_update(preferences::Message::Minimap(
                    !self.config.editor.minimap,
                ));
            }
            MenuAction::CommandPalette => self.toggle_command_palette(),
            MenuAction::ToggleFileTree => self.toggle_file_tree(),
            MenuAction::GoToLine => self.open_goto_panel(),
            MenuAction::LspHover => return self.lsp_hover(),
            MenuAction::LspGotoDefinition => return self.lsp_goto_definition(),
            MenuAction::ReloadConfig => self.reload_config(true),
            MenuAction::PerformanceReport => {
                self.status_message = Some(self.performance.summary());
            }
            MenuAction::Preferences => self.preferences.show(0),
            MenuAction::About => self
                .preferences
                .show(preferences::category_index(ABOUT_CATEGORY)),
        }
        Task::none()
    }

    pub(super) fn toggle_command_palette(&mut self) {
        self.command_palette_open = !self.command_palette_open;
        self.command_palette_query.clear();
    }

    /// The commands whose label holds the query's characters in order.
    pub(super) fn filtered_commands(&self) -> Vec<(&'static str, MenuAction)> {
        let query = self.command_palette_query.to_lowercase();
        MenuAction::ALL
            .into_iter()
            .map(|action| (action.label(), action))
            .filter(|(label, _)| fuzzy_match(&label.to_lowercase(), &query))
            .collect()
    }

    /// The hint next to a menu entry: its shortcut in the active keymap.
    fn shortcut(&self, action: MenuAction) -> Option<String> {
        self.config
            .keymap
            .shortcut_label(action.key_action()?, self.mode)
    }

    pub(super) fn menu_bar(&self, typo: &Typography) -> Element<'_, Message> {
        let mut items = row![colony_ui::widgets::identity_button(
            typo,
            "Roxanne",
            15,
            self.preferences.open,
            Message::Preferences(preferences::Message::Toggle),
        )]
        .spacing(12)
        .align_y(Alignment::Center);
        for menu in Menu::ALL {
            let open = self.active_menu == Some(menu);
            items = items.push(
                button(style::button_label(typo, menu.label(), 13))
                    .padding([3, MENU_BUTTON_PADDING_X])
                    .style(colony_ui::widgets::selection_style(open))
                    .on_press(Message::MenuSelected(menu)),
            );
        }

        let bar = container(items)
            .width(Length::Fill)
            .padding([4, 12])
            .style(style::surface(Palette::BG_SIDEBAR()));

        // The identity button comes first, then the menus.
        let open_index = self
            .active_menu
            .and_then(|open| Menu::ALL.iter().position(|menu| *menu == open))
            .map(|index| index + 1);
        MenuOverlay {
            content: bar.into(),
            overlay: self.active_menu.map(|menu| self.submenu(typo, menu)),
            dismiss_message: self.active_menu.map(Message::MenuSelected),
            active_menu_index: open_index,
        }
        .into()
    }

    fn submenu(&self, typo: &Typography, menu: Menu) -> Element<'_, Message> {
        let entries = menu.entries().iter().map(|entry| match *entry {
            MenuEntry::Action(action) => {
                let mut label =
                    row![style::button_label(typo, action.label(), 12).width(Length::Fill)]
                        .spacing(24)
                        .align_y(Alignment::Center);
                if let Some(shortcut) = self.shortcut(action) {
                    label = label.push(style::label(typo, shortcut, 11, Palette::TEXT_DIMMER()));
                }
                button(label)
                    .width(Length::Fill)
                    .padding([3, 8])
                    .style(style::flat)
                    .on_press(Message::MenuAction(action))
                    .into()
            }
            MenuEntry::Separator => container(space().height(1))
                .width(Length::Fill)
                .style(style::divider)
                .into(),
        });

        container(column(entries).spacing(4))
            .width(Length::Fixed(300.0 * typo.scale))
            .padding([8, 8])
            .style(|_| container::Style {
                background: Some(Palette::BG_CARD().into()),
                border: Border::default()
                    .rounded(6)
                    .width(1)
                    .color(Palette::BORDER_SUBTLE()),
                ..container::Style::default()
            })
            .into()
    }

    pub(super) fn command_palette(&self, typo: &Typography) -> Option<Element<'_, Message>> {
        if !self.command_palette_open {
            return None;
        }

        let input = text_input(t("palette_placeholder"), &self.command_palette_query)
            .on_input(Message::CommandPaletteChanged)
            .padding([6, 12])
            .size(typo.sz(14))
            .font(typo.regular)
            .style(style::input);

        let commands = self.filtered_commands();
        let list: Element<'_, Message> = if commands.is_empty() {
            style::label(typo, t("palette_empty"), 12, Palette::TEXT_DIMMER()).into()
        } else {
            let entries =
                commands
                    .into_iter()
                    .take(15)
                    .enumerate()
                    .map(|(index, (label, action))| {
                        let mut entry =
                            row![style::button_label(typo, label, 13).width(Length::Fill)]
                                .align_y(Alignment::Center);
                        if let Some(shortcut) = self.shortcut(action) {
                            entry = entry.push(style::label(
                                typo,
                                shortcut,
                                11,
                                Palette::TEXT_DIMMER(),
                            ));
                        }
                        button(entry)
                            .width(Length::Fill)
                            .padding([6, 16])
                            .style(style::flat)
                            .on_press(Message::CommandPaletteSelected(index))
                            .into()
                    });
            scrollable(column(entries).spacing(2))
                .height(Length::Fixed(300.0 * typo.scale))
                .into()
        };

        Some(
            container(column![input, list].spacing(8))
                .padding([12, 16])
                .width(Length::Fixed(500.0 * typo.scale))
                .style(style::surface(Palette::BG_CARD()))
                .into(),
        )
    }
}

/// Fuzzy match: each character of `query` must appear in order in `text`.
pub(super) fn fuzzy_match(text: &str, query: &str) -> bool {
    let mut text_chars = text.chars();
    query
        .chars()
        .all(|wanted| text_chars.by_ref().any(|found| found == wanted))
}

/// The menu bar, with the open menu drawn over the window below it.
struct MenuOverlay<'a> {
    content: Element<'a, Message>,
    overlay: Option<Element<'a, Message>>,
    /// Sent by a click outside the open menu.
    dismiss_message: Option<Message>,
    /// The position of the open menu's button in the bar's row.
    active_menu_index: Option<usize>,
}

impl<'a> Widget<Message, Theme, Renderer> for MenuOverlay<'a> {
    fn children(&self) -> Vec<Tree> {
        let mut children = vec![Tree::new(&self.content)];
        if let Some(overlay) = &self.overlay {
            children.push(Tree::new(overlay));
        }
        children
    }

    fn diff(&self, tree: &mut Tree) {
        let mut children = vec![self.content.as_widget()];
        if let Some(overlay) = &self.overlay {
            children.push(overlay.as_widget());
        }
        tree.diff_children(&children);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let mut children = tree.children.iter_mut();
        let content_overlay = self.content.as_widget_mut().overlay(
            children.next()?,
            layout,
            renderer,
            viewport,
            translation,
        );

        // The bar's container holds the row whose children are the buttons.
        let active_button_x = self.active_menu_index.and_then(|index| {
            let button = layout.children().next()?.children().nth(index)?;
            Some(button.bounds().x + translation.x)
        });

        let menu_overlay = match (&mut self.overlay, children.next()) {
            (Some(overlay), Some(state)) => Some(overlay::Element::new(Box::new(MenuLayer {
                bar_bottom: layout.bounds().y + layout.bounds().height + translation.y,
                left: active_button_x.unwrap_or(layout.position().x + translation.x),
                overlay,
                state,
                dismiss_message: self.dismiss_message.clone(),
            }))),
            _ => None,
        };

        match (content_overlay, menu_overlay) {
            (Some(content), Some(menu)) => {
                Some(overlay::Group::with_children(vec![content, menu]).overlay())
            }
            (content, menu) => content.or(menu),
        }
    }
}

impl<'a> From<MenuOverlay<'a>> for Element<'a, Message> {
    fn from(overlay: MenuOverlay<'a>) -> Self {
        Element::new(overlay)
    }
}

/// The open menu, below the bar and level with its button. A click anywhere
/// else closes it.
struct MenuLayer<'a, 'b> {
    bar_bottom: f32,
    left: f32,
    overlay: &'b mut Element<'a, Message>,
    state: &'b mut Tree,
    dismiss_message: Option<Message>,
}

impl overlay::Overlay<Message, Theme, Renderer> for MenuLayer<'_, '_> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let below = Size::new(bounds.width, (bounds.height - self.bar_bottom).max(0.0));
        let menu = self.overlay.as_widget_mut().layout(
            self.state,
            renderer,
            &layout::Limits::new(Size::ZERO, below),
        );
        // Kept inside the window when the button is near its right edge.
        let left = self.left.min((bounds.width - menu.size().width).max(0.0));
        layout::Node::with_children(below, vec![menu.move_to(Point::new(left, 0.0))])
            .move_to(Point::new(0.0, self.bar_bottom))
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        if let Some(menu) = layout.children().next() {
            self.overlay.as_widget().draw(
                self.state,
                renderer,
                theme,
                style,
                menu,
                cursor,
                &Rectangle::INFINITE,
            );
        }
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        if let Some(menu) = layout.children().next()
            && cursor.is_over(menu.bounds())
        {
            self.overlay.as_widget_mut().update(
                self.state,
                event,
                menu,
                cursor,
                renderer,
                clipboard,
                shell,
                &Rectangle::INFINITE,
            );
            return;
        }
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && let Some(message) = self.dismiss_message.clone()
        {
            shell.publish(message);
            shell.capture_event();
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        layout
            .children()
            .next()
            .map_or(mouse::Interaction::None, |menu| {
                self.overlay.as_widget().mouse_interaction(
                    self.state,
                    menu,
                    cursor,
                    &Rectangle::INFINITE,
                    renderer,
                )
            })
    }
}
