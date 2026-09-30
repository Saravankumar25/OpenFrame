# Third-Party Licenses — OpenFrame Studio

<!-- GENERATED FILE. Do not edit by hand. Regenerate with `npm run licenses` (scripts/license-inventory.mjs). -->

This is an inventory of the third-party packages that are compiled into, or shipped with, the OpenFrame Studio desktop application (Rust target `x86_64-pc-windows-msvc`, normal dependencies only; npm production dependencies only). Build tools, test tooling and dev-only packages are excluded. This inventory is not a substitute for the full license texts, which the release pipeline must bundle with the installer.

OpenFrame Studio's own license has **not been selected yet** — see `LICENSE-PENDING.md`.

## Summary

- Rust crates: **393**
- npm packages: **72**
- Copyleft or unknown (blocking until resolved): **0**
- Weak copyleft / needs review: **5**

## Flagged: copyleft or unknown license

None.

## Needs review: weak copyleft or unusual terms

| Package | Version | License | Category | Source |
|---|---|---|---|---|
| cssparser | 0.37.0 | MPL-2.0 | weak copyleft / review | https://github.com/servo/rust-cssparser |
| cssparser-macros | 0.7.1 | MPL-2.0 | weak copyleft / review | https://github.com/servo/rust-cssparser |
| dtoa-short | 0.3.5 | MPL-2.0 | weak copyleft / review | https://github.com/upsuper/dtoa-short |
| option-ext | 0.2.0 | MPL-2.0 | weak copyleft / review | https://github.com/soc/option-ext |
| selectors | 0.38.0 | MPL-2.0 | weak copyleft / review | https://github.com/servo/stylo |

## License expressions in use

| License expression | Packages |
|---|---:|
| MIT OR Apache-2.0 | 193 |
| MIT | 139 |
| Apache-2.0 OR MIT | 38 |
| Unicode-3.0 | 18 |
| MIT/Apache-2.0 | 15 |
| Unlicense OR MIT | 10 |
| BSD-3-Clause | 7 |
| MPL-2.0 | 5 |
| Apache-2.0 | 4 |
| MIT OR Apache-2.0 OR Zlib | 4 |
| Unlicense/MIT | 4 |
| ISC | 3 |
| Zlib | 3 |
| Apache-2.0 OR ISC OR MIT | 2 |
| BSD-3-Clause OR Apache-2.0 | 2 |
| BSD-3-Clause/MIT | 2 |
| MIT OR Zlib OR Apache-2.0 | 2 |
| Zlib OR Apache-2.0 OR MIT | 2 |
| (Apache-2.0 OR MIT) AND BSD-3-Clause | 1 |
| (MIT OR Apache-2.0) AND Unicode-3.0 | 1 |
| 0BSD | 1 |
| 0BSD OR MIT OR Apache-2.0 | 1 |
| Apache-2.0 / MIT | 1 |
| Apache-2.0 AND ISC | 1 |
| Apache-2.0 AND MIT | 1 |
| Apache-2.0 OR BSL-1.0 | 1 |
| BSD-2-Clause OR Apache-2.0 OR MIT | 1 |
| BSD-3-Clause AND MIT | 1 |
| CC0-1.0 OR MIT-0 OR Apache-2.0 | 1 |
| CDLA-Permissive-2.0 | 1 |

## Rust crates

