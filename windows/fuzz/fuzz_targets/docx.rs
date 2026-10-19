//! libFuzzer entry for the `docx` target (see src/lib.rs).
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| openframe_fuzz::docx(data));
