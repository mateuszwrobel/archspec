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
| `equivalent` | 1.0.2 | Apache-2.0 OR MIT | https://github.com/indexmap-rs/equivalent |
| `hashbrown` | 0.16.1 | MIT OR Apache-2.0 | https://github.com/rust-lang/hashbrown |
| `indexmap` | 2.13.0 | Apache-2.0 OR MIT | https://github.com/indexmap-rs/indexmap |
| `itoa` | 1.0.17 | MIT OR Apache-2.0 | https://github.com/dtolnay/itoa |
| `memchr` | 2.8.0 | Unlicense OR MIT | https://github.com/BurntSushi/memchr |
| `proc-macro2` | 1.0.106 | MIT OR Apache-2.0 | https://github.com/dtolnay/proc-macro2 |
| `quick-xml` | 0.42.0 | MIT | https://github.com/tafia/quick-xml |
| `quote` | 1.0.44 | MIT OR Apache-2.0 | https://github.com/dtolnay/quote |
| `regex` | 1.13.1 | MIT OR Apache-2.0 | https://github.com/rust-lang/regex |
| `regex-automata` | 0.4.18 | MIT OR Apache-2.0 | https://github.com/rust-lang/regex |
| `regex-syntax` | 0.8.11 | MIT OR Apache-2.0 | https://github.com/rust-lang/regex |
| `same-file` | 1.0.6 | Unlicense/MIT | https://github.com/BurntSushi/same-file |
| `serde` | 1.0.228 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| `serde_core` | 1.0.228 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| `serde_derive` | 1.0.228 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| `serde_json` | 1.0.149 | MIT OR Apache-2.0 | https://github.com/serde-rs/json |
| `serde_spanned` | 0.6.9 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| `streaming-iterator` | 0.1.9 | MIT OR Apache-2.0 | https://github.com/sfackler/streaming-iterator |
| `syn` | 2.0.115 | MIT OR Apache-2.0 | https://github.com/dtolnay/syn |
| `toml` | 0.8.23 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| `toml_datetime` | 0.6.11 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| `toml_edit` | 0.22.27 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| `toml_write` | 0.1.2 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| `tree-sitter` | 0.24.7 | MIT | https://github.com/tree-sitter/tree-sitter |
| `tree-sitter-c-sharp` | 0.23.1 | MIT | https://github.com/tree-sitter/tree-sitter-c-sharp |
| `tree-sitter-go` | 0.23.4 | MIT | https://github.com/tree-sitter/tree-sitter-go |
| `tree-sitter-language` | 0.1.8 | MIT | https://github.com/tree-sitter/tree-sitter |
| `unicode-ident` | 1.0.23 | (MIT OR Apache-2.0) AND Unicode-3.0 | https://github.com/dtolnay/unicode-ident |
| `walkdir` | 2.5.0 | Unlicense/MIT | https://github.com/BurntSushi/walkdir |
| `winapi-util` | 0.1.11 | Unlicense OR MIT | https://github.com/BurntSushi/winapi-util |
| `windows-link` | 0.2.1 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows-sys` | 0.61.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `winnow` | 0.7.14 | MIT | https://github.com/winnow-rs/winnow |
| `zmij` | 1.0.21 | MIT | https://github.com/dtolnay/zmij |

**35 crates.**
