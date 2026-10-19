//! libFuzzer entry for the `package_archive` target (see src/lib.rs).
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| openframe_fuzz::package_archive(data));
