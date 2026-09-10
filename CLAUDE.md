# vir-gtk

**Stack:** Rust (edition 2021), GTK4 (`gtk4` crate), `gio`.
**Status:** Maintained. Standalone library.
**Versioning deviation:** there is no `VERSION` file; `Cargo.toml` is the single version source (a `VERSION` file would be a second carrier Cargo cannot consume). Every bump still gets a patchnotes entry and an annotated tag at the release commit.

## What is this?
A shared styling library for the VirInvictus desktop suite. Extracted from Atrium, Conservatory, Viaduct, and Colophon to eliminate duplication of the `org.freedesktop.portal.Settings` dark-mode logic and the Kanagawa Dragon/Lotus palette generation code. Since 1.1.0 it also carries the shared base widget stylesheet (`theme::base_css`) and color conversion helpers (`color`) for custom-drawn widgets.

## Key Rules
- **No libadwaita**: This crate exists precisely to provide the features of `adw::StyleManager` and standard Adwaita CSS variables without the `libadwaita` dependency. We do not want libadwaita in the dependency tree.
- **Flat Design Identity**: The custom properties and replacement patterns are meant to style raw GTK4 widgets (`window`, `headerbar`, `button`, `row`) into the VirInvictus design idiom. This means flat shapes, calm colors, and Kanagawa themes.
- **The install ladder is the override mechanism**: crate sheets install at `USER + 1` (`install_stylesheet`), application sheets at `USER + 2` (`install_app_stylesheet`), each tier tracked and replaced per display. `base_css` carries only the unanimous flat/square core: radius, selection/checked semantics, `@define-color` blocks, typography, toasts, and OSD surfaces are deliberate per-app divergences and stay in the app sheet. Never move an app divergence into the base sheet.
- **Managed lifecycle; handles are keys**: provider installs go through `style::StyleManager` rungs (crate `USER + 1`, app `USER + 2`, explicit priorities above; `install` replaces, `remove` tears down) or `style::StyleScope` (a class-scoped forced palette at `USER + 4`). Dropping a handle never uninstalls; teardown is explicit (`remove`, `set_choice(System)`) or structural (a scope's class dies with its widget). A raw `style_context_add_provider_for_display` call is unmanaged state; prefer a rung.
- **Test the Theme Output**: Tests should always verify that palettes are successfully spliced into string templates without leaving `%TOKEN%` placeholders behind (token-shaped `%UPPERCASE%` spans; literal `%` in `font-size: 82%` is legitimate). Test coverage must encompass the CSS generation logic.
- **Local State Management**: The `portal` module splits its state by thread-safety need. The boolean state (`INITIALIZED`, `SYSTEM_DARK`, `RESOLVED_DARK`, `DEFAULT_DARK`) lives in global atomics so background threads can query `is_dark()`/`system_is_dark()` without the main thread. The GTK-bound pieces (the optional `gio::Settings`, the D-Bus connection that keeps the signal subscription alive, the listener registry) stay in thread-locals: they are `!Send` and only touched on the thread that called `init()`. There are no locks around the GTK main loop.

## Development Commands
```bash
cargo check
cargo test
cargo fmt
```
