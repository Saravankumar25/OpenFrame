//! libFuzzer entry for the `fountain` target (see src/lib.rs).
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| straft_fuzz::fountain(data));
