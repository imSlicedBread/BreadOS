# Dependency inventory

Exact resolved versions are recorded in [`Cargo.lock`](../Cargo.lock). No
third-party crate is linked into the desktop renderer or freestanding kernel.

| Package | Version | Use | License declared by package |
|---|---:|---|---|
| `uefi` | 0.40.0 | Loader firmware protocols, GOP, filesystem and boot-services exit | MIT OR Apache-2.0 |
| `uefi-raw` | 0.16.0 | UEFI ABI definitions, through `uefi` | MIT OR Apache-2.0 |
| `uefi-macros` | 0.19.0 | UEFI entry-point macro, through `uefi` | MIT OR Apache-2.0 |
| `bitflags` | 2.13.2 | UEFI flags, transitive | MIT OR Apache-2.0 |
| `cfg-if` | 1.0.5 | Conditional compilation, transitive | MIT OR Apache-2.0 |
| `log` | 0.4.34 | Loader dependency, transitive | MIT OR Apache-2.0 |
| `ptr_meta` | 0.3.2 | UEFI dependency, transitive | MIT |
| `ptr_meta_derive` | 0.3.2 | Derive support, transitive | MIT |
| `ucs2` | 0.3.3 | UEFI string support, transitive | MPL-2.0 |
| `bit_field` | 0.10.3 | UEFI dependency, transitive | Apache-2.0 OR MIT |
| `uguid` | 2.2.1 | UEFI GUID support, transitive | MIT OR Apache-2.0 |
| `proc-macro2` | 1.0.107 | Procedural macros, transitive | MIT OR Apache-2.0 |
| `quote` | 1.0.47 | Procedural macros, transitive | MIT OR Apache-2.0 |
| `syn` | 2.0.119, 3.0.6 | Procedural macros, transitive | MIT OR Apache-2.0 |
| `unicode-ident` | 1.0.26 | Procedural macros, transitive | (MIT OR Apache-2.0) AND Unicode-3.0 |

The Rust standard library is used by host-only `xtask` and test targets; neither
is shipped in the guest. The image builder uses Python 3 standard-library
modules. QEMU and OVMF are external development/test tools, not guest libraries.
This inventory is generated from the locked Cargo metadata; verify licenses and
include corresponding notices before distributing a binary image. BreadOS's
own distribution license has intentionally not been selected.
