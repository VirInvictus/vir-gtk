# vir-gtk

**Stack:** Rust, GTK4 (`gtk4` crate), `gio`.
**Status:** Maintained. Standalone library.

## What is this?
A shared styling library for the VirInvictus desktop suite. Extracted from Atrium, Conservatory, Viaduct, and Colophon to eliminate duplication of the `org.freedesktop.portal.Settings` dark-mode logic and the Kanagawa Dragon/Lotus palette generation code.

## Key Rules
- **No libadwaita**: This crate exists precisely to provide the features of `adw::StyleManager` and standard Adwaita CSS variables without the `libadwaita` dependency. We do not want libadwaita in the dependency tree.
- **Flat Design Identity**: The custom properties and replacement patterns are meant to style raw GTK4 widgets (`window`, `headerbar`, `button`, `row`) into the VirInvictus design idiom. This means flat shapes, calm colors, and Kanagawa themes.
- **Test the Theme Output**: Tests should always verify that palettes are successfully spliced into string templates without leaving `%TOKEN%` placeholders behind. Test coverage must encompass the CSS generation logic.
- **Local State Management**: The `portal` module splits its state by thread-safety need. The boolean state (`INITIALIZED`, `SYSTEM_DARK`, `RESOLVED_DARK`, `DEFAULT_DARK`) lives in global atomics so background threads can query `is_dark()`/`system_is_dark()` without the main thread. The GTK-bound pieces (the optional `gio::Settings`, the D-Bus connection that keeps the signal subscription alive, the listener registry) stay in thread-locals: they are `!Send` and only touched on the thread that called `init()`. There are no locks around the GTK main loop.

## Development Commands
```bash
cargo check
cargo test
cargo fmt
```
