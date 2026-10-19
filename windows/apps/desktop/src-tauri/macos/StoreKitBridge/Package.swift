// swift-tools-version:5.9
// StoreKit 2 bridge for the OpenFrame Studio Mac App Store build.
//
// NOT COMPILED YET. Written on a Windows machine with no Xcode. apps/desktop/src-tauri/build.rs
// runs `swift build -c release --product StoreKitBridge` for every macOS build of the app and links
// the static library (docs/engineering/37-platform-boundary.md). The Mac workflow
// (.github/workflows/mac.yml) is the first place it compiles.

import PackageDescription

let package = Package(
    name: "StoreKitBridge",
    platforms: [.macOS(.v13)],
    products: [
        .library(name: "StoreKitBridge", type: .static, targets: ["StoreKitBridge"]),
    ],
    targets: [
        .target(
            name: "StoreKitBridge",
            path: "Sources/StoreKitBridge",
            // The header documents the C interface for the Rust side, which declares the same
            // functions in platform/macos_storekit.rs. It is not compiled into this target.
            exclude: ["include"],
            linkerSettings: [.linkedFramework("StoreKit")]
        ),
    ]
)
