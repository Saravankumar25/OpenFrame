//! UniFFI's binding generator, run by `apps/macos/scripts/build-core.sh` to write the Swift side
//! of the interface from the compiled library. Development tool only.

fn main() {
    uniffi::uniffi_bindgen_main()
}
