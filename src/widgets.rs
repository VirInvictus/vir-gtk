// SPDX-License-Identifier: MIT
//! Shared plain-GTK4 widgets: the `adw::ActionRow` / `SwitchRow` /
//! `ComboRow` / `EntryRow` / `SpinRow` / `PreferencesGroup` / `AlertDialog`
//! replacements the suite's applications carried as near-verbatim copies
//! (the first-slice consolidation of the 2026-09-12 audit).
//!
//! The row builders return the row plus, where it carries an interactive
//! control, the control itself, so call sites keep the exact
//! `set_active` / `selected` / `text` surface they wired against. The rows
//! are deliberately plain `GtkListBoxRow`s in a `.boxed-list` `GtkListBox`,
//! which is the class name adwaita used, so the owned stylesheets keep
//! defining it without touching call sites. The [`Alert`] is a small modal
//! [`gtk::Window`]: stock `gtk::AlertDialog` has no extra child, no
//! per-response styling, and index-addressed buttons.
//!
//! Styling leans on the `.heading` / `.caption` / `.dim-label` /
//! `.boxed-list` utility classes [`base_css`](crate::theme::base_css)
//! ships. Exact adwaita pixel metrics are not the contract; structure and
//! behaviour are.

use gtk::glib;
use gtk::pango;
use gtk::prelude::*;
use gtk4 as gtk;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Install Escape-to-close on a plain `gtk::Window`. `adw::Dialog` /
/// `adw::Window` gave this for free; the owned plain-GTK dialogs wire it
/// explicitly so Escape dismisses on every desktop, not only where the
/// compositor happens to offer a close affordance.
///
/// The controller runs in the capture phase, so a focused entry cannot
/// swallow the key first, and holds the window weakly so the
/// window -> controller -> closure chain cannot pin the window alive.
pub fn close_on_escape(window: &impl IsA<gtk::Window>) {
    let win = window.clone().upcast::<gtk::Window>();
    let controller = gtk::EventControllerKey::new();
    // Capture phase, as documented: a focused entry inside the window runs
    // its own key controllers in the bubble phase, so capture is what makes
    // Escape reach this controller first (the failure Viaduct's original
    // captured the phase for: a focused entry swallowed the key).
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    controller.connect_key_pressed(glib::clone!(
        #[weak]
        win,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                win.close();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        }
    ));
    win.add_controller(controller);
}

/// The shared row body: title over an optional dim subtitle on the left, an
/// optional trailing suffix on the right. Returns the subtitle label so
/// [`action_row`] can hand it out for later mutation; it starts hidden when
/// the subtitle is absent or empty.
fn build_row(
    title: Option<&str>,
    subtitle: Option<&str>,
    suffix: Option<&gtk::Widget>,
) -> (gtk::ListBoxRow, gtk::Label) {
    let text = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .hexpand(true)
        .valign(gtk::Align::Center)
        .build();
    if let Some(title) = title.filter(|t| !t.is_empty()) {
        let title_label = gtk::Label::builder()
            .label(title)
            .xalign(0.0)
            .ellipsize(pango::EllipsizeMode::End)
            .build();
        text.append(&title_label);
    }
    let subtitle_label = gtk::Label::builder()
        .label(subtitle.unwrap_or_default())
        .xalign(0.0)
        .ellipsize(pango::EllipsizeMode::End)
        .css_classes(["caption", "dim-label"])
        .build();
    subtitle_label.set_tooltip_text(subtitle);
    subtitle_label.set_visible(subtitle.is_some_and(|s| !s.is_empty()));
    text.append(&subtitle_label);

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(12)
        .margin_end(12)
        .build();
    content.append(&text);
    if let Some(suffix) = suffix {
        suffix.set_valign(gtk::Align::Center);
        content.append(suffix);
    }
    let row = gtk::ListBoxRow::builder()
        .activatable(false)
        .child(&content)
        .build();
    (row, subtitle_label)
}

/// A non-activatable list row. Long titles and subtitles ellipsize; the
/// subtitle carries itself as a tooltip so nothing is lost to the cut.
pub fn row(title: &str, subtitle: Option<&str>, suffix: Option<&gtk::Widget>) -> gtk::ListBoxRow {
    build_row(Some(title), subtitle, suffix).0
}

