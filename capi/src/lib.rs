// SPDX-License-Identifier: MIT
//! The C-ABI surface over `vir-gtk`: portal-driven dark/light theming for C
//! GTK4 applications (the replacement for Framework's manual
//! `fw-theme.c` D-Bus port, built from that file's call-site surface).
//!
//! The surface is deliberately tiny: install the theme (portal listener plus
//! a base stylesheet that re-splices on every flip), read the resolved
//! state, and subscribe to flips with a C callback. Call from the main
//! thread only, like the rest of GTK. The library never panics into the
//! host: no unwraps on the boundary, and the registration registry degrades
//! to no-ops on unknown ids.
//!
//! The header (`capi/vir-gtk.h`) and the pkg-config file (`capi/vir-gtk.pc`)
//! are handwritten and version-bumped with the crate; see `capi/README.md`
//! for the install recipe.
//!
//! Licensing note (the recorded decision of 2026-09-12): this surface is
//! MIT and may be linked into GPL applications (Framework, GPL-3.0-or-later)
//! with the attribution carried in the consumer's about/docs.

use gtk4::glib;
use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering};

use vir_gtk::portal;
use vir_gtk::theme::install_default;

/// The C callback shape for dark/light flips: `(dark, user_data)`, fired on
/// the main loop thread.
type DarkChangedFn = extern "C" fn(glib::ffi::gboolean, glib::ffi::gpointer);

thread_local! {
    /// Registration slots, main-thread only (the anchor is a GObject). The
    /// `Option` layering lets [`vir_gtk_disconnect_dark_changed`] take a
    /// slot out without reshuffling the vec while a dispatch is in flight.
    static SLOTS: RefCell<Vec<Option<Slot>>> = const { RefCell::new(Vec::new()) };
}

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

struct Slot {
    id: u32,
    /// The owner object the portal listener is registered against. Never
    /// read, but held deliberately: the portal's listener is weak against
    /// it, so dropping this field on disconnect is what retires the
    /// callback.
    #[allow(dead_code)]
    anchor: glib::Object,
    callback: DarkChangedFn,
    user_data: glib::ffi::gpointer,
    /// Already the nullable shape (`Option<unsafe extern "C" fn(gpointer)>`);
    /// C callers pass NULL for none.
    destroy: glib::ffi::GDestroyNotify,
}

/// Install the theme: start the portal listener, apply the current scheme,
/// and keep `vir-gtk`'s shared base stylesheet on the crate tier, re-spliced
/// on every dark/light flip. The one-call replacement for `fw_theme_install`.
/// Call once, after a display exists (from the application's startup);
/// later calls re-splice and are otherwise no-ops.
///
/// Side effect carried over from the Rust surface: the global
/// `gtk-application-prefer-dark-theme` key tracks the resolved state.
#[no_mangle]
pub extern "C" fn vir_gtk_theme_install(default_dark: glib::ffi::gboolean) {
    install_default(default_dark != glib::ffi::GFALSE);
}

/// The composed dark/light state (portal preference composed over the
/// application default). `TRUE` when dark. Safe from any thread.
#[no_mangle]
pub extern "C" fn vir_gtk_is_dark() -> glib::ffi::gboolean {
    portal::is_dark().into()
}

/// Register `callback` to fire with the resolved state on every dark/light
/// flip (`dark`, `user_data`), the direct replacement for connecting to
/// `fw-theme`'s `notify::dark`. `destroy`, when given, runs on
/// `user_data` exactly once at disconnection, and no callback fires after
/// [`vir_gtk_disconnect_dark_changed`] returns. Returns the registration id
/// to disconnect with. Call from the main thread.
#[no_mangle]
pub extern "C" fn vir_gtk_on_dark_changed(
    callback: DarkChangedFn,
    user_data: glib::ffi::gpointer,
    destroy: glib::ffi::GDestroyNotify,
) -> glib::ffi::guint {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let anchor = glib::Object::new::<glib::Object>();
    portal::connect_dark_changed(&anchor, move |dark| dispatch(id, dark));
    SLOTS.with(|slots| {
        slots.borrow_mut().push(Some(Slot {
            id,
            anchor,
            callback,
            user_data,
            destroy,
        }))
    });
    id
}

