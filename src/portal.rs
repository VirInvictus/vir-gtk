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
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

type Listener = (glib::WeakRef<glib::Object>, Rc<dyn Fn(bool)>);

static INITIALIZED: AtomicBool = AtomicBool::new(false);
static SYSTEM_DARK: AtomicBool = AtomicBool::new(true);
static RESOLVED_DARK: AtomicBool = AtomicBool::new(true);
static DEFAULT_DARK: AtomicBool = AtomicBool::new(true);
/// Bumped every time a live `SettingChanged` signal applies. A read reply
/// carries the generation it was issued at and is dropped as stale if the
/// generation moved meanwhile, so a slow `ReadOne`/`Read` reply can never
/// overwrite fresher signal state (the 2026-09-12 audit's initial-read
/// race).
static PORTAL_GENERATION: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static SETTINGS: RefCell<Option<(gio::Settings, String)>> = const { RefCell::new(None) };
    /// Held for the process lifetime so the portal signal subscription stays
    /// alive; dropping the connection would detach the `SettingChanged`
    /// listener.
    static CONNECTION: RefCell<Option<gio::DBusConnection>> = const { RefCell::new(None) };
    /// The strong subscription handle gio 0.22 hands back from
    /// `subscribe_to_signal`; dropping it unsubscribes, so it lives next
    /// to CONNECTION for the process lifetime.
    static SUBSCRIPTION: RefCell<Option<gio::SignalSubscription>> = const { RefCell::new(None) };
    /// Same deal for the bus-daemon `NameOwnerChanged` watcher that re-reads
    /// on portal (re)start.
    static OWNER_SUBSCRIPTION: RefCell<Option<gio::SignalSubscription>> = const { RefCell::new(None) };
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

/// Extract the `color-scheme` value from a `SettingChanged` signal body:
/// `Some(scheme)` only when the body is well-formed AND names
/// `org.freedesktop.appearance` / `color-scheme`; `None` otherwise.
///
/// Every child access goes through `try_child_value`: the signal body is
/// whatever the sender put on the bus, and `child_value` panics on an
/// out-of-range index. A malformed body must degrade to "not our key", never
/// panic into the consumer's main loop.
fn setting_changed_scheme(params: &glib::Variant) -> Option<u32> {
    let ns = params.try_child_value(0)?.get::<String>()?;
    let key = params.try_child_value(1)?.get::<String>()?;
    if ns != "org.freedesktop.appearance" || key != "color-scheme" {
        return None;
    }
    params.try_child_value(2)?.as_variant()?.get::<u32>()
}

/// Whether a `NameOwnerChanged` body says the portal name was just acquired
/// (`Some(scheme)`-shaped guard for the re-read): only when the acquired name
/// is `org.freedesktop.portal.Desktop` and the new owner is non-empty. The
/// portal appearing (at startup, or after a restart) is the cue to re-read;
/// its disappearing needs no action, the next acquisition re-reads.
fn portal_name_acquired(params: &glib::Variant) -> bool {
    let Some(name) = params.try_child_value(0).and_then(|v| v.get::<String>()) else {
        return false;
    };
    if name != "org.freedesktop.portal.Desktop" {
        return false;
    }
    params
        .try_child_value(2)
        .and_then(|v| v.get::<String>())
        .is_some_and(|owner| !owner.is_empty())
}

/// A read reply is stale when the generation advanced between issuing the
/// read and the reply landing; the reply then carries pre-signal data and
/// must be dropped.
fn read_reply_is_stale(issued_generation: u64) -> bool {
    PORTAL_GENERATION.load(Ordering::Relaxed) != issued_generation
}