/// An `adw::ActionRow` successor for rows whose subtitle changes at runtime:
/// the returned label is the subtitle (kept visible so updates always show).
pub fn action_row(
    title: &str,
    subtitle: Option<&str>,
    suffix: Option<&gtk::Widget>,
) -> (gtk::ListBoxRow, gtk::Label) {
    let (row, label) = build_row(Some(title), subtitle, suffix);
    label.set_visible(true);
    (row, label)
}

/// An `adw::SwitchRow` successor; the returned `gtk::Switch` keeps the exact
/// `set_active` / `is_active` / `connect_active_notify` surface.
pub fn switch_row(title: &str, subtitle: Option<&str>) -> (gtk::ListBoxRow, gtk::Switch) {
    let switch = gtk::Switch::new();
    let row = row(title, subtitle, Some(switch.upcast_ref()));
    (row, switch)
}

/// An `adw::SpinRow` successor; the returned `gtk::SpinButton` carries the
/// `set_digits` / `set_value` / `value` / `adjustment` surface.
pub fn spin_row(
    title: &str,
    subtitle: Option<&str>,
    min: f64,
    max: f64,
    step: f64,
) -> (gtk::ListBoxRow, gtk::SpinButton) {
    let spin = gtk::SpinButton::with_range(min, max, step);
    let row = row(title, subtitle, Some(spin.upcast_ref()));
    (row, spin)
}

/// An `adw::ComboRow` successor; the returned `gtk::DropDown` carries the
/// `set_selected` / `selected` / `connect_selected_notify` surface.
pub fn combo_row(
    title: &str,
    subtitle: Option<&str>,
    items: &[&str],
) -> (gtk::ListBoxRow, gtk::DropDown) {
    let dropdown = gtk::DropDown::from_strings(items);
    let row = row(title, subtitle, Some(dropdown.upcast_ref()));
    (row, dropdown)
}

/// A row whose trailing control is a button, activatable by clicking the row
/// as well as the button (the `adw::ActionRow` + `set_activatable_widget`
/// idiom; a `GtkListBoxRow` does not forward activation to its child, so
/// the row mirrors it).
pub fn button_row(title: &str, subtitle: Option<&str>, button: &gtk::Button) -> gtk::ListBoxRow {
    let row = row(title, subtitle, Some(button.upcast_ref()));
    row.set_activatable(true);
    let owned = button.clone();
    row.connect_activate(move |_| owned.emit_clicked());
    row
}

/// An `adw::EntryRow` successor; the returned `gtk::Entry` carries the
/// `set_text` / `text` / `connect_changed` / `connect_activate` surface.
/// (The adwaita apply button has no analogue; callers that used
/// `connect_apply` switch to `connect_activate` / a nearby confirm button.)
///
/// Every part is optional because the consumers' three historical signatures
/// disagreed: Atrium and Conservatory passed `(title, text)`, Viaduct passed
/// `(title, subtitle, placeholder)`. This is the union; `adwaita`'s floating
/// title-in-entry has no plain-GTK analogue, so the title sits to the left
/// like every other row here and the placeholder carries the hint.
pub fn entry_row(
    title: Option<&str>,
    subtitle: Option<&str>,
    text: Option<&str>,
    placeholder: Option<&str>,
) -> (gtk::ListBoxRow, gtk::Entry) {
    let entry = gtk::Entry::builder()
        .hexpand(true)
        .valign(gtk::Align::Center)
        .build();
    if let Some(text) = text {
        entry.set_text(text);
    }
    if let Some(placeholder) = placeholder {
        entry.set_placeholder_text(Some(placeholder));
    }
    let row = build_row(title, subtitle, Some(entry.upcast_ref())).0;
    (row, entry)
}

/// An `adw::PreferencesGroup` successor: an optional heading and dim
/// description (with room for a trailing header suffix) over a `.boxed-list`
/// of rows. Cloning shares the same underlying widgets, the way a tuple of
/// `gtk::Box` + `gtk::ListBox` used to.
#[derive(Clone)]
pub struct Group {
    root: gtk::Box,
    header: gtk::Box,
    list: gtk::ListBox,
}