/// Remove the registration `id` (as returned by
/// [`vir_gtk_on_dark_changed`]); the destroy notify, if any, runs before the
/// call returns. Unknown or already-disconnected ids are no-ops.
#[no_mangle]
pub extern "C" fn vir_gtk_disconnect_dark_changed(id: glib::ffi::guint) {
    let mut taken: Option<Slot> = None;
    SLOTS.with(|slots| {
        let mut slots = slots.borrow_mut();
        let found = slots
            .iter()
            .position(|slot| slot.as_ref().is_some_and(|slot| slot.id == id));
        if let Some(pos) = found {
            taken = slots[pos].take();
        }
        slots.retain(|slot| slot.is_some());
    });
    if let Some(slot) = taken {
        if let Some(destroy) = slot.destroy {
            // SAFETY: GDestroyNotify is `unsafe extern "C" fn(gpointer)` by
            // GLib's contract; the caller handed us this function and this
            // `user_data` for exactly this one call, which is the only
            // place it is ever invoked.
            unsafe { destroy(slot.user_data) }
        }
    }
}

/// Fire the registration `id`'s callback. The callback and user data are
/// copied out of the borrow before the call, so a callback that disconnects
/// itself cannot re-enter the slot borrow.
fn dispatch(id: u32, dark: bool) {
    let payload = SLOTS.with(|slots| {
        slots
            .borrow()
            .iter()
            .flatten()
            .find(|slot| slot.id == id)
            .map(|slot| (slot.callback, slot.user_data))
    });
    if let Some((callback, user_data)) = payload {
        callback(glib::ffi::gboolean::from(dark), user_data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gtk4 as gtk;
    use std::cell::Cell;

    // Thread-local counters: the plain #[test] and the #[gtk::test] run on
    // different threads (and GTK callbacks here are synchronous direct
    // calls on the registering thread), so parallel suites stay isolated.
    thread_local! {
        static HITS: Cell<usize> = const { Cell::new(0) };
        static DESTROYED: Cell<usize> = const { Cell::new(0) };
    }

    extern "C" fn counting_callback(_dark: glib::ffi::gboolean, _data: glib::ffi::gpointer) {
        HITS.with(|hits| hits.set(hits.get() + 1));
    }

    extern "C" fn counting_destroy(_data: glib::ffi::gpointer) {
        DESTROYED.with(|destroyed| destroyed.set(destroyed.get() + 1));
    }

    #[test]
    fn registration_dispatches_and_disconnect_stops_it() {
        HITS.with(|hits| hits.set(0));
        DESTROYED.with(|destroyed| destroyed.set(0));
        let id = vir_gtk_on_dark_changed(
            counting_callback,
            std::ptr::null_mut(),
            Some(counting_destroy),
        );
        dispatch(id, true);
        assert_eq!(HITS.with(Cell::get), 1);
        assert_eq!(DESTROYED.with(Cell::get), 0);
        vir_gtk_disconnect_dark_changed(id);
        // The destroy notify ran exactly once, and no callback fires after
        // disconnect returns.
        assert_eq!(DESTROYED.with(Cell::get), 1);
        dispatch(id, false);
        assert_eq!(HITS.with(Cell::get), 1);
        // Disconnecting twice is a no-op (no second destroy call).
        vir_gtk_disconnect_dark_changed(id);
        assert_eq!(DESTROYED.with(Cell::get), 1);
        // Unknown ids never panic.
        vir_gtk_disconnect_dark_changed(999_999);
    }

    #[gtk::test]
    fn theme_install_splices_the_crate_tier_and_is_dark_reads_back() {
        gtk::init().unwrap();
        vir_gtk_theme_install(glib::ffi::GTRUE);
        let tier = vir_gtk::style::StyleManager::crate_tier();
        assert!(tier.is_installed());
        assert_eq!(
            vir_gtk_is_dark(),
            glib::ffi::gboolean::from(portal::is_dark())
        );
        // A second install must not panic; it re-splices.
        vir_gtk_theme_install(glib::ffi::GTRUE);
        assert!(tier.is_installed());
        tier.remove();
    }

    #[gtk::test]
    fn registration_receives_a_real_portal_broadcast_until_disconnected() {
        gtk::init().unwrap();
        HITS.with(|hits| hits.set(0));
        // Registering anchors a real portal listener. resolve_now() on a
        // not-yet-initialized portal seeds the state, which broadcasts to
        // every live listener; the registered callback must see exactly one
        // flip before disconnect, and none after.
        let id = vir_gtk_on_dark_changed(counting_callback, std::ptr::null_mut(), None);
        portal::resolve_now();
        assert_eq!(
            HITS.with(Cell::get),
            1,
            "broadcast must reach the C callback"
        );
        vir_gtk_disconnect_dark_changed(id);
        portal::resolve_now();
        assert_eq!(HITS.with(Cell::get), 1, "no callback after disconnect");
    }
}
