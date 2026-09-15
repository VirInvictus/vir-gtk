# vir-gtk

**Stack:** Rust (edition 2021), GTK4 (`gtk4` crate), `gio`.
**Status:** Maintained. Standalone library.
**Versioning deviation:** there is no `VERSION` file; `Cargo.toml` is the single version source (a `VERSION` file would be a second carrier Cargo cannot consume). Every bump still gets a patchnotes entry and an annotated tag at the release commit. The workspace member `capi/` inherits the version, and `capi/vir-gtk.pc`'s `Version:` line moves with it (the pc file is the one deliberate second carrier; pkg-config cannot read Cargo.toml).

## What is this?
A shared styling library for the VirInvictus desktop suite. Extracted from Atrium, Conservatory, Viaduct, and Colophon to eliminate duplication of the `org.freedesktop.portal.Settings` dark-mode logic and the Kanagawa Dragon/Lotus palette generation code. Since 1.1.0 it also carries the shared base widget stylesheet (`theme::base_css`) and color conversion helpers (`color`); since 1.4.0, the plain-GTK widget kit (`widgets`: rows, `Group`, `Alert`, `close_on_escape`) and the C-ABI member (`capi/`, the `vir-gtk-capi` cdylib built from Framework's `fw-theme.c` call surface; Framework's 1.0.1 shipped without adopting it, so that adoption stays pending on Framework's side).

## Key Rules
- **No libadwaita**: This crate exists precisely to provide the features of `adw::StyleManager` and standard Adwaita CSS variables without the `libadwaita` dependency. We do not want libadwaita in the dependency tree.
- **Flat Design Identity**: The custom properties and replacement patterns are meant to style raw GTK4 widgets (`window`, `headerbar`, `button`, `row`) into the VirInvictus design idiom. This means flat shapes, calm colors, and Kanagawa themes.
- **The install ladder is the override mechanism, and only a tie-breaker**: crate sheets install at `USER + 1` (`install_stylesheet`), application sheets at `USER + 2` (`install_app_stylesheet`), each tier tracked and replaced per display. GTK CSS decides by specificity first; provider priority breaks only equal-specificity ties, so an app override must match the base rule's selector shape (a bare app `window {}` loses to the base's `window.csd`). `base_css` carries the unanimous flat/square core: radius divergences (non-default rounding; the base still squares every corner it pins), `@define-color` blocks, toasts, and OSD surfaces are deliberate per-app divergences and stay in the app sheet, along with row-selection styling (the `row.activatable` states differ in all three apps) and any `font-family` rules. By the recorded 1.1.0 majority decisions the base DOES carry the button `:active`/`:checked` paint, the text `selection` tint, and the typography utilities (`.title-*` and friends, weights/sizes only); do not "fix" those back out, they are the byte-stable contract. Never move an app divergence into the base sheet.
- **Managed lifecycle; handles are keys**: provider installs go through `style::StyleManager` rungs (crate `USER + 1`, app `USER + 2`, explicit priorities above; `install` replaces, `remove` tears down) or `style::StyleScope` (a class-scoped forced palette at `USER + 4`; the scope wires its target's destroy to tear the provider down). Dropping a handle never uninstalls; teardown is explicit (`remove`, `set_choice(System)`) or structural (the scope's destroy hook). A raw `style_context_add_provider_for_display` call is unmanaged state; prefer a rung.
- **A library never panics into consumers**: zero `unwrap`/`expect`/indexing panics in the shipped code paths (tests are the exception). The portal's signal handler uses `try_child_value` so a malformed D-Bus body degrades instead of unwinding into a host app's main loop.
- **Test the Theme Output**: Tests should always verify that palettes are successfully spliced into string templates without leaving `%TOKEN%` placeholders behind (token-shaped `%UPPERCASE%` spans; literal `%` in `font-size: 82%` is legitimate). Test coverage must encompass the CSS generation logic.
- **Local State Management**: The `portal` module splits its state by thread-safety need. The boolean state (`INITIALIZED`, `SYSTEM_DARK`, `RESOLVED_DARK`, `DEFAULT_DARK`, plus the read-generation counter) lives in global atomics so background threads can query `is_dark()`/`system_is_dark()` without the main thread. The GTK-bound pieces (the optional `gio::Settings`, the D-Bus connection that keeps the signal subscriptions alive, the `SignalSubscription` handles, the listener registry) stay in thread-locals: they are `!Send` and only touched on the thread that called `init()`. There are no locks around the GTK main loop.

## Development Commands
```bash
cargo check
cargo test
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
```
CI runs fmt, clippy, and the test suite (clippy is a gate; `cargo check` stays a local habit, and the `#[gtk::test]` widget/scope tests GTK-init against the live session locally and under `xvfb-run` in CI).