| Package | Version | License | Category | Source |
|---|---|---|---|---|
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 | permissive | https://github.com/oyvindln/adler2 |
| aes | 0.9.3 | MIT OR Apache-2.0 | permissive | https://github.com/RustCrypto/block-ciphers |
| aho-corasick | 1.1.5 | Unlicense OR MIT | permissive | https://github.com/BurntSushi/aho-corasick |
| alloc-no-stdlib | 2.0.4 | BSD-3-Clause | permissive | https://github.com/dropbox/rust-alloc-no-stdlib |
| alloc-no-stdlib | 3.0.0 | BSD-3-Clause | permissive | https://github.com/dropbox/rust-alloc-no-stdlib |
| alloc-stdlib | 0.2.4 | BSD-3-Clause | permissive | https://github.com/dropbox/rust-alloc-no-stdlib |
| alloc-stdlib | 0.3.0 | BSD-3-Clause | permissive | https://github.com/dropbox/rust-alloc-no-stdlib |
| anyhow | 1.0.104 | MIT OR Apache-2.0 | permissive | https://github.com/dtolnay/anyhow |
| atomic-waker | 1.1.2 | Apache-2.0 OR MIT | permissive | https://github.com/smol-rs/atomic-waker |
| base64 | 0.22.1 | MIT OR Apache-2.0 | permissive | https://github.com/marshallpierce/rust-base64 |
| base64 | 0.23.1 | MIT OR Apache-2.0 | permissive | https://github.com/marshallpierce/rust-base64 |
| base64ct | 1.8.3 | Apache-2.0 OR MIT | permissive | https://github.com/RustCrypto/formats |
| bit-set | 0.8.0 | Apache-2.0 OR MIT | permissive | https://github.com/contain-rs/bit-set |
| bit-vec | 0.8.0 | Apache-2.0 OR MIT | permissive | https://github.com/contain-rs/bit-vec |
| bitflags | 1.3.2 | MIT/Apache-2.0 | permissive | https://github.com/bitflags/bitflags |
| bitflags | 2.13.2 | MIT OR Apache-2.0 | permissive | https://github.com/bitflags/bitflags |
| block-buffer | 0.10.4 | MIT OR Apache-2.0 | permissive | https://github.com/RustCrypto/utils |
| block-buffer | 0.12.1 | MIT OR Apache-2.0 | permissive | https://github.com/RustCrypto/utils |
| block-padding | 0.4.2 | MIT OR Apache-2.0 | permissive | https://github.com/RustCrypto/utils |
| brotli | 9.0.0 | BSD-3-Clause AND MIT | permissive | https://github.com/dropbox/rust-brotli |
| brotli-decompressor | 5.0.3 | BSD-3-Clause/MIT | permissive | https://github.com/dropbox/rust-brotli-decompressor |
| brotli-decompressor | 6.0.1 | BSD-3-Clause/MIT | permissive | https://github.com/dropbox/rust-brotli-decompressor |
| bs58 | 0.5.1 | MIT/Apache-2.0 | permissive | https://github.com/Nullus157/bs58-rs |
| bumpalo | 3.20.3 | MIT OR Apache-2.0 | permissive | https://github.com/fitzgen/bumpalo |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT | permissive | https://github.com/Lokathor/bytemuck |
| byteorder | 1.5.0 | Unlicense OR MIT | permissive | https://github.com/BurntSushi/byteorder |
| byteorder-lite | 0.1.0 | Unlicense OR MIT | permissive | https://github.com/image-rs/byteorder-lite |
| bytes | 1.12.1 | MIT | permissive | https://github.com/tokio-rs/bytes |
| camino | 1.2.6 | MIT OR Apache-2.0 | permissive | https://github.com/camino-rs/camino |
| cargo_metadata | 0.19.2 | MIT | permissive | https://github.com/oli-obk/cargo_metadata |
| cargo-platform | 0.1.9 | MIT OR Apache-2.0 | permissive | https://github.com/rust-lang/cargo |
| cbc | 0.2.1 | MIT OR Apache-2.0 | permissive | https://github.com/RustCrypto/block-modes |
| cfb | 0.14.0 | MIT | permissive | https://github.com/mdsteele/rust-cfb |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | permissive | https://github.com/rust-lang/cfg-if |
| chacha20 | 0.10.2 | MIT OR Apache-2.0 | permissive | https://github.com/RustCrypto/stream-ciphers |
| chrono | 0.4.45 | MIT OR Apache-2.0 | permissive | https://github.com/chronotope/chrono |
| cipher | 0.5.2 | MIT OR Apache-2.0 | permissive | https://github.com/RustCrypto/traits |
| color_quant | 1.1.0 | MIT | permissive | https://github.com/image-rs/color_quant |
| const-oid | 0.10.2 | Apache-2.0 OR MIT | permissive | https://github.com/RustCrypto/formats |
| const-oid | 0.9.6 | Apache-2.0 OR MIT | permissive | https://github.com/RustCrypto/formats/tree/master/const-oid |
| cookie | 0.18.2 | MIT OR Apache-2.0 | permissive | https://github.com/SergioBenitez/cookie-rs |
| core_detect | 1.0.0 | MIT/Apache-2.0 | permissive | https://github.com/thomcc/core_detect |
| cpubits | 0.1.1 | MIT OR Apache-2.0 | permissive | https://github.com/RustCrypto/utils |
| cpufeatures | 0.2.17 | MIT OR Apache-2.0 | permissive | https://github.com/RustCrypto/utils |
| cpufeatures | 0.3.1 | MIT OR Apache-2.0 | permissive | https://github.com/RustCrypto/utils |
| crc32fast | 1.5.2 | MIT OR Apache-2.0 | permissive | https://github.com/srijs/rust-crc32fast |
| crossbeam-channel | 0.5.17 | MIT OR Apache-2.0 | permissive | https://github.com/crossbeam-rs/crossbeam |
| crossbeam-deque | 0.8.8 | MIT OR Apache-2.0 | permissive | https://github.com/crossbeam-rs/crossbeam |
| crossbeam-epoch | 0.9.21 | MIT OR Apache-2.0 | permissive | https://github.com/crossbeam-rs/crossbeam |
| crossbeam-utils | 0.8.23 | MIT OR Apache-2.0 | permissive | https://github.com/crossbeam-rs/crossbeam |
| crypto-common | 0.1.7 | MIT OR Apache-2.0 | permissive | https://github.com/RustCrypto/traits |
| crypto-common | 0.2.2 | MIT OR Apache-2.0 | permissive | https://github.com/RustCrypto/traits |
| cssparser | 0.37.0 | MPL-2.0 | weak copyleft / review | https://github.com/servo/rust-cssparser |
| cssparser-macros | 0.7.1 | MPL-2.0 | weak copyleft / review | https://github.com/servo/rust-cssparser |
| csv | 1.4.0 | Unlicense/MIT | permissive | https://github.com/BurntSushi/rust-csv |
| csv-core | 0.1.13 | Unlicense/MIT | permissive | https://github.com/BurntSushi/rust-csv |
| ctor | 1.0.13 | Apache-2.0 OR MIT | permissive | https://github.com/mmastrac/linktime |
| curve25519-dalek | 4.1.3 | BSD-3-Clause | permissive | https://github.com/dalek-cryptography/curve25519-dalek/tree/main/curve25519-dalek |
| curve25519-dalek-derive | 0.1.1 | MIT/Apache-2.0 | permissive | https://github.com/dalek-cryptography/curve25519-dalek |
| darling | 0.24.1 | MIT | permissive | https://github.com/TedDriggs/darling |
| darling_core | 0.24.1 | MIT | permissive | https://github.com/TedDriggs/darling |
| darling_macro | 0.24.1 | MIT | permissive | https://github.com/TedDriggs/darling |
| defmt | 1.1.1 | MIT OR Apache-2.0 | permissive | https://github.com/knurling-rs/defmt |
| defmt-macros | 1.1.1 | MIT OR Apache-2.0 | permissive | https://github.com/knurling-rs/defmt |
| defmt-parser | 1.0.0 | MIT OR Apache-2.0 | permissive | https://github.com/knurling-rs/defmt |
| der | 0.7.10 | Apache-2.0 OR MIT | permissive | https://github.com/RustCrypto/formats/tree/master/der |
| deranged | 0.5.8 | MIT OR Apache-2.0 | permissive | https://github.com/jhpratt/deranged |
| derive_more | 2.1.1 | MIT | permissive | https://github.com/JelteF/derive_more |
| derive_more-impl | 2.1.1 | MIT | permissive | https://github.com/JelteF/derive_more |
| digest | 0.10.7 | MIT OR Apache-2.0 | permissive | https://github.com/RustCrypto/traits |
| digest | 0.11.3 | MIT OR Apache-2.0 | permissive | https://github.com/RustCrypto/traits |
| directories | 6.0.0 | MIT OR Apache-2.0 | permissive | https://github.com/soc/directories-rs |
| dirs | 7.0.0 | MIT OR Apache-2.0 | permissive | https://codeberg.org/dirs/dirs-rs |
| dirs-sys | 0.5.0 | MIT OR Apache-2.0 | permissive | https://github.com/dirs-dev/dirs-sys-rs |
| displaydoc | 0.2.7 | MIT OR Apache-2.0 | permissive | https://github.com/yaahc/displaydoc |
| dom_query | 0.28.0 | MIT | permissive | https://github.com/niklak/dom_query |
| dpi | 0.1.2 | Apache-2.0 AND MIT | permissive | https://github.com/rust-windowing/winit |
| dtoa | 1.0.11 | MIT OR Apache-2.0 | permissive | https://github.com/dtolnay/dtoa |
| dtoa-short | 0.3.5 | MPL-2.0 | weak copyleft / review | https://github.com/upsuper/dtoa-short |
| dunce | 1.0.5 | CC0-1.0 OR MIT-0 OR Apache-2.0 | permissive | https://gitlab.com/kornelski/dunce |
| dyn-clone | 1.0.20 | MIT OR Apache-2.0 | permissive | https://github.com/dtolnay/dyn-clone |
| ecb | 0.2.1 | MIT OR Apache-2.0 | permissive | https://github.com/RustCrypto/block-modes |
| ed25519 | 2.2.3 | Apache-2.0 OR MIT | permissive | https://github.com/RustCrypto/signatures/tree/master/ed25519 |
| ed25519-dalek | 2.2.0 | BSD-3-Clause | permissive | https://github.com/dalek-cryptography/curve25519-dalek/tree/main/ed25519-dalek |
| either | 1.18.0 | MIT OR Apache-2.0 | permissive | https://github.com/rayon-rs/either |
| encoding_rs | 0.8.42 | (Apache-2.0 OR MIT) AND BSD-3-Clause | permissive | https://github.com/hsivonen/encoding_rs |
| equivalent | 1.0.2 | Apache-2.0 OR MIT | permissive | https://github.com/indexmap-rs/equivalent |
| erased-serde | 0.4.10 | MIT OR Apache-2.0 | permissive | https://github.com/dtolnay/erased-serde |
| fallible-iterator | 0.3.0 | MIT/Apache-2.0 | permissive | https://github.com/sfackler/rust-fallible-iterator |
| fallible-streaming-iterator | 0.1.9 | MIT/Apache-2.0 | permissive | https://github.com/sfackler/fallible-streaming-iterator |
| fastrand | 2.5.0 | Apache-2.0 OR MIT | permissive | https://github.com/smol-rs/fastrand |
| fdeflate | 0.3.7 | MIT OR Apache-2.0 | permissive | https://github.com/image-rs/fdeflate |
| flate2 | 1.1.10 | MIT OR Apache-2.0 | permissive | https://github.com/rust-lang/flate2-rs |
| fnv | 1.0.7 | Apache-2.0 / MIT | permissive | https://github.com/servo/rust-fnv |
| foldhash | 0.1.5 | Zlib | permissive | https://github.com/orlp/foldhash |
| foldhash | 0.2.0 | Zlib | permissive | https://github.com/orlp/foldhash |
| form_urlencoded | 1.2.2 | MIT OR Apache-2.0 | permissive | https://github.com/servo/rust-url |
| fs4 | 0.13.1 | MIT OR Apache-2.0 | permissive | https://github.com/al8n/fs4-rs |
| futures-channel | 0.3.34 | MIT OR Apache-2.0 | permissive | https://github.com/rust-lang/futures-rs |
| futures-core | 0.3.34 | MIT OR Apache-2.0 | permissive | https://github.com/rust-lang/futures-rs |
| futures-io | 0.3.34 | MIT OR Apache-2.0 | permissive | https://github.com/rust-lang/futures-rs |
| futures-macro | 0.3.34 | MIT OR Apache-2.0 | permissive | https://github.com/rust-lang/futures-rs |
| futures-sink | 0.3.34 | MIT OR Apache-2.0 | permissive | https://github.com/rust-lang/futures-rs |
| futures-task | 0.3.34 | MIT OR Apache-2.0 | permissive | https://github.com/rust-lang/futures-rs |
| futures-util | 0.3.34 | MIT OR Apache-2.0 | permissive | https://github.com/rust-lang/futures-rs |
| generic-array | 0.14.7 | MIT | permissive | https://github.com/fizyk20/generic-array |
| getrandom | 0.2.17 | MIT OR Apache-2.0 | permissive | https://github.com/rust-random/getrandom |
| getrandom | 0.3.4 | MIT OR Apache-2.0 | permissive | https://github.com/rust-random/getrandom |
| getrandom | 0.4.3 | MIT OR Apache-2.0 | permissive | https://github.com/rust-random/getrandom |
| gif | 0.14.2 | MIT OR Apache-2.0 | permissive | https://github.com/image-rs/image-gif |
| glob | 0.3.4 | MIT OR Apache-2.0 | permissive | https://github.com/rust-lang/glob |
| hashbrown | 0.12.3 | MIT OR Apache-2.0 | permissive | https://github.com/rust-lang/hashbrown |
| hashbrown | 0.15.5 | MIT OR Apache-2.0 | permissive | https://github.com/rust-lang/hashbrown |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 | permissive | https://github.com/rust-lang/hashbrown |
| hashlink | 0.10.0 | MIT OR Apache-2.0 | permissive | https://github.com/kyren/hashlink |
| heck | 0.5.0 | MIT OR Apache-2.0 | permissive | https://github.com/withoutboats/heck |
| hex | 0.4.3 | MIT OR Apache-2.0 | permissive | https://github.com/KokaKiwi/rust-hex |
| html5ever | 0.39.0 | MIT OR Apache-2.0 | permissive | https://github.com/servo/html5ever |
| http | 1.5.0 | MIT OR Apache-2.0 | permissive | https://github.com/hyperium/http |
| http-body | 1.1.0 | MIT | permissive | https://github.com/hyperium/http-body |
| http-body-util | 0.1.5 | MIT | permissive | https://github.com/hyperium/http-body |
| http-range | 0.1.5 | MIT | permissive | https://github.com/bancek/rust-http-range |
| httparse | 1.10.1 | MIT OR Apache-2.0 | permissive | https://github.com/seanmonstar/httparse |
| hybrid-array | 0.4.15 | MIT OR Apache-2.0 | permissive | https://github.com/RustCrypto/hybrid-array |
| hyper | 1.11.1 | MIT | permissive | https://github.com/hyperium/hyper |
| hyper-rustls | 0.27.10 | Apache-2.0 OR ISC OR MIT | permissive | https://github.com/rustls/hyper-rustls |
| hyper-util | 0.1.21 | MIT | permissive | https://github.com/hyperium/hyper-util |
| ico | 0.5.0 | MIT | permissive | https://github.com/mdsteele/rust-ico |
| icu_collections | 2.3.0 | Unicode-3.0 | permissive | https://github.com/unicode-org/icu4x |
| icu_locale_core | 2.3.0 | Unicode-3.0 | permissive | https://github.com/unicode-org/icu4x |
| icu_normalizer | 2.3.0 | Unicode-3.0 | permissive | https://github.com/unicode-org/icu4x |
| icu_normalizer_data | 2.3.0 | Unicode-3.0 | permissive | https://github.com/unicode-org/icu4x |
| icu_properties | 2.3.0 | Unicode-3.0 | permissive | https://github.com/unicode-org/icu4x |
| icu_properties_data | 2.3.0 | Unicode-3.0 | permissive | https://github.com/unicode-org/icu4x |
| icu_provider | 2.3.1 | Unicode-3.0 | permissive | https://github.com/unicode-org/icu4x |
| ident_case | 1.0.1 | MIT/Apache-2.0 | permissive | https://github.com/TedDriggs/ident_case |
| idna | 1.1.0 | MIT OR Apache-2.0 | permissive | https://github.com/servo/rust-url/ |
| idna_adapter | 1.2.2 | Apache-2.0 OR MIT | permissive | https://github.com/hsivonen/idna_adapter |
| image | 0.25.10 | MIT OR Apache-2.0 | permissive | https://github.com/image-rs/image |
| image-webp | 0.2.4 | MIT OR Apache-2.0 | permissive | https://github.com/image-rs/image-webp |
| indexmap | 1.9.3 | Apache-2.0 OR MIT | permissive | https://github.com/bluss/indexmap |
| indexmap | 2.14.2 | Apache-2.0 OR MIT | permissive | https://github.com/indexmap-rs/indexmap |
| infer | 0.22.0 | MIT | permissive | https://github.com/bojand/infer |
| inout | 0.2.2 | MIT OR Apache-2.0 | permissive | https://github.com/RustCrypto/utils |
| ipnet | 2.12.2 | MIT OR Apache-2.0 | permissive | https://github.com/krisprice/ipnet |
| itoa | 1.0.18 | MIT OR Apache-2.0 | permissive | https://github.com/dtolnay/itoa |
| jiff | 0.2.37 | Unlicense OR MIT | permissive | https://github.com/BurntSushi/jiff |
| jiff-core | 0.1.1 | Unlicense OR MIT | permissive | https://github.com/BurntSushi/jiff |
| jiff-tzdb | 0.1.8 | Unlicense OR MIT | permissive | https://github.com/BurntSushi/jiff |
| jiff-tzdb-platform | 0.1.3 | Unlicense OR MIT | permissive | https://github.com/BurntSushi/jiff |
| json-patch | 4.2.0 | MIT/Apache-2.0 | permissive | https://github.com/idubrov/json-patch |
| jsonptr | 0.7.1 | MIT OR Apache-2.0 | permissive | https://github.com/chanced/jsonptr |
| keyboard-types | 0.8.3 | MIT OR Apache-2.0 | permissive | https://github.com/rust-windowing/keyboard-types |
| lazy_static | 1.5.0 | MIT OR Apache-2.0 | permissive | https://github.com/rust-lang-nursery/lazy-static.rs |
| libc | 0.2.189 | MIT OR Apache-2.0 | permissive | https://github.com/rust-lang/libc |
| libsqlite3-sys | 0.35.0 | MIT | permissive | https://github.com/rusqlite/rusqlite |
| litemap | 0.8.3 | Unicode-3.0 | permissive | https://github.com/unicode-org/icu4x |
| lock_api | 0.4.14 | MIT OR Apache-2.0 | permissive | https://github.com/Amanieu/parking_lot |
| log | 0.4.34 | MIT OR Apache-2.0 | permissive | https://github.com/rust-lang/log |
| lopdf | 0.45.0 | MIT | permissive | https://github.com/J-F-Liu/lopdf |
| lru-slab | 0.1.3 | MIT OR Apache-2.0 OR Zlib | permissive | https://github.com/Ralith/lru-slab |
| markup5ever | 0.39.0 | MIT OR Apache-2.0 | permissive | https://github.com/servo/html5ever |
| matchers | 0.2.0 | MIT | permissive | https://github.com/hawkw/matchers |
| md-5 | 0.11.0 | MIT OR Apache-2.0 | permissive | https://github.com/RustCrypto/hashes |
| memchr | 2.8.3 | Unlicense OR MIT | permissive | https://github.com/BurntSushi/memchr |
| mime | 0.3.17 | MIT OR Apache-2.0 | permissive | https://github.com/hyperium/mime |
| miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 | permissive | https://github.com/Frommi/miniz_oxide/tree/master/miniz_oxide |
| miniz_oxide | 0.9.1 | MIT OR Zlib OR Apache-2.0 | permissive | https://github.com/Frommi/miniz_oxide/tree/master/miniz_oxide |
| mio | 1.2.3 | MIT | permissive | https://github.com/tokio-rs/mio |
| moxcms | 0.8.1 | BSD-3-Clause OR Apache-2.0 | permissive | https://github.com/awxkee/moxcms |
| muda | 0.20.0 | Apache-2.0 OR MIT | permissive | https://github.com/tauri-apps/muda |
| multiversion_no_op | 1.0.0 | Apache-2.0 OR MIT | permissive | https://github.com/hsivonen/multiversion_no_op |
| new_debug_unreachable | 1.0.6 | MIT | permissive | https://github.com/mbrubeck/rust-debug-unreachable |
| nom | 8.0.0 | MIT | permissive | https://github.com/rust-bakery/nom |
| ntapi | 0.4.3 | Apache-2.0 OR MIT | permissive | https://github.com/MSxDOS/ntapi |
| nu-ansi-term | 0.50.3 | MIT | permissive | https://github.com/nushell/nu-ansi-term |
| num-conv | 0.2.2 | MIT OR Apache-2.0 | permissive | https://github.com/jhpratt/num-conv |
| num-traits | 0.2.19 | MIT OR Apache-2.0 | permissive | https://github.com/rust-num/num-traits |
| once_cell | 1.21.4 | MIT OR Apache-2.0 | permissive | https://github.com/matklad/once_cell |
| open | 5.4.4 | MIT | permissive | https://github.com/Byron/open-rs |
| option-ext | 0.2.0 | MPL-2.0 | weak copyleft / review | https://github.com/soc/option-ext |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 | permissive | https://github.com/Amanieu/parking_lot |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 | permissive | https://github.com/Amanieu/parking_lot |
| pdf-writer | 0.13.0 | MIT OR Apache-2.0 | permissive | https://github.com/typst/pdf-writer |
| percent-encoding | 2.3.2 | MIT OR Apache-2.0 | permissive | https://github.com/servo/rust-url/ |
| phf | 0.13.1 | MIT | permissive | https://github.com/rust-phf/rust-phf |
| phf_generator | 0.13.1 | MIT | permissive | https://github.com/rust-phf/rust-phf |
| phf_macros | 0.13.1 | MIT | permissive | https://github.com/rust-phf/rust-phf |
| phf_shared | 0.13.1 | MIT | permissive | https://github.com/rust-phf/rust-phf |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT | permissive | https://github.com/taiki-e/pin-project-lite |
| pkcs8 | 0.10.2 | Apache-2.0 OR MIT | permissive | https://github.com/RustCrypto/formats/tree/master/pkcs8 |
| plist | 1.10.1 | MIT | permissive | https://github.com/ebarnard/rust-plist/ |
| png | 0.17.16 | MIT OR Apache-2.0 | permissive | https://github.com/image-rs/image-png |
| png | 0.18.1 | MIT OR Apache-2.0 | permissive | https://github.com/image-rs/image-png |
| potential_utf | 0.1.6 | Unicode-3.0 | permissive | https://github.com/unicode-org/icu4x |
| powerfmt | 0.2.0 | MIT OR Apache-2.0 | permissive | https://github.com/jhpratt/powerfmt |
| ppv-lite86 | 0.2.21 | MIT OR Apache-2.0 | permissive | https://github.com/cryptocorrosion/cryptocorrosion |
| precomputed-hash | 0.1.1 | MIT | permissive | https://github.com/emilio/precomputed-hash |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | permissive | https://github.com/dtolnay/proc-macro2 |
| pxfm | 0.1.30 | BSD-3-Clause OR Apache-2.0 | permissive | https://github.com/awxkee/pxfm |
| quick-error | 2.0.1 | MIT/Apache-2.0 | permissive | http://github.com/tailhook/quick-error |
| quick-xml | 0.42.0 | MIT | permissive | https://github.com/tafia/quick-xml |
| quinn | 0.11.12 | MIT OR Apache-2.0 | permissive | https://github.com/quinn-rs/quinn |
| quinn-proto | 0.11.18 | MIT OR Apache-2.0 | permissive | https://github.com/quinn-rs/quinn |
| quinn-udp | 0.5.15 | MIT OR Apache-2.0 | permissive | https://github.com/quinn-rs/quinn |
| quote | 1.0.47 | MIT OR Apache-2.0 | permissive | https://github.com/dtolnay/quote |
| rand | 0.10.3 | MIT OR Apache-2.0 | permissive | https://github.com/rust-random/rand |
| rand | 0.8.8 | MIT OR Apache-2.0 | permissive | https://github.com/rust-random/rand |
| rand_chacha | 0.3.1 | MIT OR Apache-2.0 | permissive | https://github.com/rust-random/rand |
| rand_core | 0.10.1 | MIT OR Apache-2.0 | permissive | https://github.com/rust-random/rand_core |
| rand_core | 0.6.4 | MIT OR Apache-2.0 | permissive | https://github.com/rust-random/rand |
| rand_pcg | 0.10.2 | MIT OR Apache-2.0 | permissive | https://github.com/rust-random/rngs |
| rangemap | 1.8.0 | MIT/Apache-2.0 | permissive | https://github.com/jeffparsons/rangemap |
| raw-window-handle | 0.6.2 | MIT OR Apache-2.0 OR Zlib | permissive | https://github.com/rust-windowing/raw-window-handle |
| rayon | 1.12.0 | MIT OR Apache-2.0 | permissive | https://github.com/rayon-rs/rayon |
| rayon-core | 1.13.0 | MIT OR Apache-2.0 | permissive | https://github.com/rayon-rs/rayon |
| ref-cast | 1.0.27 | MIT OR Apache-2.0 | permissive | https://github.com/dtolnay/ref-cast |
| ref-cast-impl | 1.0.27 | MIT OR Apache-2.0 | permissive | https://github.com/dtolnay/ref-cast |
| regex | 1.13.1 | MIT OR Apache-2.0 | permissive | https://github.com/rust-lang/regex |
| regex-automata | 0.4.18 | MIT OR Apache-2.0 | permissive | https://github.com/rust-lang/regex |
| regex-syntax | 0.8.11 | MIT OR Apache-2.0 | permissive | https://github.com/rust-lang/regex |
| reqwest | 0.12.28 | MIT OR Apache-2.0 | permissive | https://github.com/seanmonstar/reqwest |
| rfd | 0.16.0 | MIT | permissive | https://github.com/PolyMeilex/rfd |
| ring | 0.17.14 | Apache-2.0 AND ISC | permissive | https://github.com/briansmith/ring |
| rusqlite | 0.37.0 | MIT | permissive | https://github.com/rusqlite/rusqlite |
| rust_xlsxwriter | 0.90.2 | MIT OR Apache-2.0 | permissive | https://github.com/jmcnamara/rust_xlsxwriter |
| rustc-hash | 2.1.3 | Apache-2.0 OR MIT | permissive | https://github.com/rust-lang/rustc-hash |
| rustls | 0.23.45 | Apache-2.0 OR ISC OR MIT | permissive | https://github.com/rustls/rustls |
| rustls-pki-types | 1.15.1 | MIT OR Apache-2.0 | permissive | https://github.com/rustls/pki-types |
| rustls-webpki | 0.103.15 | ISC | permissive | https://github.com/rustls/webpki |
| ryu | 1.0.23 | Apache-2.0 OR BSL-1.0 | permissive | https://github.com/dtolnay/ryu |
| same-file | 1.0.6 | Unlicense/MIT | permissive | https://github.com/BurntSushi/same-file |
| schemars | 0.8.22 | MIT | permissive | https://github.com/GREsau/schemars |
| schemars | 0.9.0 | MIT | permissive | https://github.com/GREsau/schemars |
| schemars | 1.2.2 | MIT | permissive | https://github.com/GREsau/schemars |
| schemars_derive | 0.8.22 | MIT | permissive | https://github.com/GREsau/schemars |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 | permissive | https://github.com/bluss/scopeguard |
| selectors | 0.38.0 | MPL-2.0 | weak copyleft / review | https://github.com/servo/stylo |
| semver | 1.0.28 | MIT OR Apache-2.0 | permissive | https://github.com/dtolnay/semver |
| serde | 1.0.229 | MIT OR Apache-2.0 | permissive | https://github.com/serde-rs/serde |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | permissive | https://github.com/serde-rs/serde |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 | permissive | https://github.com/serde-rs/serde |
| serde_derive_internals | 0.29.1 | MIT OR Apache-2.0 | permissive | https://github.com/serde-rs/serde |
| serde_json | 1.0.151 | MIT OR Apache-2.0 | permissive | https://github.com/serde-rs/json |
| serde_repr | 0.1.21 | MIT OR Apache-2.0 | permissive | https://github.com/dtolnay/serde-repr |
| serde_spanned | 1.1.1 | MIT OR Apache-2.0 | permissive | https://github.com/toml-rs/toml |
| serde_urlencoded | 0.7.1 | MIT/Apache-2.0 | permissive | https://github.com/nox/serde_urlencoded |
| serde_with | 3.24.0 | MIT OR Apache-2.0 | permissive | https://github.com/jonasbb/serde_with/ |
| serde_with_macros | 3.24.0 | MIT OR Apache-2.0 | permissive | https://github.com/jonasbb/serde_with/ |
| serde-untagged | 0.1.9 | MIT OR Apache-2.0 | permissive | https://github.com/dtolnay/serde-untagged |
| serialize-to-javascript | 0.1.2 | MIT OR Apache-2.0 | permissive | https://github.com/chippers/serialize-to-javascript |
| serialize-to-javascript-impl | 0.1.2 | MIT OR Apache-2.0 | permissive | https://github.com/chippers/serialize-to-javascript |
| servo_arc | 0.4.3 | MIT OR Apache-2.0 | permissive | https://github.com/servo/stylo |
| sha2 | 0.10.9 | MIT OR Apache-2.0 | permissive | https://github.com/RustCrypto/hashes |
| sha2 | 0.11.0 | MIT OR Apache-2.0 | permissive | https://github.com/RustCrypto/hashes |
| sharded-slab | 0.1.7 | MIT | permissive | https://github.com/hawkw/sharded-slab |
| signature | 2.2.0 | Apache-2.0 OR MIT | permissive | https://github.com/RustCrypto/traits/tree/master/signature |
| simd-adler32 | 0.3.10 | MIT | permissive | https://github.com/mcountryman/simd-adler32 |
| simdutf8 | 0.1.5 | MIT OR Apache-2.0 | permissive | https://github.com/rusticstuff/simdutf8 |
| similar | 2.7.0 | Apache-2.0 | permissive | https://github.com/mitsuhiko/similar |
| siphasher | 1.0.4 | MIT OR Apache-2.0 | permissive | https://github.com/jedisct1/rust-siphash |
| slab | 0.4.12 | MIT | permissive | https://github.com/tokio-rs/slab |
| smallvec | 1.16.2 | MIT OR Apache-2.0 | permissive | https://github.com/servo/rust-smallvec |
| socket2 | 0.6.5 | MIT OR Apache-2.0 | permissive | https://github.com/rust-lang/socket2 |
| softbuffer | 0.4.8 | MIT OR Apache-2.0 | permissive | https://github.com/rust-windowing/softbuffer |
| spki | 0.7.3 | Apache-2.0 OR MIT | permissive | https://github.com/RustCrypto/formats/tree/master/spki |
| stable_deref_trait | 1.2.1 | MIT OR Apache-2.0 | permissive | https://github.com/storyyeller/stable_deref_trait |
| string_cache | 0.9.0 | MIT OR Apache-2.0 | permissive | https://github.com/servo/string-cache |
| stringprep | 0.1.5 | MIT/Apache-2.0 | permissive | https://github.com/sfackler/rust-stringprep |
| strsim | 0.11.1 | MIT | permissive | https://github.com/rapidfuzz/strsim-rs |
| subtle | 2.6.1 | BSD-3-Clause | permissive | https://github.com/dalek-cryptography/subtle |
| symlink | 0.1.0 | MIT/Apache-2.0 | permissive | https://gitlab.com/chris-morgan/symlink |
| syn | 2.0.119 | MIT OR Apache-2.0 | permissive | https://github.com/dtolnay/syn |
| syn | 3.0.6 | MIT OR Apache-2.0 | permissive | https://github.com/dtolnay/syn |
| sync_wrapper | 1.0.2 | Apache-2.0 | permissive | https://github.com/Actyx/sync_wrapper |
| synstructure | 0.14.0 | MIT | permissive | https://github.com/mystor/synstructure |
| sysinfo | 0.37.2 | MIT | permissive | https://github.com/GuillaumeGomez/sysinfo |
| tao | 0.37.1 | Apache-2.0 | permissive | https://github.com/tauri-apps/tao |
| tauri | 2.12.0 | Apache-2.0 OR MIT | permissive | https://github.com/tauri-apps/tauri |
| tauri-codegen | 2.7.0 | Apache-2.0 OR MIT | permissive | https://github.com/tauri-apps/tauri |
| tauri-macros | 2.7.0 | Apache-2.0 OR MIT | permissive | https://github.com/tauri-apps/tauri |
| tauri-plugin-dialog | 2.8.0 | Apache-2.0 OR MIT | permissive | https://github.com/tauri-apps/plugins-workspace |
| tauri-plugin-fs | 2.6.0 | Apache-2.0 OR MIT | permissive | https://github.com/tauri-apps/plugins-workspace |
| tauri-plugin-opener | 2.6.0 | Apache-2.0 OR MIT | permissive | https://github.com/tauri-apps/plugins-workspace |
| tauri-plugin-single-instance | 2.5.0 | Apache-2.0 OR MIT | permissive | https://github.com/tauri-apps/plugins-workspace |
| tauri-runtime | 2.12.0 | Apache-2.0 OR MIT | permissive | https://github.com/tauri-apps/tauri |
| tauri-runtime-wry | 2.12.0 | Apache-2.0 OR MIT | permissive | https://github.com/tauri-apps/tauri |
| tauri-utils | 2.10.0 | Apache-2.0 OR MIT | permissive | https://github.com/tauri-apps/tauri |
| tendril | 0.5.1 | MIT OR Apache-2.0 | permissive | https://github.com/servo/html5ever |
| termcolor | 1.4.1 | Unlicense OR MIT | permissive | https://github.com/BurntSushi/termcolor |
| thiserror | 2.0.21 | MIT OR Apache-2.0 | permissive | https://github.com/dtolnay/thiserror |
| thiserror-impl | 2.0.21 | MIT OR Apache-2.0 | permissive | https://github.com/dtolnay/thiserror |
| thread_local | 1.1.10 | MIT OR Apache-2.0 | permissive | https://github.com/Amanieu/thread_local-rs |
| time | 0.3.55 | MIT OR Apache-2.0 | permissive | https://github.com/time-rs/time |
| time-core | 0.1.9 | MIT OR Apache-2.0 | permissive | https://github.com/time-rs/time |
| time-macros | 0.2.32 | MIT OR Apache-2.0 | permissive | https://github.com/time-rs/time |
| tinystr | 0.8.4 | Unicode-3.0 | permissive | https://github.com/unicode-org/icu4x |
| tinyvec | 1.13.3 | Zlib OR Apache-2.0 OR MIT | permissive | https://github.com/Lokathor/tinyvec |
| tokio | 1.53.1 | MIT | permissive | https://github.com/tokio-rs/tokio |
| tokio-macros | 2.7.2 | MIT | permissive | https://github.com/tokio-rs/tokio |
| tokio-rustls | 0.26.6 | MIT OR Apache-2.0 | permissive | https://github.com/rustls/tokio-rustls |
| tokio-util | 0.7.19 | MIT | permissive | https://github.com/tokio-rs/tokio |
| toml | 1.1.6+spec-1.1.0 | MIT OR Apache-2.0 | permissive | https://github.com/toml-rs/toml |
| toml_datetime | 1.1.1+spec-1.1.0 | MIT OR Apache-2.0 | permissive | https://github.com/toml-rs/toml |
| toml_parser | 1.1.3+spec-1.1.0 | MIT OR Apache-2.0 | permissive | https://github.com/toml-rs/toml |
| toml_writer | 1.1.2+spec-1.1.0 | MIT OR Apache-2.0 | permissive | https://github.com/toml-rs/toml |
| tower | 0.5.3 | MIT | permissive | https://github.com/tower-rs/tower |
| tower-http | 0.6.11 | MIT | permissive | https://github.com/tower-rs/tower-http |
| tower-layer | 0.3.3 | MIT | permissive | https://github.com/tower-rs/tower |
| tower-service | 0.3.3 | MIT | permissive | https://github.com/tower-rs/tower |
| tracing | 0.1.44 | MIT | permissive | https://github.com/tokio-rs/tracing |
| tracing-appender | 0.2.5 | MIT | permissive | https://github.com/tokio-rs/tracing |
| tracing-attributes | 0.1.31 | MIT | permissive | https://github.com/tokio-rs/tracing |
| tracing-core | 0.1.36 | MIT | permissive | https://github.com/tokio-rs/tracing |
| tracing-log | 0.2.0 | MIT | permissive | https://github.com/tokio-rs/tracing |
| tracing-serde | 0.2.0 | MIT | permissive | https://github.com/tokio-rs/tracing |
| tracing-subscriber | 0.3.23 | MIT | permissive | https://github.com/tokio-rs/tracing |
| trash | 5.2.9 | MIT | permissive | https://github.com/ArturKovacs/trash |
| tray-icon | 0.25.1 | MIT OR Apache-2.0 | permissive | https://github.com/tauri-apps/tray-icon |
| try-lock | 0.2.5 | MIT | permissive | https://github.com/seanmonstar/try-lock |
| ts-rs | 11.1.0 | MIT | permissive | https://github.com/Aleph-Alpha/ts-rs |
| ts-rs-macros | 11.1.0 | MIT | permissive | https://github.com/Aleph-Alpha/ts-rs |
| typeid | 1.0.3 | MIT OR Apache-2.0 | permissive | https://github.com/dtolnay/typeid |
| typenum | 1.20.1 | MIT OR Apache-2.0 | permissive | https://github.com/paholg/typenum |
| unicode-bidi | 0.3.18 | MIT OR Apache-2.0 | permissive | https://github.com/servo/unicode-bidi |
| unicode-ident | 1.0.26 | (MIT OR Apache-2.0) AND Unicode-3.0 | permissive | https://github.com/dtolnay/unicode-ident |
| unicode-normalization | 0.1.25 | MIT OR Apache-2.0 | permissive | https://github.com/unicode-rs/unicode-normalization |
| unicode-properties | 0.1.4 | MIT/Apache-2.0 | permissive | https://github.com/unicode-rs/unicode-properties |
| unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 | permissive | https://github.com/unicode-rs/unicode-segmentation |
| untrusted | 0.9.0 | ISC | permissive | https://github.com/briansmith/untrusted |
| url | 2.5.8 | MIT OR Apache-2.0 | permissive | https://github.com/servo/rust-url |
| urlpattern | 0.6.0 | MIT | permissive | https://github.com/denoland/rust-urlpattern |
| utf8_iter | 1.0.4 | Apache-2.0 OR MIT | permissive | https://github.com/hsivonen/utf8_iter |
| uuid | 1.26.1 | Apache-2.0 OR MIT | permissive | https://github.com/uuid-rs/uuid |
| walkdir | 2.5.0 | Unlicense/MIT | permissive | https://github.com/BurntSushi/walkdir |
| want | 0.3.1 | MIT | permissive | https://github.com/seanmonstar/want |
| web_atoms | 0.2.6 | MIT OR Apache-2.0 | permissive | https://github.com/servo/html5ever |
| web-time | 1.1.0 | MIT OR Apache-2.0 | permissive | https://github.com/daxpedda/web-time |
| webpki-roots | 1.0.9 | CDLA-Permissive-2.0 | permissive | https://github.com/rustls/webpki-roots |
| webview2-com | 0.39.1 | MIT | permissive | https://github.com/wravery/webview2-rs |
| webview2-com-macros | 0.8.1 | MIT | permissive | https://github.com/wravery/webview2-rs |
| webview2-com-sys | 0.39.1 | MIT | permissive | https://github.com/wravery/webview2-rs |
| weezl | 0.1.12 | MIT OR Apache-2.0 | permissive | https://github.com/image-rs/weezl |
| weezl | 0.2.1 | MIT OR Apache-2.0 | permissive | https://github.com/image-rs/weezl |
| winapi | 0.3.9 | MIT/Apache-2.0 | permissive | https://github.com/retep998/winapi-rs |
| winapi-util | 0.1.11 | Unlicense OR MIT | permissive | https://github.com/BurntSushi/winapi-util |
| window-vibrancy | 0.8.1 | Apache-2.0 OR MIT | permissive | https://github.com/tauri-apps/tauri-plugin-vibrancy |
| windows | 0.61.3 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows | 0.62.2 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows_x86_64_msvc | 0.52.6 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows_x86_64_msvc | 0.53.1 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-collections | 0.2.0 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-collections | 0.3.2 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-core | 0.61.2 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-core | 0.62.2 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-future | 0.2.1 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-future | 0.3.2 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-implement | 0.60.2 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-interface | 0.59.3 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-link | 0.1.3 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-link | 0.2.1 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-numerics | 0.2.0 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-numerics | 0.3.1 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-result | 0.3.4 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-result | 0.4.1 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-strings | 0.4.2 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-strings | 0.5.1 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-sys | 0.59.0 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-sys | 0.60.2 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-targets | 0.52.6 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-targets | 0.53.5 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-threading | 0.1.0 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-threading | 0.2.1 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| windows-version | 0.1.7 | MIT OR Apache-2.0 | permissive | https://github.com/microsoft/windows-rs |
| winnow | 1.0.4 | MIT | permissive | https://github.com/winnow-rs/winnow |
| writeable | 0.6.4 | Unicode-3.0 | permissive | https://github.com/unicode-org/icu4x |
| wry | 0.57.0 | Apache-2.0 OR MIT | permissive | https://github.com/tauri-apps/wry |
| yoke | 0.8.3 | Unicode-3.0 | permissive | https://github.com/unicode-org/icu4x |
| yoke-derive | 0.8.3 | Unicode-3.0 | permissive | https://github.com/unicode-org/icu4x |
| zerocopy | 0.8.59 | BSD-2-Clause OR Apache-2.0 OR MIT | permissive | https://github.com/google/zerocopy |
| zerofrom | 0.1.8 | Unicode-3.0 | permissive | https://github.com/unicode-org/icu4x |
| zerofrom-derive | 0.1.8 | Unicode-3.0 | permissive | https://github.com/unicode-org/icu4x |
| zeroize | 1.9.0 | Apache-2.0 OR MIT | permissive | https://github.com/RustCrypto/utils |
| zerotrie | 0.2.5 | Unicode-3.0 | permissive | https://github.com/unicode-org/icu4x |
| zerovec | 0.11.8 | Unicode-3.0 | permissive | https://github.com/unicode-org/icu4x |
| zerovec-derive | 0.11.6 | Unicode-3.0 | permissive | https://github.com/unicode-org/icu4x |
| zip | 4.6.1 | MIT | permissive | https://github.com/zip-rs/zip2 |
| zlib-rs | 0.6.8 | Zlib | permissive | https://github.com/trifectatechfoundation/zlib-rs |
| zmij | 1.0.23 | MIT | permissive | https://github.com/dtolnay/zmij |
| zopfli | 0.8.3 | Apache-2.0 | permissive | https://github.com/zopfli-rs/zopfli |
| zune-core | 0.5.3 | MIT OR Apache-2.0 OR Zlib | permissive | https://github.com/etemesi254/zune-image |
| zune-jpeg | 0.5.15 | MIT OR Apache-2.0 OR Zlib | permissive | https://github.com/etemesi254/zune-image/tree/dev/crates/zune-jpeg |

