//! Prints every registered operation (name, kind, argument and result types) as JSON, for the
//! Swift type generator (`scripts/gen-swift-types.mjs`). Development tool only.

fn main() {
    println!("{}", openframe_ffi::operation_inventory_json());
}
