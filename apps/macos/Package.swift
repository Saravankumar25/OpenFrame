// swift-tools-version:6.0
// OpenFrame Studio for the Mac: a SwiftUI shell over the shared Rust core (ADR-0027).
//
// Open this package in Xcode, or build and test it with `apps/macos/scripts/test.sh`.
// `apps/macos/scripts/build-core.sh` must run first: it builds the Rust core into
// Frameworks/OpenFrameFFI.xcframework and writes Sources/OpenFrameCore/Generated/openframe_ffi.swift.
//
// Targets:
//   OpenFrameFFI     the Rust core as a static library (build product, not committed)
//   OpenFrameCore    the typed Swift interface to the core: Core, generated operations and types
//   OpenFrameUI      screens and their models (no rules: every decision is made by the core)
//   OpenFrameStudio  the app entry point
import PackageDescription

let package = Package(
    name: "OpenFrameStudio",
    defaultLocalization: "en",
    platforms: [.macOS(.v14)],
    products: [
        .library(name: "OpenFrameCore", targets: ["OpenFrameCore"]),
        .library(name: "OpenFrameUI", targets: ["OpenFrameUI"]),
        .executable(name: "OpenFrameStudio", targets: ["OpenFrameStudio"]),
    ],
    targets: [
        .binaryTarget(name: "OpenFrameFFI", path: "Frameworks/OpenFrameFFI.xcframework"),
        .target(
            name: "OpenFrameCore",
            dependencies: ["OpenFrameFFI"],
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
            name: "OpenFrameUI",
            dependencies: ["OpenFrameCore"],
            resources: [.copy("Resources/themes.json")]
        ),
        .executableTarget(name: "OpenFrameStudio", dependencies: ["OpenFrameUI"]),
        .testTarget(name: "OpenFrameCoreTests", dependencies: ["OpenFrameCore"]),
        .testTarget(
            name: "OpenFrameUITests",
            dependencies: ["OpenFrameUI"],
            exclude: ["theme-reference.json"]
        ),
    ]
)
