# vir-gtk

[![CI](https://github.com/VirInvictus/vir-gtk/actions/workflows/ci.yml/badge.svg)](https://github.com/VirInvictus/vir-gtk/actions/workflows/ci.yml)

A standalone Rust library extracting the shared GTK4 styling and D-Bus portal interaction layer for the VirInvictus desktop suite.

`vir-gtk` provides the foundational visual identity for `Atrium`, `Conservatory`, `Viaduct`, and `Colophon`, replacing `libadwaita` with a bespoke Kanagawa-themed framework. 

## Capabilities

- **`vir_gtk::portal`**: Handles `org.freedesktop.portal.Settings` DBus resolution. Supports composing the desktop's system color scheme against application-specific forced preferences (e.g., `force-dark`), and provides an `is_dark()` accessor and change-listener registry that drops dead weak refs safely.
- **`vir_gtk::theme`**: Provides baked `DRAGON` and `LOTUS` hex palettes, token-replacement CSS injectors, and GTK 4.16+ custom property block generators (`--c-*`).

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
vir-gtk = { git = "https://github.com/VirInvictus/vir-gtk.git", branch = "main" }
```

## Usage

```rust
use vir_gtk::portal;
use vir_gtk::theme::{Palette, install_stylesheet};

// Listen to desktop color changes
portal::init(None, None, true);

// Fetch a predefined palette and inject CSS
let palette = Palette::dragon();
let css = format!("{} window {{ background: var(--c-bg-window); }}", palette.to_css_custom_properties());
install_stylesheet(&css);
```