fn portal_generation() -> u64 {
    PORTAL_GENERATION.load(Ordering::Relaxed)
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

/// The raw system preference, before composition against the application's
/// `gio::Settings`: what the desktop environment last broadcast (or the
/// application default before the portal's answer lands). Safe from any
/// thread.
pub fn system_is_dark() -> bool {
    get(&SYSTEM_DARK)
}

/// The composed dark/light state: the portal's system preference composed
/// against the application's bound settings key (`force-dark` and
/// `force-light` win; the application default stands until the portal's
/// answer arrives). Safe from any thread.
///
/// Note the state moves asynchronously: right after [`init()`] this is the
/// application default, and the portal's answer (and every later system flip)
/// lands through the main loop. Register a listener with
/// [`connect_dark_changed()`] to re-splice styles when it moves.
pub fn is_dark() -> bool {
    get(&RESOLVED_DARK)
}

/// Re-run the composition immediately from the stored settings key and the
/// current system preference, broadcasting to listeners only when the
/// composed state changed. Safe from the main thread; call after mutating a
/// bound settings key directly.
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

/// Register `f` to fire whenever the composed dark/light state changes. The
/// `owner` is held as a weak reference, so callbacks bound to UI elements
/// stop firing (and are pruned at the next broadcast) when the element dies.
/// Callbacks run on the thread that runs the main loop.
///
/// This is the hook that keeps styles live: `init()` seeds the initial state
/// and the portal listener, and each state change broadcasts to these
/// callbacks, so an application that wants its sheets to follow the system
/// re-splices inside the callback (or calls [`crate::theme::install_default`]
/// and skips the manual loop).
pub fn connect_dark_changed<F>(owner: &impl IsA<glib::Object>, f: F)
where
    F: Fn(bool) + 'static,
{
    let weak = owner.upcast_ref::<glib::Object>().downgrade();
    LISTENERS.with(|l| l.borrow_mut().push((weak, Rc::new(f))));
}

/// Fire the live listeners. Each callback is cloned out of the `LISTENERS`
/// borrow and fired outside it: a listener that re-enters the portal
/// mid-broadcast (a `resolve_now()` after a state change, or a
/// [`connect_dark_changed`] from inside a callback) needs mutable access to
/// `LISTENERS`, and holding the borrow across the call panicked the `RefCell`
/// (the 1.0.3 bug). The pass itself iterates by index over a length sampled up
/// front, so a nested broadcast still runs to completion, and a listener
/// registered mid-pass joins the next broadcast rather than this one.
fn broadcast(dark: bool) {
    let len = LISTENERS.with(|l| {
        let mut l = l.borrow_mut();
        l.retain(|(owner, _)| owner.upgrade().is_some());
        l.len()
    });
    for i in 0..len {
        let listener = LISTENERS.with(|l| {
            l.borrow()
                .get(i)
                .map(|(owner, f)| (owner.clone(), Rc::clone(f)))
        });
        if let Some((owner, f)) = listener {
            if owner.upgrade().is_some() {
                f(dark);
            }
        }
    }
}

/// Initialize the portal: seed the state, wire the optional settings
/// composition, and start the asynchronous D-Bus work. Call once during
/// application startup, from the main thread; it returns without blocking,
/// and the portal's current scheme arrives through the main loop.
///
/// `settings` + `settings_key` bind the composition to a `gio::Settings`
/// string key (e.g. `"theme"` holding `system`/`dark`/`light`/
/// `force-dark`/`force-light`); explicit overrides beat the system
/// broadcast. `default_dark` is the pre-portal and no-preference fallback.
///
/// Side effect, documented because applications that manage it themselves
/// would silently fight this crate: on every composed change the global
/// `GtkSettings:gtk-application-prefer-dark-theme` is set to the resolved
/// state. Applications needing exclusive control of that key should not
/// compose through this crate, or should re-assert their value after portal
/// flips.
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
        let conn = match res {
            Ok(conn) => conn,
            Err(error) => {
                // Headless sessions and broken portal daemons land here: the
                // application default stands either way, but silent
                // degradation hides real breakage.
                glib::g_warning!("vir-gtk", "portal settings unavailable: {error}");
                return;
            }
        };
        let subscription = conn.subscribe_to_signal(
            Some("org.freedesktop.portal.Desktop"),
            Some("org.freedesktop.portal.Settings"),
            Some("SettingChanged"),
            Some("/org/freedesktop/portal/desktop"),
            None,
            gio::DBusSignalFlags::NONE,
            |sig| {
                let Some(scheme) = setting_changed_scheme(sig.parameters) else {
                    return;
                };
                bump_portal_generation();
                apply_portal_scheme(scheme);
            },
        );
        // Watch the bus daemon for the portal name appearing: the first
        // arrival (the portal can start after the application) and every
        // restart after a crash trigger a fresh read, so changes made while
        // no portal was up are picked up instead of being lost until the
        // next signal.
        let reread_conn = conn.clone();
        let owner_subscription = conn.subscribe_to_signal(
            Some("org.freedesktop.DBus"),
            Some("org.freedesktop.DBus"),
            Some("NameOwnerChanged"),
            Some("/org/freedesktop/DBus"),
            Some("org.freedesktop.portal.Desktop"),
            gio::DBusSignalFlags::NONE,
            move |sig| {
                if portal_name_acquired(sig.parameters) {
                    read_portal_scheme_async(&reread_conn);
                }
            },
        );
        read_portal_scheme_async(&conn);
        CONNECTION.with(|b| *b.borrow_mut() = Some(conn));
        SUBSCRIPTION.with(|b| *b.borrow_mut() = Some(subscription));
        // Held next to the SettingChanged subscription for the same reason:
        // dropping it would unsubscribe the restart watcher.
        OWNER_SUBSCRIPTION.with(|b| *b.borrow_mut() = Some(owner_subscription));
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

/// Apply a `ReadOne`/`Read` reply, dropping it as stale when a live signal
/// applied after the read was issued: the reply then carries pre-signal data,
/// and applying it would undo the newer state.
fn apply_read_reply(issued_generation: u64, scheme: u32) {
    if read_reply_is_stale(issued_generation) {
        return;
    }
    let dark = portal_scheme_preference(scheme).unwrap_or_else(|| get(&DEFAULT_DARK));
    set(&SYSTEM_DARK, dark);
    re_resolve();
}

/// A live `SettingChanged` apply invalidates every in-flight read reply.
fn bump_portal_generation() {
    PORTAL_GENERATION.fetch_add(1, Ordering::Relaxed);
}

fn read_portal_scheme_async(conn: &gio::DBusConnection) {
    let args = ("org.freedesktop.appearance", "color-scheme").to_variant();
    let Ok(reply_ty) = glib::VariantTy::new("(v)") else {
        return;
    };
    // Stamped on both reply paths: a signal applying meanwhile bumps the
    // generation and the reply drops instead of overwriting fresher state.
    let generation = portal_generation();
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
                    apply_read_reply(generation, scheme);
                }
            }
            Err(_) => {
                // Older portals only implement Read, whose reply nests the
                // value variant one level deeper. The generation is taken
                // fresh: state may have moved during the ReadOne attempt.
                let args = ("org.freedesktop.appearance", "color-scheme").to_variant();
                let Ok(reply_ty) = glib::VariantTy::new("(v)") else {
                    return;
                };
                let generation = portal_generation();
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
                    move |res| match res {
                        Ok(reply) => {
                            let scheme = reply
                                .child_value(0)
                                .as_variant()
                                .and_then(|v| v.as_variant())
                                .and_then(|v| v.get::<u32>());
                            if let Some(scheme) = scheme {
                                apply_read_reply(generation, scheme);
                            }
                        }
                        Err(read_error) => {
                            // Both read shapes failed: the application
                            // default stands, but silent degradation hides
                            // real breakage (no portal, or a broken one).
                            glib::g_warning!(
                                "vir-gtk",
                                "portal color-scheme read failed on both \
                                 ReadOne and Read: {read_error}; standing on \
                                 the application default"
                            );
                        }
                    },
                );
            }
        },
    );
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use gtk4::glib::object::{Cast, ObjectExt};
    use gtk4::prelude::*;
    use gtk4::{gio, glib};

    use super::{portal_scheme_preference, resolve_is_dark, LISTENERS};

    /// Register a live listener against a `Cancellable` owner (a plain
    /// GObject, no display needed) and leak the owner so the weak ref stays
    /// upgradeable for the length of the test. libtest may run tests on a
    /// shared thread, so each test starts from an empty registry.
    fn register_listener(f: impl Fn(bool) + 'static) {
        let owner = gio::Cancellable::new();
        let weak = owner.upcast_ref::<glib::Object>().downgrade();
        LISTENERS.with(|l| l.borrow_mut().push((weak, Rc::new(f))));
        std::mem::forget(owner);
    }

    fn clear_listeners() {
        LISTENERS.with(|l| *l.borrow_mut() = Vec::new());
    }

    #[test]
    fn broadcast_survives_a_re_entrant_listener() {
        // The 1.0.3 loop held the `LISTENERS` borrow across each callback,
        // so a listener whose nested `broadcast()` needed the borrow panicked
        // with `BorrowMutError` even though the index iteration was supposed
        // to make re-entrancy safe.
        clear_listeners();
        let fired = Rc::new(RefCell::new(Vec::new()));
        let reentered = Rc::new(Cell::new(false));

        let first_fired = fired.clone();
        let first_reentered = reentered.clone();
        register_listener(move |dark| {
            first_fired.borrow_mut().push(("first", dark));
            if !first_reentered.replace(true) {
                super::broadcast(!dark);
            }
        });
        let second_fired = fired.clone();
        register_listener(move |dark| second_fired.borrow_mut().push(("second", dark)));

        super::broadcast(true);

        // The nested pass ran to completion inside the first callback and
        // the outer pass resumed where it left off.
        assert_eq!(
            *fired.borrow(),
            vec![
                ("first", true),
                ("first", false),
                ("second", false),
                ("second", true)
            ]
        );
    }

    #[test]
    fn broadcast_survives_a_mid_pass_registration() {
        // The same 1.0.3 borrow bug from the registration side: a callback
        // that called `connect_dark_changed()` mid-broadcast panicked the
        // `RefCell` instead of joining the next pass.
        clear_listeners();
        let fired = Rc::new(RefCell::new(Vec::<String>::new()));
        let registered = Rc::new(Cell::new(false));

        let first_fired = fired.clone();
        let first_registered = registered.clone();
        let late_for_first = fired.clone();
        register_listener(move |dark| {
            first_fired.borrow_mut().push(format!("first {dark}"));
            if !first_registered.replace(true) {
                let late_fired = late_for_first.clone();
                register_listener(move |d| late_fired.borrow_mut().push(format!("late {d}")));
            }
        });
        let second_fired = fired.clone();
        register_listener(move |dark| second_fired.borrow_mut().push(format!("second {dark}")));

        // The pass already running is undisturbed: the late listener is not
        // called by it (the length was sampled up front).
        super::broadcast(true);
        assert_eq!(*fired.borrow(), vec!["first true", "second true"]);

        // The late listener joins the next broadcast.
        super::broadcast(false);
        assert_eq!(
            *fired.borrow(),
            vec![
                "first true",
                "second true",
                "first false",
                "second false",
                "late false"
            ]
        );
    }

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

    fn setting_changed_variant(ns: &str, key: &str, scheme: u32) -> glib::Variant {
        // The real signal body is (ssv): the scheme arrives boxed in a
        // variant, which is why the reader goes through as_variant().
        glib::Variant::tuple_from_iter([
            ns.to_variant(),
            key.to_variant(),
            glib::Variant::from_variant(&scheme.to_variant()),
        ])
    }

    #[test]
    fn setting_changed_extracts_the_color_scheme_value() {
        let body = setting_changed_variant("org.freedesktop.appearance", "color-scheme", 1);
        assert_eq!(super::setting_changed_scheme(&body), Some(1));
        let body = setting_changed_variant("org.freedesktop.appearance", "color-scheme", 0);
        assert_eq!(super::setting_changed_scheme(&body), Some(0));
    }

    #[test]
    fn setting_changed_ignores_other_keys_and_malformed_bodies() {
        // Right shape, wrong namespace/key: not ours, no apply.
        for (ns, key) in [
            ("org.freedesktop.appearance", "accent-color"),
            ("org.example", "color-scheme"),
        ] {
            let body = setting_changed_variant(ns, key, 1);
            assert_eq!(
                super::setting_changed_scheme(&body),
                None,
                "{ns}/{key} must not match"
            );
        }
        // Malformed bodies (a hostile or broken sender): no panic, no apply.
        // Each of these panicked via `child_value` asserts before the
        // try_child_value hardening.
        let empty: glib::Variant =
            glib::Variant::tuple_from_iter(std::iter::empty::<glib::Variant>());
        assert_eq!(super::setting_changed_scheme(&empty), None);
        let short = glib::Variant::tuple_from_iter(["org.freedesktop.appearance".to_variant()]);
        assert_eq!(super::setting_changed_scheme(&short), None);
        let wrong_types = glib::Variant::tuple_from_iter([
            1u32.to_variant(),
            2u32.to_variant(),
            3u32.to_variant(),
        ]);
        assert_eq!(super::setting_changed_scheme(&wrong_types), None);
        let bad_scheme =
            ("org.freedesktop.appearance", "color-scheme", "not-a-scheme").to_variant();
        assert_eq!(super::setting_changed_scheme(&bad_scheme), None);
    }

    #[test]
    fn name_owner_changed_triggers_reread_only_for_the_portal() {
        let acquired = ("org.freedesktop.portal.Desktop", "", "cafe1234").to_variant();
        assert!(super::portal_name_acquired(&acquired));
        // Vanishing: no re-read (the next acquisition does it).
        let lost = ("org.freedesktop.portal.Desktop", "cafe1234", "").to_variant();
        assert!(!super::portal_name_acquired(&lost));
        // Some other name on the bus.
        let other = ("org.example.Other", "", "beef5678").to_variant();
        assert!(!super::portal_name_acquired(&other));
        // Malformed body: no panic, no re-read.
        let empty: glib::Variant =
            glib::Variant::tuple_from_iter(std::iter::empty::<glib::Variant>());
        assert!(!super::portal_name_acquired(&empty));
    }

    #[test]
    fn a_signal_apply_invalidates_in_flight_read_replies() {
        // The stale-read race: a ReadOne reply landing after a newer
        // SettingChanged must be dropped, not applied over it.
        let issued = super::portal_generation();
        assert!(!super::read_reply_is_stale(issued));
        super::bump_portal_generation();
        assert!(super::read_reply_is_stale(issued));
        // A reply issued after the bump is fresh again.
        let reissued = super::portal_generation();
        assert!(!super::read_reply_is_stale(reissued));
    }
}
