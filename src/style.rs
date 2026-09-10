// SPDX-License-Identifier: MIT
//! The managed stylesheet lifecycle: named rungs of the display priority
//! ladder ([`StyleManager`]).
//!
//! Two rules govern everything here. First, the ladder is the override
//! mechanism: the system `gtk.css` sits below `USER`, crate sheets install
//! at `USER + 1`, application sheets at `USER + 2`, and anything an app
//! layers above that (Conservatory's runtime accent provider sits at
//! `USER + 3`) is just another rung. Second, handles are keys, not owners:
//! a stylesheet is display-global state, so dropping a [`StyleManager`]
//! changes nothing; teardown is explicit ([`StyleManager::remove`]).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk4 as gtk;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;

use crate::theme::{base_css, Palette};

thread_local! {
    /// The providers this crate last installed per (display, priority), so a
    /// re-install on a theme switch replaces its tier's provider instead of
    /// accumulating providers on the display. GTK objects are `!Send`; styling
    /// runs on the main thread.
    static INSTALLED: RefCell<Vec<(gtk::gdk::Display, u32, gtk::CssProvider)>> =
        const { RefCell::new(Vec::new()) };
}

fn install_at(css: &str, priority: u32) -> Option<gtk::CssProvider> {
    let display = gtk::gdk::Display::default()?;
    let provider = gtk::CssProvider::new();
    provider.load_from_string(css);
    INSTALLED.with(|installed| {
        let mut installed = installed.borrow_mut();
        if let Some(pos) = installed
            .iter()
            .position(|(d, p, _)| d == &display && p == &priority)
        {
            let (_, _, old) = installed.remove(pos);
            gtk::style_context_remove_provider_for_display(&display, &old);
        }
        gtk::style_context_add_provider_for_display(&display, &provider, priority);
        installed.push((display, priority, provider.clone()));
    });
    Some(provider)
}

fn remove_at(priority: u32) {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    INSTALLED.with(|installed| {
        let mut installed = installed.borrow_mut();
        if let Some(pos) = installed
            .iter()
            .position(|(d, p, _)| d == &display && p == &priority)
        {
            let (_, _, old) = installed.remove(pos);
            gtk::style_context_remove_provider_for_display(&display, &old);
        }
    });
}

fn is_installed_at(priority: u32) -> bool {
    let Some(display) = gtk::gdk::Display::default() else {
        return false;
    };
    INSTALLED.with(|installed| {
        installed
            .borrow()
            .iter()
            .any(|(d, p, _)| d == &display && p == &priority)
    })
}

/// A handle to one rung of the stylesheet ladder: the (display, priority)
/// slot this crate tracks on the default display. Cloning shares the rung.
///
/// Handles are keys, not owners: the installed sheet outlives every clone
/// of its handle, and dropping them all changes nothing. Tear a rung down
/// with [`StyleManager::remove`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleManager {
    priority: u32,
}

impl StyleManager {
    /// The crate tier (`STYLE_PROVIDER_PRIORITY_USER + 1`): the sheets this
    /// crate owns, `base_css` and the palette blocks. This is the rung
    /// [`crate::theme::install_stylesheet`] manages.
    pub fn crate_tier() -> Self {
        Self {
            priority: gtk::STYLE_PROVIDER_PRIORITY_USER + 1,
        }
    }

    /// The application tier (`STYLE_PROVIDER_PRIORITY_USER + 2`): the
    /// application's own sheet, whose rules beat the shared base by
    /// priority. This is the rung [`crate::theme::install_app_stylesheet`]
    /// manages.
    pub fn app_tier() -> Self {
        Self {
            priority: gtk::STYLE_PROVIDER_PRIORITY_USER + 2,
        }
    }

    /// A rung at an explicit priority, for layers above the app sheet.
    /// Conservatory's runtime accent provider installs at
    /// `STYLE_PROVIDER_PRIORITY_USER + 3`; managed this way it can also be
    /// removed or queried instead of being hand-rolled state.
    pub fn at_priority(priority: u32) -> Self {
        Self { priority }
    }

    /// The GTK priority this rung installs at.
    pub fn priority(&self) -> u32 {
        self.priority
    }

    /// Install (or replace) this rung's sheet on the default display.
    /// Returns the new provider, or `None` when there is no display.
    pub fn install(&self, css: &str) -> Option<gtk::CssProvider> {
        install_at(css, self.priority)
    }

