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

use std::cell::RefCell;

use gtk4 as gtk;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
