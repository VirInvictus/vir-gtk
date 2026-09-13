# vir-gtk-capi

The C-ABI surface over [vir-gtk](https://github.com/VirInvictus/vir-gtk):
portal-driven dark/light theming for C GTK4 applications. Built to replace
Framework's manual `fw-theme.c` D-Bus port; see `vir-gtk.h` for the four
entry points (`theme_install`, `is_dark`, `on_dark_changed`,
`disconnect_dark_changed`).

## Build and install

```sh
cargo build -p vir-gtk-capi --release
install -Dm644 vir-gtk.h /usr/local/include/vir-gtk/vir-gtk.h
install -Dm644 vir-gtk.pc /usr/local/lib/pkgconfig/vir-gtk.pc
install -m755 ../target/release/libvir_gtk_capi.so /usr/local/lib/
```

(The staticlib, `libvir_gtk_capi.a`, lands in the same directory if a
static link is preferred.) The `Version:` in `vir-gtk.pc` moves with the
crate version; it is part of the release sync set.

## Consumers

Framework (GPL-3.0-or-later) is the target consumer and adopts at its 1.0.1.
Licensing: this surface is MIT and may be linked into GPL applications with
attribution.
