// SPDX-License-Identifier: MIT
//! Dark/light resolution over `org.freedesktop.portal.Settings`.

use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use std::cell::{Cell, RefCell};

type Listener = (glib::WeakRef<glib::Object>, Box<dyn Fn(bool)>);

fn portal_scheme_is_dark(scheme: u32) -> bool {
    scheme == 1
}

fn resolve_is_dark(nick: &str, system_dark: bool) -> bool {
    match nick {
        "force-light" | "light" => false,
        "force-dark" | "dark" => true,
        _ => system_dark,
    }
}

thread_local! {
    static INITIALIZED: Cell<bool> = const { Cell::new(false) };
    static SYSTEM_DARK: Cell<bool> = const { Cell::new(true) };
    static RESOLVED_DARK: Cell<bool> = const { Cell::new(true) };
    static SETTINGS: RefCell<Option<gio::Settings>> = const { RefCell::new(None) };
    static SETTINGS_KEY: RefCell<Option<String>> = const { RefCell::new(None) };
    static BUS: RefCell<Option<gio::DBusConnection>> = const { RefCell::new(None) };
    static LISTENERS: RefCell<Vec<Listener>> = const { RefCell::new(Vec::new()) };
}

pub fn system_is_dark() -> bool {
    SYSTEM_DARK.with(Cell::get)
}

pub fn is_dark() -> bool {
    RESOLVED_DARK.with(Cell::get)
}

pub fn resolve_now() {
    re_resolve();
}

fn re_resolve() {
    let nick = SETTINGS.with(|s| {
        let s_ref = s.borrow();
        if let Some(s) = s_ref.as_ref() {
            let key =
                SETTINGS_KEY.with(|k| k.borrow().clone().unwrap_or_else(|| "theme".to_string()));
            Some(s.string(&key).to_string())
        } else {
            None
        }
    });

    let dark = match nick {
        Some(nick) => resolve_is_dark(&nick, system_is_dark()),
        None => system_is_dark(),
    };

    if RESOLVED_DARK.with(Cell::get) == dark && INITIALIZED.with(Cell::get) {
        return;
    }
    RESOLVED_DARK.with(|d| d.set(dark));

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

fn broadcast(dark: bool) {
    let mut listeners = LISTENERS.with(|l| std::mem::take(&mut *l.borrow_mut()));
    listeners.retain(|(owner, _)| owner.upgrade().is_some());
    for (_, f) in &listeners {
        f(dark);
    }
    LISTENERS.with(|l| {
        let mut current = l.borrow_mut();
        listeners.append(&mut current);
        *current = listeners;
    });
}

pub fn init(settings: Option<gio::Settings>, settings_key: Option<&str>, default_dark: bool) {
    if INITIALIZED.with(Cell::get) {
        return;
    }
    SYSTEM_DARK.with(|d| d.set(default_dark));
    RESOLVED_DARK.with(|d| d.set(default_dark));

    let key = settings_key.unwrap_or("theme").to_string();
    if let Some(settings) = settings {
        settings.connect_changed(Some(&key), |_, _| re_resolve());
        SETTINGS.with(|s| *s.borrow_mut() = Some(settings));
        SETTINGS_KEY.with(|k| *k.borrow_mut() = Some(key));
    }

    if let Ok(conn) = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE) {
        if let Some(scheme) = read_portal_scheme(&conn) {
            SYSTEM_DARK.with(|d| d.set(portal_scheme_is_dark(scheme)));
        }
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
                let Some(dark) = params
                    .child_value(2)
                    .as_variant()
                    .and_then(|v| v.get::<u32>())
                    .map(portal_scheme_is_dark)
                else {
                    return;
                };
                SYSTEM_DARK.with(|d| d.set(dark));
                re_resolve();
            },
        );
        BUS.with(|b| *b.borrow_mut() = Some(conn));
    }
    re_resolve();
    INITIALIZED.with(|i| i.set(true));
}

fn read_portal_scheme(conn: &gio::DBusConnection) -> Option<u32> {
    let args = ("org.freedesktop.appearance", "color-scheme").to_variant();
    let reply_ty = glib::VariantTy::new("(v)").ok()?;
    let call = |method: &str| {
        conn.call_sync(
            Some("org.freedesktop.portal.Desktop"),
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.Settings",
            method,
            Some(&args),
            Some(reply_ty),
            gio::DBusCallFlags::NONE,
            1000,
            gio::Cancellable::NONE,
        )
    };
    match call("ReadOne") {
        Ok(reply) => reply.child_value(0).as_variant()?.get::<u32>(),
        Err(_) => call("Read")
            .ok()?
            .child_value(0)
            .as_variant()?
            .as_variant()?
            .get::<u32>(),
    }
}
