# Credits

The `archspec` tool (crate `rust-arch-test-kit`) is licensed under the
Apache License 2.0 — the full text lives in [LICENSE](LICENSE).

This listing credits every third-party crate whose code is linked into the
`archspec` binary, as resolved from the committed lockfile. It is
**generated** — do not edit it by hand. Regenerate it from a checkout of
this crate with:

    ./scripts/gen-credits.sh

The listing covers the normal (runtime) dependency closure of the root
package: crates used only for development, or only on the build host as
build-script tooling, are not part of the distributed binary and are not
listed here. Grammar crates embed their upstream grammar sources from the
linked repository; the license stated in a grammar crate's row is also the
license of the grammar it vendors.

## Dependencies

| Crate | Version | License | Repository |
|---|---|---|---|
| `aho-corasick` | 1.1.5 | Unlicense OR MIT | https://github.com/BurntSushi/aho-corasick |
| `allocator-api2` | 0.2.21 | MIT OR Apache-2.0 | https://github.com/zakarumych/allocator-api2 |
| `arrayvec` | 0.7.8 | MIT OR Apache-2.0 | https://github.com/bluss/arrayvec |
| `bitflags` | 2.11.0 | MIT OR Apache-2.0 | https://github.com/bitflags/bitflags |
| `brokk-tree-sitter-kotlin` | 0.4.6 | MIT | https://github.com/BrokkAi/tree-sitter-kotlin |
| `castaway` | 0.2.4 | MIT | https://github.com/sagebind/castaway |
| `cfg-if` | 1.0.4 | MIT OR Apache-2.0 | https://github.com/rust-lang/cfg-if |
| `compact_str` | 0.10.0 | MIT | https://github.com/ParkMyCar/compact_str |
| `crossbeam-utils` | 0.8.23 | MIT OR Apache-2.0 | https://github.com/crossbeam-rs/crossbeam |
| `dashmap` | 6.2.1 | MIT | https://github.com/xacrimon/dashmap |
| `equivalent` | 1.0.2 | Apache-2.0 OR MIT | https://github.com/indexmap-rs/equivalent |
| `errno` | 0.3.14 | MIT OR Apache-2.0 | https://github.com/lambda-fairy/rust-errno |
| `fast-glob` | 1.1.2 | MIT | https://github.com/oxc-project/fast-glob.git |
| `float-cmp` | 0.10.0 | MIT | https://github.com/mikedilger/float-cmp |
| `foldhash` | 0.2.0 | Zlib | https://github.com/orlp/foldhash |
| `halfbrown` | 0.4.0 | Apache-2.0/MIT | https://github.com/Licenser/halfbrown |
| `hashbrown` | 0.14.5 | MIT OR Apache-2.0 | https://github.com/rust-lang/hashbrown |
| `hashbrown` | 0.16.1 | MIT OR Apache-2.0 | https://github.com/rust-lang/hashbrown |
| `indexmap` | 2.13.0 | Apache-2.0 OR MIT | https://github.com/indexmap-rs/indexmap |
| `itoa` | 1.0.17 | MIT OR Apache-2.0 | https://github.com/dtolnay/itoa |
| `json-strip-comments` | 3.1.2 | Apache-2.0 | https://github.com/oxc-project/json-strip-comments |
| `libc` | 0.2.182 | MIT OR Apache-2.0 | https://github.com/rust-lang/libc |
| `linux-raw-sys` | 0.12.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/sunfishcode/linux-raw-sys |
| `lock_api` | 0.4.14 | MIT OR Apache-2.0 | https://github.com/Amanieu/parking_lot |
| `memchr` | 2.8.0 | Unlicense OR MIT | https://github.com/BurntSushi/memchr |
| `nodejs-built-in-modules` | 1.0.0 | MIT | https://github.com/oxc-project/nodejs-built-in-modules |
| `num-traits` | 0.2.19 | MIT OR Apache-2.0 | https://github.com/rust-num/num-traits |
| `once_cell` | 1.21.4 | MIT OR Apache-2.0 | https://github.com/matklad/once_cell |
| `oxc_resolver` | 11.24.3 | MIT | https://github.com/oxc-project/oxc-resolver |
| `parking_lot_core` | 0.9.12 | MIT OR Apache-2.0 | https://github.com/Amanieu/parking_lot |
| `percent-encoding` | 2.3.2 | MIT OR Apache-2.0 | https://github.com/servo/rust-url/ |
| `pin-project-lite` | 0.2.17 | Apache-2.0 OR MIT | https://github.com/taiki-e/pin-project-lite |
| `proc-macro2` | 1.0.106 | MIT OR Apache-2.0 | https://github.com/dtolnay/proc-macro2 |
| `quick-xml` | 0.42.0 | MIT | https://github.com/tafia/quick-xml |
| `quote` | 1.0.44 | MIT OR Apache-2.0 | https://github.com/dtolnay/quote |
| `redox_syscall` | 0.5.18 | MIT | https://gitlab.redox-os.org/redox-os/syscall |
| `ref-cast` | 1.0.27 | MIT OR Apache-2.0 | https://github.com/dtolnay/ref-cast |
| `ref-cast-impl` | 1.0.27 | MIT OR Apache-2.0 | https://github.com/dtolnay/ref-cast |
| `regex` | 1.13.1 | MIT OR Apache-2.0 | https://github.com/rust-lang/regex |
| `regex-automata` | 0.4.18 | MIT OR Apache-2.0 | https://github.com/rust-lang/regex |
| `regex-syntax` | 0.8.11 | MIT OR Apache-2.0 | https://github.com/rust-lang/regex |
| `rustc-hash` | 2.1.3 | Apache-2.0 OR MIT | https://github.com/rust-lang/rustc-hash |
| `rustix` | 1.1.5 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/bytecodealliance/rustix |
| `rustversion` | 1.0.23 | MIT OR Apache-2.0 | https://github.com/dtolnay/rustversion |
| `ryu` | 1.0.23 | Apache-2.0 OR BSL-1.0 | https://github.com/dtolnay/ryu |
| `same-file` | 1.0.6 | Unlicense/MIT | https://github.com/BurntSushi/same-file |
| `scopeguard` | 1.2.0 | MIT OR Apache-2.0 | https://github.com/bluss/scopeguard |
| `self_cell` | 1.3.0 | Apache-2.0 OR GPL-2.0-only | https://github.com/Voultapher/self_cell |
| `serde` | 1.0.228 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| `serde_core` | 1.0.228 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| `serde_derive` | 1.0.228 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| `serde_json` | 1.0.149 | MIT OR Apache-2.0 | https://github.com/serde-rs/json |
| `serde_spanned` | 0.6.9 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| `simd-json` | 0.18.1 | Apache-2.0 OR MIT | https://github.com/simd-lite/simd-json |
| `simdutf8` | 0.1.5 | MIT OR Apache-2.0 | https://github.com/rusticstuff/simdutf8 |
| `smallvec` | 1.16.3 | MIT OR Apache-2.0 | https://github.com/servo/rust-smallvec |
| `static_assertions` | 1.1.0 | MIT OR Apache-2.0 | https://github.com/nvzqz/static-assertions-rs |
| `streaming-iterator` | 0.1.9 | MIT OR Apache-2.0 | https://github.com/sfackler/streaming-iterator |
| `syn` | 2.0.115 | MIT OR Apache-2.0 | https://github.com/dtolnay/syn |
| `syn` | 3.0.9 | MIT OR Apache-2.0 | https://github.com/dtolnay/syn |
| `thiserror` | 2.0.21 | MIT OR Apache-2.0 | https://github.com/dtolnay/thiserror |
| `thiserror-impl` | 2.0.21 | MIT OR Apache-2.0 | https://github.com/dtolnay/thiserror |
| `toml` | 0.8.23 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| `toml_datetime` | 0.6.11 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| `toml_edit` | 0.22.27 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| `toml_write` | 0.1.2 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| `tracing` | 0.1.44 | MIT | https://github.com/tokio-rs/tracing |
| `tracing-attributes` | 0.1.31 | MIT | https://github.com/tokio-rs/tracing |
| `tracing-core` | 0.1.36 | MIT | https://github.com/tokio-rs/tracing |
| `tree-sitter` | 0.24.7 | MIT | https://github.com/tree-sitter/tree-sitter |
| `tree-sitter-c-sharp` | 0.23.1 | MIT | https://github.com/tree-sitter/tree-sitter-c-sharp |
| `tree-sitter-go` | 0.23.4 | MIT | https://github.com/tree-sitter/tree-sitter-go |
| `tree-sitter-javascript` | 0.23.1 | MIT | https://github.com/tree-sitter/tree-sitter-javascript |
| `tree-sitter-language` | 0.1.7 | MIT | https://github.com/tree-sitter/tree-sitter |
| `tree-sitter-python` | 0.23.6 | MIT | https://github.com/tree-sitter/tree-sitter-python |
| `tree-sitter-typescript` | 0.23.2 | MIT | https://github.com/tree-sitter/tree-sitter-typescript |
| `unicode-ident` | 1.0.23 | (MIT OR Apache-2.0) AND Unicode-3.0 | https://github.com/dtolnay/unicode-ident |
| `value-trait` | 0.12.2 | Apache-2.0/MIT | https://github.com/simd-lite/value-trait |
| `walkdir` | 2.5.0 | Unlicense/MIT | https://github.com/BurntSushi/walkdir |
| `winapi-util` | 0.1.11 | Unlicense OR MIT | https://github.com/BurntSushi/winapi-util |
| `windows` | 0.62.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows-collections` | 0.3.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows-core` | 0.62.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows-future` | 0.3.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows-implement` | 0.60.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows-interface` | 0.59.3 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows-link` | 0.2.1 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows-numerics` | 0.3.1 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows-result` | 0.4.1 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows-strings` | 0.5.1 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows-sys` | 0.61.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows-threading` | 0.2.1 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `winnow` | 0.7.14 | MIT | https://github.com/winnow-rs/winnow |
| `zmij` | 1.0.21 | MIT | https://github.com/dtolnay/zmij |

**94 crates.**