    /// Remove this rung's provider from the default display, if one is
    /// installed. Removing an uninstalled rung is a no-op.
    pub fn remove(&self) {
        remove_at(self.priority);
    }

    /// Whether this rung currently carries a sheet on the default display.
    pub fn is_installed(&self) -> bool {
        is_installed_at(self.priority)
    }
}

/// The CSS class a [`StyleScope`] stamps on its target widget and scopes
/// its sheets under. Grep-friendly and deliberately not a name any consumer
/// sheet uses.
pub const SCOPE_CLASS: &str = "vir-style-scope";

/// The priority a scope's display provider installs at: one rung above the
/// app's runtime layers (Conservatory's accent provider sits at `USER + 3`),
/// so a forced subtree re-themes over everything global. Scopes are
/// class-scoped and disjoint, so several can share the rung safely.
pub const STYLE_SCOPE_PRIORITY: u32 = gtk::STYLE_PROVIDER_PRIORITY_USER + 4;

/// The appearance choice a [`StyleScope`] enforces; the shape of a bound
/// `gio::Settings` string key.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ThemeChoice {
    /// Follow the portal's composed global state: no scoped sheet, the
    /// global ladder styles the subtree.
    #[default]
    System,
    /// Force the dark palette regardless of the global state.
    Dark,
    /// Force the light palette regardless of the global state.
    Light,
}

impl ThemeChoice {
    /// Parse the nick forms the portal's resolver already accepts, plus the
    /// explicit `system`: `system`/`default` → `System`,
    /// `dark`/`force-dark` → `Dark`, `light`/`force-light` → `Light`.
    /// Unknown nicks are `None`; the caller picks the fallback (the
    /// [`StyleScope`] binding warns and stands on `System`, the portal's
    /// degrade-loudly rule).
    pub fn from_nick(nick: &str) -> Option<Self> {
        match nick {
            "system" | "default" => Some(Self::System),
            "dark" | "force-dark" => Some(Self::Dark),
            "light" | "force-light" => Some(Self::Light),
            _ => None,
        }
    }

