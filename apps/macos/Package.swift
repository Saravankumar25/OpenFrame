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
//   OpenFrameE2EDriver  end-to-end driver injected into the release app by E2E/run.sh (AppKit only)
import PackageDescription

let package = Package(
    name: "OpenFrameStudio",
    defaultLocalization: "en",
    platforms: [.macOS(.v14)],
    products: [
        .library(name: "OpenFrameCore", targets: ["OpenFrameCore"]),
        .library(name: "OpenFrameUI", targets: ["OpenFrameUI"]),
        .executable(name: "OpenFrameStudio", targets: ["OpenFrameStudio"]),
        // The end-to-end driver, injected into the release app by E2E/run.sh. Never in the app bundle.
        .library(name: "OpenFrameE2EDriver", type: .dynamic, targets: ["OpenFrameE2EDriver"]),
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
            resources: [
                .copy("Resources/themes.json"), .copy("Resources/macDefaultTheme.json"),
                .copy("Resources/elementRules.json"),
            ]
        ),
        .executableTarget(name: "OpenFrameStudio", dependencies: ["OpenFrameUI"]),
        .target(name: "OpenFrameE2EShim"),
        .target(name: "OpenFrameE2EDriver", dependencies: ["OpenFrameE2EShim"], swiftSettings: [.swiftLanguageMode(.v5)]),
        .testTarget(name: "OpenFrameCoreTests", dependencies: ["OpenFrameCore"]),
        .testTarget(
            name: "OpenFrameUITests",
            dependencies: ["OpenFrameUI"],
            exclude: ["theme-reference.json"]
        ),
    ]
)