## npm packages (production)

| Package | Version | License | Category | Source |
|---|---|---|---|---|
| @dnd-kit/accessibility | 3.1.1 | MIT | permissive | https://registry.npmjs.org/@dnd-kit/accessibility/-/accessibility-3.1.1.tgz |
| @dnd-kit/core | 6.3.1 | MIT | permissive | https://registry.npmjs.org/@dnd-kit/core/-/core-6.3.1.tgz |
| @dnd-kit/sortable | 10.0.0 | MIT | permissive | https://registry.npmjs.org/@dnd-kit/sortable/-/sortable-10.0.0.tgz |
| @dnd-kit/utilities | 3.2.2 | MIT | permissive | https://registry.npmjs.org/@dnd-kit/utilities/-/utilities-3.2.2.tgz |
| @floating-ui/core | 1.8.0 | MIT | permissive | https://registry.npmjs.org/@floating-ui/core/-/core-1.8.0.tgz |
| @floating-ui/dom | 1.8.0 | MIT | permissive | https://registry.npmjs.org/@floating-ui/dom/-/dom-1.8.0.tgz |
| @floating-ui/react-dom | 2.1.9 | MIT | permissive | https://registry.npmjs.org/@floating-ui/react-dom/-/react-dom-2.1.9.tgz |
| @floating-ui/utils | 0.2.12 | MIT | permissive | https://registry.npmjs.org/@floating-ui/utils/-/utils-0.2.12.tgz |
| @radix-ui/primitive | 1.1.7 | MIT | permissive | https://registry.npmjs.org/@radix-ui/primitive/-/primitive-1.1.7.tgz |
| @radix-ui/react-arrow | 1.1.15 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-arrow/-/react-arrow-1.1.15.tgz |
| @radix-ui/react-collection | 1.1.15 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-collection/-/react-collection-1.1.15.tgz |
| @radix-ui/react-compose-refs | 1.1.5 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-compose-refs/-/react-compose-refs-1.1.5.tgz |
| @radix-ui/react-context | 1.2.2 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-context/-/react-context-1.2.2.tgz |
| @radix-ui/react-context-menu | 2.3.7 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-context-menu/-/react-context-menu-2.3.7.tgz |
| @radix-ui/react-dialog | 1.1.23 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-dialog/-/react-dialog-1.1.23.tgz |
| @radix-ui/react-direction | 1.1.4 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-direction/-/react-direction-1.1.4.tgz |
| @radix-ui/react-dismissable-layer | 1.1.19 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-dismissable-layer/-/react-dismissable-layer-1.1.19.tgz |
| @radix-ui/react-dropdown-menu | 2.1.24 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-dropdown-menu/-/react-dropdown-menu-2.1.24.tgz |
| @radix-ui/react-focus-guards | 1.1.6 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-focus-guards/-/react-focus-guards-1.1.6.tgz |
| @radix-ui/react-focus-scope | 1.1.16 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-focus-scope/-/react-focus-scope-1.1.16.tgz |
| @radix-ui/react-id | 1.1.4 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-id/-/react-id-1.1.4.tgz |
| @radix-ui/react-menu | 2.1.24 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-menu/-/react-menu-2.1.24.tgz |
| @radix-ui/react-popover | 1.1.23 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-popover/-/react-popover-1.1.23.tgz |
| @radix-ui/react-popper | 1.3.7 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-popper/-/react-popper-1.3.7.tgz |
| @radix-ui/react-portal | 1.1.17 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-portal/-/react-portal-1.1.17.tgz |
| @radix-ui/react-presence | 1.1.10 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-presence/-/react-presence-1.1.10.tgz |
| @radix-ui/react-primitive | 2.1.10 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-primitive/-/react-primitive-2.1.10.tgz |
| @radix-ui/react-roving-focus | 1.1.19 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-roving-focus/-/react-roving-focus-1.1.19.tgz |
| @radix-ui/react-slot | 1.3.3 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-slot/-/react-slot-1.3.3.tgz |
| @radix-ui/react-tooltip | 1.2.16 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-tooltip/-/react-tooltip-1.2.16.tgz |
| @radix-ui/react-use-callback-ref | 1.1.4 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-use-callback-ref/-/react-use-callback-ref-1.1.4.tgz |
| @radix-ui/react-use-controllable-state | 1.2.6 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-use-controllable-state/-/react-use-controllable-state-1.2.6.tgz |
| @radix-ui/react-use-effect-event | 0.0.5 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-use-effect-event/-/react-use-effect-event-0.0.5.tgz |
| @radix-ui/react-use-is-hydrated | 0.1.3 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-use-is-hydrated/-/react-use-is-hydrated-0.1.3.tgz |
| @radix-ui/react-use-layout-effect | 1.1.4 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-use-layout-effect/-/react-use-layout-effect-1.1.4.tgz |
| @radix-ui/react-use-rect | 1.1.4 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-use-rect/-/react-use-rect-1.1.4.tgz |
| @radix-ui/react-use-size | 1.1.4 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-use-size/-/react-use-size-1.1.4.tgz |
| @radix-ui/react-visually-hidden | 1.2.11 | MIT | permissive | https://registry.npmjs.org/@radix-ui/react-visually-hidden/-/react-visually-hidden-1.2.11.tgz |
| @radix-ui/rect | 1.1.3 | MIT | permissive | https://registry.npmjs.org/@radix-ui/rect/-/rect-1.1.3.tgz |
| @tanstack/query-core | 5.104.0 | MIT | permissive | https://registry.npmjs.org/@tanstack/query-core/-/query-core-5.104.0.tgz |
| @tanstack/react-query | 5.104.0 | MIT | permissive | https://registry.npmjs.org/@tanstack/react-query/-/react-query-5.104.0.tgz |
| @tanstack/react-virtual | 3.14.13 | MIT | permissive | https://registry.npmjs.org/@tanstack/react-virtual/-/react-virtual-3.14.13.tgz |
| @tanstack/virtual-core | 3.17.11 | MIT | permissive | https://registry.npmjs.org/@tanstack/virtual-core/-/virtual-core-3.17.11.tgz |
| @tauri-apps/api | 2.12.0 | Apache-2.0 OR MIT | permissive | https://registry.npmjs.org/@tauri-apps/api/-/api-2.12.0.tgz |
| @tauri-apps/plugin-dialog | 2.8.0 | MIT OR Apache-2.0 | permissive | https://registry.npmjs.org/@tauri-apps/plugin-dialog/-/plugin-dialog-2.8.0.tgz |
| @types/react | 19.3.0 | MIT | permissive | https://registry.npmjs.org/@types/react/-/react-19.3.0.tgz |
| @types/react-dom | 19.3.0 | MIT | permissive | https://registry.npmjs.org/@types/react-dom/-/react-dom-19.3.0.tgz |
| aria-hidden | 1.2.6 | MIT | permissive | https://registry.npmjs.org/aria-hidden/-/aria-hidden-1.2.6.tgz |
| csstype | 3.2.3 | MIT | permissive | https://registry.npmjs.org/csstype/-/csstype-3.2.3.tgz |
| detect-node-es | 1.1.0 | MIT | permissive | https://registry.npmjs.org/detect-node-es/-/detect-node-es-1.1.0.tgz |
| get-nonce | 1.0.1 | MIT | permissive | https://registry.npmjs.org/get-nonce/-/get-nonce-1.0.1.tgz |
| lucide-react | 0.544.0 | ISC | permissive | https://registry.npmjs.org/lucide-react/-/lucide-react-0.544.0.tgz |
| orderedmap | 2.1.1 | MIT | permissive | https://registry.npmjs.org/orderedmap/-/orderedmap-2.1.1.tgz |
| prosemirror-commands | 1.7.2 | MIT | permissive | https://registry.npmjs.org/prosemirror-commands/-/prosemirror-commands-1.7.2.tgz |
| prosemirror-history | 1.5.1 | MIT | permissive | https://registry.npmjs.org/prosemirror-history/-/prosemirror-history-1.5.1.tgz |
| prosemirror-keymap | 1.2.3 | MIT | permissive | https://registry.npmjs.org/prosemirror-keymap/-/prosemirror-keymap-1.2.3.tgz |
| prosemirror-model | 1.25.12 | MIT | permissive | https://registry.npmjs.org/prosemirror-model/-/prosemirror-model-1.25.12.tgz |
| prosemirror-state | 1.4.4 | MIT | permissive | https://registry.npmjs.org/prosemirror-state/-/prosemirror-state-1.4.4.tgz |
| prosemirror-transform | 1.12.2 | MIT | permissive | https://registry.npmjs.org/prosemirror-transform/-/prosemirror-transform-1.12.2.tgz |
| prosemirror-view | 1.42.6 | MIT | permissive | https://registry.npmjs.org/prosemirror-view/-/prosemirror-view-1.42.6.tgz |
| react | 19.3.0 | MIT | permissive | https://registry.npmjs.org/react/-/react-19.3.0.tgz |
| react-dom | 19.3.0 | MIT | permissive | https://registry.npmjs.org/react-dom/-/react-dom-19.3.0.tgz |
| react-remove-scroll | 2.7.2 | MIT | permissive | https://registry.npmjs.org/react-remove-scroll/-/react-remove-scroll-2.7.2.tgz |
| react-remove-scroll-bar | 2.3.8 | MIT | permissive | https://registry.npmjs.org/react-remove-scroll-bar/-/react-remove-scroll-bar-2.3.8.tgz |
| react-style-singleton | 2.2.3 | MIT | permissive | https://registry.npmjs.org/react-style-singleton/-/react-style-singleton-2.2.3.tgz |
| rope-sequence | 1.3.4 | MIT | permissive | https://registry.npmjs.org/rope-sequence/-/rope-sequence-1.3.4.tgz |
| scheduler | 0.28.0 | MIT | permissive | https://registry.npmjs.org/scheduler/-/scheduler-0.28.0.tgz |
| tslib | 2.8.1 | 0BSD | permissive | https://registry.npmjs.org/tslib/-/tslib-2.8.1.tgz |
| use-callback-ref | 1.3.3 | MIT | permissive | https://registry.npmjs.org/use-callback-ref/-/use-callback-ref-1.3.3.tgz |
| use-sidecar | 1.1.3 | MIT | permissive | https://registry.npmjs.org/use-sidecar/-/use-sidecar-1.1.3.tgz |
| w3c-keyname | 2.2.8 | MIT | permissive | https://registry.npmjs.org/w3c-keyname/-/w3c-keyname-2.2.8.tgz |
| zustand | 5.0.15 | MIT | permissive | https://registry.npmjs.org/zustand/-/zustand-5.0.15.tgz |
