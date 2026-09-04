// SPDX-License-Identifier: MIT
//! Dark/light resolution over `org.freedesktop.portal.Settings`.
//!
//! State shape, deliberately split: the boolean state lives in global atomics
//! so any thread (a background decoder, an async worker) can read
//! [`is_dark()`]/[`system_is_dark()`] without touching the main thread; the
//! GTK-bound pieces stay in thread-locals because `gio::Settings`, the D-Bus
//! connection, and listener closures capturing widgets are all `!Send` and are
//! only ever touched on the thread that called [`init()`].
//!
//! `init()` never blocks: the portal read goes out asynchronously and the
//! answer lands through the main loop, so startup renders on the app's default
//! immediately instead of waiting up to a full D-Bus timeout.

use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};

type Listener = (glib::WeakRef<glib::Object>, Box<dyn Fn(bool)>);

static INITIALIZED: AtomicBool = AtomicBool::new(false);
static SYSTEM_DARK: AtomicBool = AtomicBool::new(true);
static RESOLVED_DARK: AtomicBool = AtomicBool::new(true);
static DEFAULT_DARK: AtomicBool = AtomicBool::new(true);

thread_local! {
    static SETTINGS: RefCell<Option<(gio::Settings, String)>> = const { RefCell::new(None) };
    /// Held for the process lifetime so the portal signal subscription stays
    /// alive; dropping the connection would detach the `SettingChanged`
    /// listener.
    static CONNECTION: RefCell<Option<gio::DBusConnection>> = const { RefCell::new(None) };
    static LISTENERS: RefCell<Vec<Listener>> = const { RefCell::new(Vec::new()) };
}

/// The freedesktop `color-scheme` preference: `1` prefers dark, `2` prefers
/// light, anything else (including `0`, "no preference") expresses nothing and
/// the application's default stands.
fn portal_scheme_preference(scheme: u32) -> Option<bool> {
    match scheme {
        1 => Some(true),
        2 => Some(false),
        _ => None,
    }
}

fn resolve_is_dark(nick: &str, system_dark: bool) -> bool {
    match nick {
        "force-light" | "light" => false,
        "force-dark" | "dark" => true,
        _ => system_dark,
    }
}

fn get(atom: &AtomicBool) -> bool {
    atom.load(Ordering::Relaxed)
}

fn set(atom: &AtomicBool, value: bool) {
    atom.store(value, Ordering::Relaxed);
}

pub fn system_is_dark() -> bool {
    get(&SYSTEM_DARK)
}

pub fn is_dark() -> bool {
    get(&RESOLVED_DARK)
}

pub fn resolve_now() {
    re_resolve();
}

fn re_resolve() {
    let nick = SETTINGS.with(|s| {
        let s_ref = s.borrow();
        if let Some((s, key)) = s_ref.as_ref() {
            Some(s.string(key).to_string())
        } else {
            None
        }
    });

    let dark = match nick {
        Some(nick) => resolve_is_dark(&nick, system_is_dark()),
        None => system_is_dark(),
    };

    if get(&RESOLVED_DARK) == dark && get(&INITIALIZED) {
        return;
    }
    set(&RESOLVED_DARK, dark);

    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(dark);
    }
    broadcast(dark);
}

pub fn connect_dark_changed<F>(owner: &impl IsA<glib::Object>, f: F)
where
    F: Fn(bool) + 'static,
{
    let weak = owner.upcast_ref::<glib::Object>().downgrade();
    LISTENERS.with(|l| l.borrow_mut().push((weak, Box::new(f))));
}

/// Fire the live listeners. Iterates by index over a length sampled up front
/// (no `mem::take`), so a callback that mutates state and re-enters
/// [`broadcast`] still reaches every listener, and a callback that registers a
/// new listener neither panics the `RefCell` nor gets skipped by the pass that
/// is already running.
fn broadcast(dark: bool) {
    let len = LISTENERS.with(|l| {
        let mut l = l.borrow_mut();
        l.retain(|(owner, _)| owner.upgrade().is_some());
        l.len()
    });
    for i in 0..len {
        LISTENERS.with(|l| {
            let l = l.borrow();
            if let Some((owner, f)) = l.get(i) {
                if owner.upgrade().is_some() {
                    f(dark);
                }
            }
        });
    }
}