    /// The canonical nick: `system`, `dark`, or `light`. This is what
    /// [`StyleScope::set_choice`] writes back through a bound settings key.
    pub fn nick(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    /// The palette this choice pins, if any: `Some(true)` is dark,
    /// `Some(false)` is light, `None` (`System`) defers to the global state.
    pub fn forced(self) -> Option<bool> {
        match self {
            Self::System => None,
            Self::Dark => Some(true),
            Self::Light => Some(false),
        }
    }

    /// The composed dark state: a forced choice wins, `System` defers to
    /// `global_dark` (the portal's `is_dark()`).
    pub fn resolves_to(self, global_dark: bool) -> bool {
        self.forced().unwrap_or(global_dark)
    }
}

/// Scope a flat stylesheet (the shape `base_css` and the consumer token
/// templates emit: rules and comments, no braces inside comments, no nested
/// at-rules) so its rules only match inside a subtree carrying `class`.
///
/// Every rule is emitted twice: once in descendant form, `.{class} sel`, so
/// it matches the selector inside the scope, and once with the class
/// appended to the selector's final compound, `sel.{class}`, so a rule whose
/// subject IS the scope root still matches (`window.csd` on a window-rooted
/// scope). Comments and declarations pass through verbatim.
///
/// Widgets whose CSS nodes do not descend from the target in the CSS tree
/// (tooltips, and popovers on some shells) keep the globally-installed look;
/// that is a documented edge, not something the transform can reach.
pub fn scope_css(css: &str, class: &str) -> String {
    let mut out = String::with_capacity(css.len() * 2);
    let mut rest = css;
    while let Some(open) = rest.find('{') {
        let (head, after) = rest.split_at(open);
        let Some(close) = after.find('}') else {
            break;
        };
        let decls = &after[1..close];
        // The head carries any comments preceding the rule plus the selector
        // list; only the text after the last comment close is rewritten, so
        // comments pass through verbatim.
        let sel_start = head.rfind("*/").map_or(0, |pos| pos + 2);
        out.push_str(&head[..sel_start]);
        let selectors = head[sel_start..].trim();
        if !selectors.is_empty() {
            let scoped: Vec<String> = selectors
                .split(',')
                .map(|sel| {
                    let sel = sel.trim();
                    format!(".{class} {sel}, {sel}.{class}")
                })
                .collect();
            out.push_str(&scoped.join(", "));
        }
        out.push('{');
        out.push_str(decls);
        out.push('}');
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}

/// The one rule the transform cannot derive from the sheets: the scope
/// root's own canvas, matching the base sheet's `window` rule.
fn scope_root_rule(palette: &Palette) -> String {
    format!(
        ".{SCOPE_CLASS} {{ background-color: {}; color: {}; }}",
        palette.bg_window, palette.fg
    )
}

type ExtraCss = Rc<dyn Fn(&Palette) -> String>;

/// A forced-palette scope over one widget subtree. While the choice is
/// [`ThemeChoice::Dark`] or [`ThemeChoice::Light`], the target carries
/// [`SCOPE_CLASS`] and one display provider at [`STYLE_SCOPE_PRIORITY`]
/// serves the forced palette's sheets scoped under the class: `base_css`,
/// plus the builder's `extra_css` (the app's own divergences, re-spliced),
/// plus a root-canvas rule. [`ThemeChoice::System`] removes the provider and
/// the class, handing the subtree back to the global ladder.
///
/// The scope re-applies only when its choice changes: forced palettes are
/// palette-fixed, and `System` tracks the global state through the app's own
/// display-tier re-splice on portal flips. The provider is display-global
/// and class-scoped, so it dies structurally with the window the target
/// lives in; a `StyleScope` handle is a controller, not an owner, and
/// dropping it changes nothing. The type is `!Send`; keep it on the main
/// thread, next to the widget it scopes.
#[derive(Clone)]
pub struct StyleScope {
    target: glib::WeakRef<gtk::Widget>,
    choice: Rc<Cell<ThemeChoice>>,
    extra: Option<ExtraCss>,
    provider: Rc<RefCell<Option<gtk::CssProvider>>>,
    bound: Rc<RefCell<Option<(gio::Settings, String)>>>,
}

impl StyleScope {
    /// Begin building a scope rooted at `target`: a window, or any widget
    /// subtree that should force its own palette (Conservatory's Now Playing
    /// full-screen is a stack page inside the main window, so the subtree is
    /// the unit, not the window).
    pub fn builder(target: &impl IsA<gtk::Widget>) -> StyleScopeBuilder {
        let weak = glib::WeakRef::<gtk::Widget>::new();
        weak.set(Some(target.upcast_ref::<gtk::Widget>()));
        StyleScopeBuilder {
            target: weak,
            extra: None,
        }
    }

    /// The scope's current choice.
    pub fn choice(&self) -> ThemeChoice {
        self.choice.get()
    }

    /// Enforce `choice` now. When the scope is bound to a settings key
    /// ([`StyleScope::bind_settings`]), the nick is written back through the
    /// key and the change re-applies from there; a failed write warns and
    /// the local choice stands. Unbound scopes apply directly.
    pub fn set_choice(&self, choice: ThemeChoice) {
        let bound = self.bound.borrow().clone();
        if let Some((settings, key)) = bound {
            if let Err(error) = settings.set_string(&key, choice.nick()) {
                glib::g_warning!("vir-gtk", "could not write '{key}': {error}");
            }
        }
        self.choice.set(choice);
        self.apply();
    }

    /// Bind the choice to a `gio::Settings` string key: the key is read
    /// immediately (and applied), re-read on every change, and becomes the
    /// write-back target of [`StyleScope::set_choice`]. Unknown nicks warn
    /// on the `vir-gtk` log domain and stand on [`ThemeChoice::System`].
    /// The key should be a writable string key; the scope keeps the settings
    /// connection alive for its own lifetime.
    pub fn bind_settings(&self, settings: &gio::Settings, key: &str) {
        *self.bound.borrow_mut() = Some((settings.clone(), key.to_string()));
        settings.connect_changed(Some(key), {
            let scope = self.clone();
            move |settings, key| scope.read_choice_from(settings, key)
        });
        self.read_choice_from(settings, key);
    }

    fn read_choice_from(&self, settings: &gio::Settings, key: &str) {
        let nick = settings.string(key);
        let choice = ThemeChoice::from_nick(&nick).unwrap_or_else(|| {
            glib::g_warning!(
                "vir-gtk",
                "unknown theme nick '{nick}' in settings key '{key}'; following the system"
            );
            ThemeChoice::System
        });
        self.choice.set(choice);
        self.apply();
    }

    fn apply(&self) {
        // Detach whatever the scope last installed, wherever the choice
        // lands next; the class follows the same fate.
        if let Some(old) = self.provider.borrow_mut().take() {
            if let Some(display) = gtk::gdk::Display::default() {
                gtk::style_context_remove_provider_for_display(&display, &old);
            }
        }
        let Some(widget) = self.target.upgrade() else {
            return;
        };
        let Some(dark) = self.choice.get().forced() else {
            widget.remove_css_class(SCOPE_CLASS);
            return;
        };
        let Some(display) = gtk::gdk::Display::default() else {
            return;
        };
        let palette = if dark {
            Palette::dragon()
        } else {
            Palette::lotus()
        };
        let mut css = format!(
            "{}\n{}\n",
            scope_root_rule(&palette),
            scope_css(&base_css(&palette), SCOPE_CLASS)
        );
        if let Some(extra) = &self.extra {
            css.push_str(&scope_css(&extra(&palette), SCOPE_CLASS));
            css.push('\n');
        }
        widget.add_css_class(SCOPE_CLASS);
        let provider = gtk::CssProvider::new();
        provider.load_from_string(&css);
        gtk::style_context_add_provider_for_display(&display, &provider, STYLE_SCOPE_PRIORITY);
        *self.provider.borrow_mut() = Some(provider);
    }
}

/// Builder for a [`StyleScope`]; see [`StyleScope::builder`].
pub struct StyleScopeBuilder {
    target: glib::WeakRef<gtk::Widget>,
    extra: Option<ExtraCss>,
}

impl StyleScopeBuilder {
    /// App-supplied divergence CSS, spliced with the forced palette and
    /// scoped under [`SCOPE_CLASS`] like the base sheet. Without it a forced
    /// subtree wears the unanimous base; with it the subtree keeps the app's
    /// own look (Conservatory re-splices its app template here, tokens and
    /// all).
    pub fn extra_css<F>(mut self, f: F) -> Self
    where
        F: Fn(&Palette) -> String + 'static,
    {
        self.extra = Some(Rc::new(f));
        self
    }

    /// Materialize the scope with [`ThemeChoice::System`]: nothing installs
    /// until a choice forces a palette (or a bound key supplies one).
    pub fn build(self) -> StyleScope {
        let scope = StyleScope {
            target: self.target,
            choice: Rc::new(Cell::new(ThemeChoice::System)),
            extra: self.extra,
            provider: Rc::new(RefCell::new(None)),
            bound: Rc::new(RefCell::new(None)),
        };
        scope.apply();
        scope
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::base_css;

    #[test]
    fn ladder_rungs_sit_at_the_documented_priorities() {
        assert_eq!(
            StyleManager::crate_tier().priority(),
            gtk::STYLE_PROVIDER_PRIORITY_USER + 1
        );
        assert_eq!(
            StyleManager::app_tier().priority(),
            gtk::STYLE_PROVIDER_PRIORITY_USER + 2
        );
        assert_eq!(StyleManager::at_priority(1234).priority(), 1234);
    }

    #[test]
    fn clones_share_their_rung() {
        let tier = StyleManager::app_tier();
        let clone = tier.clone();
        assert_eq!(tier.priority(), clone.priority());
        assert_eq!(tier, clone);
    }

    #[test]
    fn teardown_and_queries_are_safe_without_a_display() {
        // No test initializes GTK, so `Display::default()` is `None` here:
        // the whole lifecycle must degrade to no-ops rather than panic.
        // This is also the headless path real callers hit in CI.
        let tier = StyleManager::crate_tier();
        assert!(!tier.is_installed());
        tier.remove();
        tier.remove();
        assert!(!tier.is_installed());
    }

    #[test]
    fn theme_choice_parses_the_portal_nick_forms() {
        assert_eq!(ThemeChoice::from_nick("system"), Some(ThemeChoice::System));
        assert_eq!(ThemeChoice::from_nick("default"), Some(ThemeChoice::System));
        for nick in ["dark", "force-dark"] {
            assert_eq!(ThemeChoice::from_nick(nick), Some(ThemeChoice::Dark));
        }
        for nick in ["light", "force-light"] {
            assert_eq!(ThemeChoice::from_nick(nick), Some(ThemeChoice::Light));
        }
        for nick in ["", "Dark", "darrk", "prefer-dark", "auto"] {
            assert_eq!(ThemeChoice::from_nick(nick), None, "{nick:?} is unknown");
        }
    }

    #[test]
    fn theme_choice_nicks_round_trip() {
        for choice in [ThemeChoice::System, ThemeChoice::Dark, ThemeChoice::Light] {
            assert_eq!(ThemeChoice::from_nick(choice.nick()), Some(choice));
        }
    }

    #[test]
    fn forced_choices_win_and_system_defers() {
        assert_eq!(ThemeChoice::System.forced(), None);
        assert_eq!(ThemeChoice::Dark.forced(), Some(true));
        assert_eq!(ThemeChoice::Light.forced(), Some(false));
        assert!(ThemeChoice::System.resolves_to(true));
        assert!(!ThemeChoice::System.resolves_to(false));
        assert!(ThemeChoice::Dark.resolves_to(false));
        assert!(!ThemeChoice::Light.resolves_to(true));
    }

    #[test]
    fn scope_priority_sits_one_rung_above_the_app_runtime_layers() {
        // Conservatory's accent provider installs at USER + 3; a forced
        // scope must re-theme over it.
        assert_eq!(STYLE_SCOPE_PRIORITY, gtk::STYLE_PROVIDER_PRIORITY_USER + 4);
    }

    #[test]
    fn scope_css_prefixes_descendants_and_appends_the_root_form() {
        let out = scope_css("button { color: red; }", "sc");
        assert!(out.contains(".sc button"), "{out}");
        assert!(out.contains("button.sc"), "{out}");
        assert!(out.ends_with("}"));
    }

    #[test]
    fn scope_css_handles_lists_children_and_pseudo_classes() {
        let out = scope_css(
            "window, .background { background: x; }\n.linked > button:not(:first-child) { border-left-width: 0; }",
            "sc",
        );
        assert!(out.contains("window.sc"), "{out}");
        assert!(out.contains(".background.sc"), "{out}");
        assert!(
            out.contains(".sc .linked > button:not(:first-child)"),
            "{out}"
        );
        assert!(out.contains("button:not(:first-child).sc"), "{out}");
    }

    #[test]
    fn scope_css_keeps_comments_and_declarations_verbatim() {
        let css = "/* a comment */\nentry { font-size: 82%; color: #fff; }";
        let out = scope_css(css, "sc");
        assert!(out.contains("/* a comment */"), "{out}");
        assert!(out.contains("font-size: 82%;"), "{out}");
        assert!(out.contains("color: #fff;"), "{out}");
    }

    #[test]
    fn scoped_base_sheet_matches_the_scope_and_leaves_no_token() {
        // The literal % in font-size: 82% is legitimate; a %UPPERCASE_%
        // span after substitution is not (the base_css contract, re-pinned
        // through the scoping transform).
        let has_raw_token = |css: &str| {
            css.char_indices().any(|(i, c)| {
                if c != '%' {
                    return false;
                }
                match css[i + 1..].find('%') {
                    Some(end) => {
                        let inner = &css[i + 1..i + 1 + end];
                        !inner.is_empty()
                            && inner.chars().all(|c| c.is_ascii_uppercase() || c == '_')
                    }
                    None => false,
                }
            })
        };
        let out = scope_css(&base_css(&Palette::dragon()), SCOPE_CLASS);
        assert!(out.contains(&format!(".{SCOPE_CLASS} button")), "{out}");
        assert!(out.contains(&format!("button.{SCOPE_CLASS}")), "{out}");
        assert!(!has_raw_token(&out), "raw token survived scoped base_css");
    }

    #[test]
    fn scope_root_rule_carries_the_window_canvas() {
        let rule = scope_root_rule(&Palette::lotus());
        assert_eq!(
            rule,
            ".vir-style-scope { background-color: #e7dba0; color: #545464; }"
        );
    }

    #[test]
    fn scope_css_of_an_empty_sheet_is_empty() {
        assert_eq!(scope_css("", "sc"), "");
    }
}
