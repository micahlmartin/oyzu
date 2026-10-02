# Preserved dependency notices

These original notice files are preserved before integrating the ZIP decoder.
They are not a complete distribution notice bundle or a licensing approval.
No first-party Oyzu license is selected here.

| Package | Source | Declared license | Preserved files |
| --- | --- | --- | --- |
| zip 8.6.0 | [crates.io release](https://crates.io/crates/zip/8.6.0) | MIT | `zip-8.6.0/LICENSE` |
| typed-path 0.12.3 | [crates.io release](https://crates.io/crates/typed-path/0.12.3) | MIT OR Apache-2.0 | `typed-path-0.12.3/LICENSE-MIT`, `typed-path-0.12.3/LICENSE-APACHE` |

Files were copied without editing from the corresponding Cargo registry source
packages. Package checksums are recorded in Cargo.lock. Both alternative
typed-path license texts are retained without changing its license expression.
ZIP uses only stored/DEFLATE decoding; default features are disabled and flate2
uses Oyzu's existing Rust backend. Original dependency authorship is unchanged.
The complete resolved shipping graph, notices and distribution obligations still
require the OEP-0003 review owned by @micahlmartin.