pub fn init(settings: Option<gio::Settings>, settings_key: Option<&str>, default_dark: bool) {
    if get(&INITIALIZED) {
        return;
    }
    set(&DEFAULT_DARK, default_dark);
    set(&SYSTEM_DARK, default_dark);
    set(&RESOLVED_DARK, default_dark);

    let key = settings_key.unwrap_or("theme").to_string();
    if let Some(settings) = settings {
        settings.connect_changed(Some(&key), |_, _| re_resolve());
        SETTINGS.with(|s| *s.borrow_mut() = Some((settings, key)));
    }

    gio::bus_get(gio::BusType::Session, gio::Cancellable::NONE, |res| {
        let Ok(conn) = res else {
            return;
        };
        conn.signal_subscribe(
            Some("org.freedesktop.portal.Desktop"),
            Some("org.freedesktop.portal.Settings"),
            Some("SettingChanged"),
            Some("/org/freedesktop/portal/desktop"),
            None,
            gio::DBusSignalFlags::NONE,
            |_, _, _, _, _, params| {
                let ns = params.child_value(0).get::<String>();
                let key = params.child_value(1).get::<String>();
                if ns.as_deref() != Some("org.freedesktop.appearance")
                    || key.as_deref() != Some("color-scheme")
                {
                    return;
                }
                let Some(scheme) = params
                    .child_value(2)
                    .as_variant()
                    .and_then(|v| v.get::<u32>())
                else {
                    return;
                };
                apply_portal_scheme(scheme);
            },
        );
        read_portal_scheme_async(&conn);
        CONNECTION.with(|b| *b.borrow_mut() = Some(conn));
    });
    re_resolve();
    set(&INITIALIZED, true);
}

fn apply_portal_scheme(scheme: u32) {
    // "No preference" falls back to the application default instead of
    // forcing light: the portal expressing nothing must not override the
    // caller's `default_dark`.
    let dark = portal_scheme_preference(scheme).unwrap_or_else(|| get(&DEFAULT_DARK));
    set(&SYSTEM_DARK, dark);
    re_resolve();
}

fn read_portal_scheme_async(conn: &gio::DBusConnection) {
    let args = ("org.freedesktop.appearance", "color-scheme").to_variant();
    let Ok(reply_ty) = glib::VariantTy::new("(v)") else {
        return;
    };
    let fallback_conn = conn.clone();
    conn.call(
        Some("org.freedesktop.portal.Desktop"),
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Settings",
        "ReadOne",
        Some(&args),
        Some(reply_ty),
        gio::DBusCallFlags::NONE,
        1000,
        gio::Cancellable::NONE,
        move |res| match res {
            Ok(reply) => {
                if let Some(scheme) = reply
                    .child_value(0)
                    .as_variant()
                    .and_then(|v| v.get::<u32>())
                {
                    apply_portal_scheme(scheme);
                }
            }
            Err(_) => {
                // Older portals only implement Read, whose reply nests the
                // value variant one level deeper.
                let args = ("org.freedesktop.appearance", "color-scheme").to_variant();
                let Ok(reply_ty) = glib::VariantTy::new("(v)") else {
                    return;
                };
                fallback_conn.call(
                    Some("org.freedesktop.portal.Desktop"),
                    "/org/freedesktop/portal/desktop",
                    "org.freedesktop.portal.Settings",
                    "Read",
                    Some(&args),
                    Some(reply_ty),
                    gio::DBusCallFlags::NONE,
                    1000,
                    gio::Cancellable::NONE,
                    move |res| {
                        if let Ok(reply) = res {
                            let scheme = reply
                                .child_value(0)
                                .as_variant()
                                .and_then(|v| v.as_variant())
                                .and_then(|v| v.get::<u32>());
                            if let Some(scheme) = scheme {
                                apply_portal_scheme(scheme);
                            }
                        }
                    },
                );
            }
        },
    );
}

#[cfg(test)]
mod tests {
    use super::{portal_scheme_preference, resolve_is_dark};

    #[test]
    fn portal_scheme_uses_freedesktop_color_scheme_values() {
        // org.freedesktop.appearance color-scheme: 0 = no preference,
        // 1 = dark, 2 = light. Anything else expresses nothing, and the
        // application default stands (this is the 1.0.3 fix: scheme 0 used
        // to resolve as light and override `default_dark`).
        assert_eq!(portal_scheme_preference(1), Some(true));
        assert_eq!(portal_scheme_preference(2), Some(false));
        assert_eq!(portal_scheme_preference(0), None);
        assert_eq!(portal_scheme_preference(42), None);
    }

    #[test]
    fn explicit_nicks_override_the_system_preference() {
        for nick in ["force-dark", "dark"] {
            assert!(resolve_is_dark(nick, false), "{nick} must force dark");
            assert!(resolve_is_dark(nick, true), "{nick} must force dark");
        }
        for nick in ["force-light", "light"] {
            assert!(!resolve_is_dark(nick, true), "{nick} must force light");
            assert!(!resolve_is_dark(nick, false), "{nick} must force light");
        }
    }

    #[test]
    fn unknown_nicks_follow_the_system_preference() {
        for nick in ["system", "default", "", "Dark"] {
            assert!(resolve_is_dark(nick, true), "{nick:?} must follow system");
            assert!(!resolve_is_dark(nick, false), "{nick:?} must follow system");
        }
    }

    #[test]
    fn state_atomics_are_readable_off_the_main_thread() {
        // The 1.0.3 shape: the boolean state is global, so a background
        // thread reads real values instead of thread-local defaults.
        std::thread::scope(|s| {
            s.spawn(|| {
                let _ = super::is_dark();
                let _ = super::system_is_dark();
            });
        });
    }
}
