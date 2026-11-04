// swift-tools-version:6.0
// Straft Studio for the Mac: a SwiftUI shell over the shared Rust core (ADR-0027).
//
// Open this package in Xcode, or build and test it with `apps/macos/scripts/test.sh`.
// `apps/macos/scripts/build-core.sh` must run first: it builds the Rust core into
// Frameworks/StraftFFI.xcframework and writes Sources/StraftCore/Generated/straft_ffi.swift.
//
// Targets:
//   StraftFFI     the Rust core as a static library (build product, not committed)
//   StraftCore    the typed Swift interface to the core: Core, generated operations and types
//   StraftUI      screens and their models (no rules: every decision is made by the core)
//   StraftStudio  the app entry point
//   StraftE2EDriver  end-to-end driver injected into the release app by E2E/run.sh (AppKit only)
import PackageDescription

let package = Package(
    name: "StraftStudio",
    defaultLocalization: "en",
    platforms: [.macOS(.v14)],
    products: [
        .library(name: "StraftCore", targets: ["StraftCore"]),
        .library(name: "StraftUI", targets: ["StraftUI"]),
        .executable(name: "StraftStudio", targets: ["StraftStudio"]),
        // The end-to-end driver, injected into the release app by E2E/run.sh. Never in the app bundle.
        .library(name: "StraftE2EDriver", type: .dynamic, targets: ["StraftE2EDriver"]),
    ],
    targets: [
        .binaryTarget(name: "StraftFFI", path: "Frameworks/StraftFFI.xcframework"),
        .target(
            name: "StraftCore",
            dependencies: ["StraftFFI"],
            exclude: ["Generated/operations.json"],
            swiftSettings: [.swiftLanguageMode(.v5)],
            linkerSettings: [
                .linkedFramework("Security"),
                .linkedFramework("SystemConfiguration"),
                .linkedFramework("CoreFoundation"),
                .linkedLibrary("iconv"),
                .linkedLibrary("resolv"),
            ]
        ),
        .target(
            name: "StraftUI",
            dependencies: ["StraftCore"],
            resources: [
                .copy("Resources/themes.json"), .copy("Resources/macDefaultTheme.json"),
                .copy("Resources/elementRules.json"),
            ]
        ),
        .executableTarget(name: "StraftStudio", dependencies: ["StraftUI"]),
        .target(name: "StraftE2EShim"),
        .target(name: "StraftE2EDriver", dependencies: ["StraftE2EShim"], swiftSettings: [.swiftLanguageMode(.v5)]),
        .testTarget(name: "StraftCoreTests", dependencies: ["StraftCore"]),
        .testTarget(
            name: "StraftUITests",
            dependencies: ["StraftUI"],
            exclude: ["theme-reference.json"]
        ),
    ]
)
