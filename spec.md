# vir-gtk Specification

**Domain**: GUI framework / System integration  
**Language**: Rust (edition 2021)  
**Frameworks**: GTK4 (`gtk4`), `gio`, `glib`  

## 1. Architecture

`vir-gtk` is a stateless dependency crate that manages GTK4 visual rendering and D-Bus signaling for the VirInvictus Rust apps. It exists to guarantee cross-app visual parity after the removal of `libadwaita` (Phase 10/20 cuts).

### 1.1 Portal Resolution (`vir_gtk::portal`)

The `portal` module handles `org.freedesktop.portal.Settings` directly via `gio::bus_get_sync`.
- **System Parity**: `color-scheme` (0 = no preference, 1 = prefer dark, 2 = prefer light).
- **Fallback**: The portal safely degrades to a user-defined default if the D-Bus connection fails (e.g., headless environments).
- **Composition**: Applications pass a `gio::Settings` handle and key (e.g., `"theme"`). The module composes this against the system preference (e.g., `force-dark` overrides a light system).

### 1.2 Themes (`vir_gtk::theme`)

Contains the definitive **Kanagawa Dragon** and **Kanagawa Lotus** hex values for the suite.
- Generates generic custom properties (`to_css_custom_properties`) or executes string token replacement (`replace_tokens`) to inject colors into raw CSS strings.
- Injects the resulting CSS at `STYLE_PROVIDER_PRIORITY_USER + 1` to reliably override the system `~/.config/gtk-4.0/gtk.css`.
