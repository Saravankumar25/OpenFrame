import Foundation

/// Finds a file shipped with OpenFrameUI. In the app bundle the resource bundle sits in
/// Contents/Resources; SwiftPM's own `Bundle.module` looks next to the executable and at the
/// build folder, and stops the app when neither exists, which is wrong on a person's Mac.
enum PackagedResources {
    static let bundleName = "OpenFrameStudio_OpenFrameUI.bundle"

    static func url(_ name: String, _ ext: String) -> URL? {
        let candidates = [Bundle.main.resourceURL, Bundle.main.bundleURL, Bundle.main.executableURL?.deletingLastPathComponent()]
        for base in candidates.compactMap({ $0 }) {
            if let bundle = Bundle(url: base.appendingPathComponent(bundleName)),
               let url = bundle.url(forResource: name, withExtension: ext) {
                return url
            }
        }
        // Tests and `swift run`: the package build folder.
        return Bundle.module.url(forResource: name, withExtension: ext)
    }
}
