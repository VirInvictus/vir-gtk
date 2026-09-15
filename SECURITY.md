# Security policy

vir-gtk is a styling and D-Bus portal library embedded in the VirInvictus
desktop suite: the Rust consumers (Atrium, Conservatory, Viaduct) and any
C consumer of the `capi/` surface. A defect here is a defect in every
host application's main loop, so reports are welcome and taken seriously.

## Reporting a vulnerability

Do not open a public issue for a security report. Use GitHub's private
vulnerability reporting for this repository
(<https://github.com/VirInvictus/vir-gtk/security/advisories/new>), or
write to <larocque.brandon@gmail.com> with "vir-gtk security" in the
subject. You will get a response within a few days.

## Scope

The interesting surfaces, in rough order:

- **The portal module's parse paths.** Signal bodies and read replies come
  off the bus from whatever the sender put there; a malformed body must
  degrade (with a warning on the `vir-gtk` log domain), never panic into
  or corrupt a host application.
- **The C-ABI surface (`capi/`).** Anything a C caller can trip into
  undefined behavior from the documented call patterns: NULL handling,
  destroy-notify re-entrancy, use-after-disconnect.
- **Provider lifecycle.** Leaked or orphaned `GtkCssProvider`s are a
  correctness bug class here (display-global state), not just hygiene.

## Supported versions

Consumers track `main` by lock revision and take updates through explicit
`cargo update` events, so fixes land on `main` and ride the next consumer
adoption. Only the newest tagged release is supported for the C surface.
