# vir-gtk

A standalone Rust library that provides the shared GTK4 styling and D-Bus portal interaction layer for the VirInvictus desktop suite.

`vir-gtk` exists to replace `libadwaita`. It provides the foundational visual identity for `Atrium`, `Conservatory`, and `Viaduct`, injecting a bespoke Kanagawa-themed framework directly into standard GTK4 widgets. (Colophon and Framework still style themselves; the roadmap's Phase 2 tracks the palette/registry migration and the C-ABI for Framework.) By centralizing the theme engine and D-Bus color-scheme portal listener, the applications using it maintain pixel-perfect consistency and respond instantly to system-wide dark/light mode toggles without duplicating boilerplate.

## Architecture and Capabilities

`vir-gtk` is divided into three primary modules:

### The Portal Module (`vir_gtk::portal`)

The portal module is responsible for reading and monitoring the system's preferred color scheme via the `org.freedesktop.portal.Settings` D-Bus interface.

It handles the complexity of composing the desktop's system color scheme against an application's internal preferences (for example, if a user sets the app to `force-dark` while the system is light). It exposes an `is_dark()` accessor and a change-listener registry that drops dead weak references safely, ensuring no memory leaks occur across the application lifecycle.

### The Theme Module (`vir_gtk::theme`)

The theme module provides the definitive Kanagawa Dragon (dark) and Kanagawa Lotus (light) hex palettes used across the suite.

It exposes methods to inject these palettes into GTK4 applications. It supports generating standard GTK 4.16+ custom property blocks (e.g., `--c-bg-window`) or performing direct string token replacement (e.g., swapping `%BG_WINDOW%` for the hex code) on legacy CSS stylesheets.

It also ships `base_css()`, the shared flat, square base widget sheet (window chrome, headerbar, lists, buttons, entries, popovers, the Adwaita utility classes, and a scoped focus ring) spliced with a palette, plus a two-tier install ladder: `install_stylesheet` carries the crate's sheets at `USER + 1` and `install_app_stylesheet` carries the application's own sheet at `USER + 2`, so app-specific overrides win by priority rather than by install order.

### The Color Module (`vir_gtk::color`)

For widgets that draw themselves (charts, waveforms, spectrums): `to_gdk_rgba` and `to_cairo_rgba` turn the palette's hex strings into GDK and cairo values (strict CSS hex parsing; malformed input is `None`, not a panic), and `redraw_on_theme_change` re-queues a widget's draw when the portal flips dark/light. Cairo needs no dependency here: the cairo values are plain floats you pass to your own cairo context.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
vir-gtk = { git = "https://github.com/VirInvictus/vir-gtk.git", branch = "main" }
```

## Usage

A standard initialization block in a VirInvictus application sets up the portal listener and applies the stylesheet based on the initial system state.

```rust
use vir_gtk::portal;
use vir_gtk::theme::{Palette, install_stylesheet};

fn main() {
    // Initialize the portal listener to sync with system dark/light mode.
    // The arguments allow you to bind the listener to an app's gio::Settings.
    portal::init(None, None, true);

    // Fetch the correct palette based on the portal's resolved state.
    let palette = if portal::is_dark() {
        Palette::dragon()
    } else {
        Palette::lotus()
    };

    // Generate the CSS custom properties block and inject it.
    let css = format!("{} window {{ background: var(--c-bg-window); }}", palette.to_css_custom_properties());
    install_stylesheet(&css);
}
```

## Design Philosophy

The VirInvictus suite uses flat, calm interfaces built on the Kanagawa color palette. `vir-gtk` strips away the rounded corners, gradients, and heavy shadows of Adwaita in favor of sharp, distinct boundaries and muted contrast. The library assumes that the application will style standard `gtk::Box`, `gtk::HeaderBar`, and `gtk::Button` widgets manually using the injected custom properties.

## Support

If vir-gtk's useful to you and you'd like to chip in:

- liberapay · [liberapay.com/bdkl](https://liberapay.com/bdkl/)
- bitcoin
  ```
  bc1qkge6zr45tzqfwfmvma2ylumt6mg7wlwmhr05yv
  ```