/// Build a [`Group`].
pub fn group(title: Option<&str>, description: Option<&str>) -> Group {
    let text = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .valign(gtk::Align::Center)
        .build();
    if let Some(title) = title.filter(|t| !t.is_empty()) {
        text.append(
            &gtk::Label::builder()
                .label(title)
                .xalign(0.0)
                .css_classes(["heading"])
                .build(),
        );
    }
    if let Some(description) = description.filter(|d| !d.is_empty()) {
        text.append(
            &gtk::Label::builder()
                .label(description)
                .xalign(0.0)
                .wrap(true)
                .css_classes(["caption", "dim-label"])
                .build(),
        );
    }
    let header = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .visible(title.is_some() || description.is_some())
        .build();
    header.append(&text);
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .build();
    root.append(&header);
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    root.append(&list);
    Group { root, header, list }
}

impl Group {
    /// The widget to place (a dialog extra child, a page section).
    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }

    /// A trailing widget in the header line (the way
    /// `adw::PreferencesGroup::set_header_suffix` placed one). Reveals the
    /// header even when the group has no title/description.
    pub fn set_header_suffix(&self, suffix: &impl IsA<gtk::Widget>) {
        suffix.as_ref().set_valign(gtk::Align::Center);
        self.header.append(suffix);
        self.header.set_visible(true);
    }

    /// Append a row; any non-row widget is wrapped in a non-activatable row,
    /// the way `adw::PreferencesGroup::add` did.
    pub fn add(&self, child: &impl IsA<gtk::Widget>) {
        if let Some(row) = child.as_ref().downcast_ref::<gtk::ListBoxRow>() {
            self.list.append(row);
        } else {
            let wrapper = gtk::ListBoxRow::builder()
                .activatable(false)
                .child(child)
                .build();
            self.list.append(&wrapper);
        }
    }

    /// Remove every row (for groups whose contents rebuild at runtime).
    pub fn clear(&self) {
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
    }
}

/// The visual weight of an [`Alert`] response button, mapping to the CSS
/// classes the owned stylesheets and libadwaita both define.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Appearance {
    /// The regular button look (no extra class).
    Normal,
    /// `suggested-action`.
    Suggested,
    /// `destructive-action`.
    Destructive,
}

impl Appearance {
    /// The CSS class this appearance adds to its button (`None` for
    /// [`Appearance::Normal`]).
    pub fn css_class(self) -> Option<&'static str> {
        match self {
            Self::Normal => None,
            Self::Suggested => Some("suggested-action"),
            Self::Destructive => Some("destructive-action"),
        }
    }
}

type ResponseHandler = Box<dyn Fn(&str)>;

struct AlertState {
    responses: RefCell<Vec<(String, gtk::Button)>>,
    handler: RefCell<Option<ResponseHandler>>,
    default_response: RefCell<Option<String>>,
    close_response: RefCell<String>,
    /// Exactly-once dispatch per presentation: a button click emits its id
    /// and closes; the window's close path (Escape, the WM close button,
    /// `close()`) emits the close response only if nothing was emitted yet.
    /// [`Alert::present`] resets the latch, so a re-presented dialog answers
    /// again (the adwaita dialogs this replaces are reusable).
    responded: Cell<bool>,
}

impl AlertState {
    fn emit(&self, id: &str) {
        if self.responded.replace(true) {
            return;
        }
        if let Some(handler) = self.handler.borrow().as_ref() {
            handler(id);
        }
    }
}

/// A small modal dialog: the `adw::AlertDialog` replacement. Heading and
/// body over an optional extra child, named responses with per-response
/// appearance, a default response for Enter, and a close response for
/// Escape / the WM close.
///
/// `present` derives the transient parent from the anchor widget's root
/// window (the anchor may be a button inside another dialog; the transient
/// parent must be that window, not the main one). Every close path emits
/// exactly one response per presentation: a button press emits its id,
/// dismissal without a button emits the close response (`"close"` unless
/// [`Alert::set_close_response`] says otherwise, matching adwaita), and a
/// re-presented dialog answers again.
pub struct Alert {
    win: gtk::Window,
    extra_slot: gtk::Box,
    button_box: gtk::Box,
    state: Rc<AlertState>,
}

