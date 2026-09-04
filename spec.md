# vir-gtk Specification

**Domain**: GUI framework and system integration.
**Language**: Rust (edition 2021).
**Frameworks**: GTK4 (`gtk4`), `gio`, `glib`.

## 1. Architecture and Scope

`vir-gtk` is a small dependency crate managing GTK4 visual rendering and D-Bus signaling for the VirInvictus Rust applications. It exists to guarantee cross-app visual parity after the intentional removal of `libadwaita` from the ecosystem. It provides no widgets itself. Instead, it provides the backend engines required to style standard GTK widgets properly.

### 1.1 Portal Resolution (`vir_gtk::portal`)

The `portal` module handles `org.freedesktop.portal.Settings` over an asynchronous `gio::bus_get`: `init()` never blocks the calling (main) thread, and the portal's answer arrives through the main loop.
- **System Parity**: It listens for the `color-scheme` key (0 = no preference, 1 = prefer dark, 2 = prefer light) broadcast by the desktop environment. A real preference (1 or 2) overrides the application default; "no preference" and unknown values do not — the caller's `default_dark` stands.
- **Fallback Constraints**: The portal degrades safely to a user-defined default if the D-Bus connection fails. This ensures applications do not crash in headless environments or non-standard Wayland sessions.
- **Composition Engine**: Applications can pass a `gio::Settings` handle and a specific key (e.g., `"theme"`). The module composes this against the system preference so that explicit `force-dark` or `force-light` overrides take precedence over the system broadcast.
- **Threading**: The boolean state is global atomics, readable from any thread; the GTK-bound state (settings, connection, listeners) is main-thread thread-locals. Listener callbacks fire on the thread that runs the main loop, iterated by index so re-entrant broadcasts and mid-broadcast registrations are safe.

### 1.2 Themes (`vir_gtk::theme`)

Contains the definitive **Kanagawa Dragon** (dark) and **Kanagawa Lotus** (light) hex values.
- **CSS Variable Generation**: Generates generic custom properties (`to_css_custom_properties`) conforming to standard CSS variable notation (`--c-*`) to be parsed natively by GTK 4.16+.
- **Token Replacement**: Provides `replace_tokens` to inject colors into raw CSS strings for legacy applications or highly specific override blocks that do not use CSS variables. Every palette slot has a token, including `%BG%` and `%HEADING%`.
- **Priority Injection**: Injects the resulting CSS at `STYLE_PROVIDER_PRIORITY_USER + 1` via `install_stylesheet`, which replaces the provider this crate previously installed on that display rather than accumulating providers across theme switches. This guarantees the injected theme reliably overrides the system's `~/.config/gtk-4.0/gtk.css` without breaking application-specific overrides.

## 2. API Contract

- `portal::init(settings: Option<gio::Settings>, key: Option<&str>, default_dark: bool)` must be called once during application startup. It returns without blocking; the portal's current scheme arrives asynchronously and re-resolves state when it lands.
- `portal::is_dark() -> bool` provides synchronous access to the currently resolved color state, safe from any thread.
- `portal::connect_dark_changed(owner: &impl IsA<glib::Object>, f: F)` registers a callback that fires whenever the composed state changes; the owner is held as a weak reference, so callbacks bound to UI elements drop out when the element dies.