impl Alert {
    /// Build the dialog. `heading` and `body` skip empty strings, so
    /// passing an empty string keeps that line out of the layout.
    pub fn new(heading: Option<&str>, body: Option<&str>) -> Self {
        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .margin_top(20)
            .margin_bottom(20)
            .margin_start(20)
            .margin_end(20)
            .build();
        if let Some(heading) = heading.filter(|h| !h.is_empty()) {
            content.append(
                &gtk::Label::builder()
                    .label(heading)
                    .wrap(true)
                    .justify(gtk::Justification::Center)
                    .css_classes(["heading"])
                    .build(),
            );
        }
        if let Some(body) = body.filter(|b| !b.is_empty()) {
            content.append(
                &gtk::Label::builder()
                    .label(body)
                    .wrap(true)
                    .justify(gtk::Justification::Center)
                    .build(),
            );
        }
        let extra_slot = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        content.append(&extra_slot);
        let button_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk::Align::End)
            .margin_top(8)
            .build();
        content.append(&button_box);

        let win = gtk::Window::builder()
            .title(heading.unwrap_or_default())
            .modal(true)
            .resizable(false)
            .default_width(360)
            .child(&content)
            .build();
        close_on_escape(&win);

        let state = Rc::new(AlertState {
            responses: RefCell::new(Vec::new()),
            handler: RefCell::new(None),
            default_response: RefCell::new(None),
            close_response: RefCell::new("close".to_string()),
            responded: Cell::new(false),
        });
        // Any close path that skipped the buttons (Escape, the WM close
        // button) still answers, with the close response.
        let close_state = Rc::downgrade(&state);
        win.connect_close_request(move |_| {
            if let Some(state) = close_state.upgrade() {
                let id = state.close_response.borrow().clone();
                state.emit(&id);
            }
            glib::Propagation::Proceed
        });

        Self {
            win,
            extra_slot,
            button_box,
            state,
        }
    }

    /// The dialog window itself (to focus an entry after presenting, to
    /// anchor popovers, and the like).
    pub fn window(&self) -> &gtk::Window {
        &self.win
    }

    /// Set the widget shown between the body text and the buttons (a rename
    /// entry, a form). `None` clears it.
    pub fn set_extra_child(&self, child: Option<&impl IsA<gtk::Widget>>) {
        while let Some(old) = self.extra_slot.first_child() {
            self.extra_slot.remove(&old);
        }
        if let Some(child) = child {
            self.extra_slot.append(child);
        }
    }

    /// Add a response button. The id is what the response handler receives.
    /// Styling goes through [`Alert::set_response_appearance`] after adding
    /// (the Atrium/Conservatory shape, kept so their call sites convert
    /// mechanically).
    pub fn add_response(&self, id: &str, label: &str) {
        let button = gtk::Button::with_label(label);
        let state = Rc::downgrade(&self.state);
        let weak = self.win.downgrade();
        let response = id.to_string();
        button.connect_clicked(move |_| {
            if let Some(state) = state.upgrade() {
                state.emit(&response);
            }
            if let Some(win) = weak.upgrade() {
                win.close();
            }
        });
        self.button_box.append(&button);
        self.state
            .responses
            .borrow_mut()
            .push((id.to_string(), button));
    }

    /// Weigh a response button (`suggested-action` / `destructive-action`;
    /// [`Appearance::Normal`] strips any class back off).
    pub fn set_response_appearance(&self, id: &str, appearance: Appearance) {
        if let Some((_, button)) = self
            .state
            .responses
            .borrow()
            .iter()
            .find(|(rid, _)| rid == id)
        {
            for class in ["suggested-action", "destructive-action"] {
                button.remove_css_class(class);
            }
            if let Some(class) = appearance.css_class() {
                button.add_css_class(class);
            }
        }
    }

    /// The response activated by Enter (via the window default widget;
    /// entries opt in with `set_activates_default(true)`).
    pub fn set_default_response(&self, id: Option<&str>) {
        *self.state.default_response.borrow_mut() = id.map(str::to_string);
    }

    /// The response emitted when the dialog is dismissed without a button
    /// (Escape, the WM close). Defaults to `"close"`, matching adwaita.
    pub fn set_close_response(&self, id: &str) {
        *self.state.close_response.borrow_mut() = id.to_string();
    }

    /// The response currently emitted on dismissal without a button.
    pub fn close_response(&self) -> String {
        self.state.close_response.borrow().clone()
    }

    /// Register the handler that receives the response id. Exactly one id
    /// is emitted per dialog presentation, whichever close path wins.
    pub fn connect_response(&self, handler: impl Fn(&str) + 'static) {
        *self.state.handler.borrow_mut() = Some(Box::new(handler));
    }

    /// Show the dialog, transient for the anchor widget's root window.
    /// `None` presents without a transient parent (the rare headless or
    /// test case). Each presentation answers exactly once: presenting
    /// resets the response latch, so a dialog can be reused for a second
    /// question the way the adwaita dialogs it replaces are.
    pub fn present(&self, parent: Option<&impl IsA<gtk::Widget>>) {
        self.state.responded.set(false);
        if let Some(parent) = parent {
            let root = parent.as_ref().root().and_downcast::<gtk::Window>();
            self.win.set_transient_for(root.as_ref());
        }
        if let Some(id) = self.state.default_response.borrow().as_deref() {
            if let Some((_, button)) = self
                .state
                .responses
                .borrow()
                .iter()
                .find(|(rid, _)| rid == id)
            {
                self.win.set_default_widget(Some(button));
            }
        }
        self.win.present();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The row builders are pure widget construction, so the useful thing to
    /// pin is the shape the stylesheet and the callers depend on.
    /// `#[gtk::test]`, not `#[test]`: cargo runs tests on a thread pool and
    /// widget construction needs one initialized GTK; the macro funnels
    /// every body through one already-initialized thread.
    #[gtk::test]
    fn rows_are_not_activatable_unless_they_act() {
        gtk::init().unwrap();
        assert!(!row("Title", None, None).is_activatable());
        let button = gtk::Button::new();
        assert!(button_row("Title", None, &button).is_activatable());
    }

    #[gtk::test]
    fn button_row_forward_activation_to_its_button() {
        gtk::init().unwrap();
        let clicked = std::rc::Rc::new(std::cell::Cell::new(0));
        let button = gtk::Button::new();
        let hits = clicked.clone();
        button.connect_clicked(move |_| hits.set(hits.get() + 1));
        let row = button_row("Reset", None, &button);
        row.emit_by_name::<()>("activate", &[]);
        assert_eq!(clicked.get(), 1, "row activation must click the button");
    }

    #[gtk::test]
    fn entry_row_carries_every_optional_part() {
        gtk::init().unwrap();
        let (row, entry) = entry_row(
            Some("Token"),
            Some("From the account page"),
            Some("abc123"),
            Some("paste the token here"),
        );
        assert_eq!(entry.text(), "abc123");
        assert_eq!(
            entry.placeholder_text().as_deref(),
            Some("paste the token here")
        );
        assert!(row.child().is_some());
        // The all-empty form must build too: no part is required.
        let (_, empty) = entry_row(None, None, None, None);
        assert_eq!(empty.text(), "");
    }

    #[gtk::test]
    fn group_list_is_a_non_selectable_boxed_list() {
        gtk::init().unwrap();
        let group = group(Some("Title"), None);
        let list = group
            .widget()
            .last_child()
            .unwrap()
            .downcast::<gtk::ListBox>()
            .unwrap();
        assert!(list.has_css_class("boxed-list"));
        assert_eq!(list.selection_mode(), gtk::SelectionMode::None);
    }

    #[gtk::test]
    fn group_add_wraps_and_clear_empties() {
        gtk::init().unwrap();
        let group = group(None, None);
        let (list_row, _) = switch_row("Sync", None);
        let list_row = list_row.clone();
        group.add(&list_row);
        let plain = gtk::Label::new(Some("bare"));
        group.add(&plain);
        let list = group
            .widget()
            .last_child()
            .unwrap()
            .downcast::<gtk::ListBox>()
            .unwrap();
        assert_eq!(count_rows(&list), 2, "bare widgets must be wrapped");
        group.clear();
        assert_eq!(count_rows(&list), 0);
    }

    fn count_rows(list: &gtk::ListBox) -> usize {
        let mut n = 0;
        let mut child = list.first_child();
        while let Some(row) = child {
            n += 1;
            child = row.next_sibling();
        }
        n
    }

    #[test]
    fn appearances_map_to_the_owned_classes() {
        assert_eq!(Appearance::Normal.css_class(), None);
        assert_eq!(Appearance::Suggested.css_class(), Some("suggested-action"));
        assert_eq!(
            Appearance::Destructive.css_class(),
            Some("destructive-action")
        );
    }

    #[gtk::test]
    fn close_on_escape_pins_the_capture_phase() {
        // The 1.4.0 kit shipped the controller phase-less (GTK defaults to
        // bubble) while every doc promised capture, silently dropping the
        // behavior Viaduct's pre-adoption code existed for. The test pins
        // the phase so the docs stay true.
        gtk::init().unwrap();
        let win = gtk::Window::new();
        close_on_escape(&win);
        let controllers = win.observe_controllers();
        let found = (0..controllers.n_items())
            .filter_map(|i| controllers.item(i))
            .filter_map(|obj| obj.downcast::<gtk::EventControllerKey>().ok())
            .any(|c| c.propagation_phase() == gtk::PropagationPhase::Capture);
        assert!(found, "the Escape controller must run in the capture phase");
    }

    #[gtk::test]
    fn alert_dismissal_emits_the_close_response_exactly_once() {
        gtk::init().unwrap();
        let alert = Alert::new(Some("Delete?"), Some("This cannot be undone."));
        let seen = Rc::new(RefCell::new(Vec::<String>::new()));
        let sink = seen.clone();
        alert.connect_response(move |id| sink.borrow_mut().push(id.to_string()));
        alert.add_response("cancel", "Cancel");
        alert.set_response_appearance("cancel", Appearance::Normal);
        alert.add_response("delete", "Delete");
        alert.set_response_appearance("delete", Appearance::Destructive);
        // Two close paths firing must still answer exactly once. The signal
        // is emitted directly because a bare test window never maps, and
        // `close()` is a no-op on unmapped windows; the signal is what every
        // real close path (Escape via close_on_escape, the WM button,
        // `close()` on a mapped window) runs through.
        alert.window().emit_by_name::<bool>("close-request", &[]);
        alert.window().emit_by_name::<bool>("close-request", &[]);
        assert_eq!(*seen.borrow(), vec!["close".to_string()]);
    }

    #[gtk::test]
    fn alert_close_response_is_rebrandable() {
        gtk::init().unwrap();
        let alert = Alert::new(None, None);
        assert_eq!(alert.close_response(), "close");
        alert.set_close_response("cancel");
        assert_eq!(alert.close_response(), "cancel");
        let seen = Rc::new(RefCell::new(Vec::<String>::new()));
        let sink = seen.clone();
        alert.connect_response(move |id| sink.borrow_mut().push(id.to_string()));
        alert.window().emit_by_name::<bool>("close-request", &[]);
        assert_eq!(*seen.borrow(), vec!["cancel".to_string()]);
    }

    #[gtk::test]
    fn alert_a_represented_dialog_answers_again() {
        // The spec contract is "exactly one response id per presentation";
        // the 1.4.0 latch was per Alert lifetime, dead on any re-present.
        gtk::init().unwrap();
        let alert = Alert::new(Some("Again?"), None);
        let seen = Rc::new(RefCell::new(Vec::<String>::new()));
        let sink = seen.clone();
        alert.connect_response(move |id| sink.borrow_mut().push(id.to_string()));
        // Within one presentation, two close paths still answer once.
        alert.window().emit_by_name::<bool>("close-request", &[]);
        alert.window().emit_by_name::<bool>("close-request", &[]);
        assert_eq!(*seen.borrow(), vec!["close".to_string()]);
        // A fresh presentation resets the latch.
        alert.present(None::<&gtk::Widget>);
        alert.window().emit_by_name::<bool>("close-request", &[]);
        assert_eq!(
            *seen.borrow(),
            vec!["close".to_string(), "close".to_string()]
        );
    }

    #[gtk::test]
    fn alert_extra_child_swaps() {
        gtk::init().unwrap();
        let alert = Alert::new(None, None);
        let first = gtk::Entry::new();
        alert.set_extra_child(Some(&first));
        let second = gtk::Entry::new();
        alert.set_extra_child(Some(&second));
        // The slot is the first child of the content box after the labels;
        // a swap that kept the old child would strand two entries.
        let slot = alert.window().first_child().unwrap();
        let slot = slot.downcast::<gtk::Box>().unwrap();
        let mut entries = 0usize;
        let mut child = slot.first_child();
        while let Some(widget) = child {
            if widget.downcast_ref::<gtk::Box>().is_some() {
                let mut nested = widget.first_child();
                while let Some(inner) = nested {
                    if inner.downcast_ref::<gtk::Entry>().is_some() {
                        entries += 1;
                    }
                    nested = inner.next_sibling();
                }
            }
            child = widget.next_sibling();
        }
        assert_eq!(entries, 1, "set_extra_child must replace, not stack");
    }
}
