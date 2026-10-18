// Small pictures of image items. The core hands back the local file of a managed asset; the
// picture is read and reduced off the main thread and kept in a memory cache, so a long grid
// scrolls without reading full-size images.

import AppKit
import ImageIO
import OpenFrameCore
import SwiftUI

private struct SendableImage: @unchecked Sendable {
    let image: CGImage
}

@MainActor
final class VaultThumbnails {
    static let shared = VaultThumbnails()
    private let cache = NSCache<NSString, NSImage>()

    private init() {
        cache.countLimit = 400
    }

    func image(core: Core, assetId: String, global: Bool, maxPixel: Int) async -> NSImage? {
        let key = "\(global ? "g" : "p"):\(assetId):\(maxPixel)" as NSString
        if let hit = cache.object(forKey: key) { return hit }
        guard let url = try? await core.assetFile(assetId, global: global) else { return nil }
        let decoded = await Task.detached(priority: .utility) { () -> SendableImage? in
            guard let source = CGImageSourceCreateWithURL(url as CFURL, nil) else { return nil }
            let options: [CFString: Any] = [
                kCGImageSourceCreateThumbnailFromImageAlways: true,
                kCGImageSourceCreateThumbnailWithTransform: true,
                kCGImageSourceShouldCacheImmediately: true,
                kCGImageSourceThumbnailMaxPixelSize: maxPixel,
            ]
            return CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary).map(SendableImage.init)
        }.value
        guard let decoded else { return nil }
        let image = NSImage(cgImage: decoded.image, size: NSSize(width: decoded.image.width, height: decoded.image.height))
        cache.setObject(image, forKey: key)
        return image
    }
}

/// The picture of an image item, with a plain stand-in while it loads or when it cannot be read.
struct VaultThumbnailView: View {
    let item: VaultItemDto
    let store: StoreSel
    var height: CGFloat
    var maxPixel: Int = 480
    var fill: Bool = true

    @Environment(AppModel.self) private var app
    @Environment(\.ofTheme) private var theme
    @State private var image: NSImage?
    @State private var failed = false

    var body: some View {
        ZStack {
            theme["surface2"]
            if let image {
                Image(nsImage: image)
                    .resizable()
                    .aspectRatio(contentMode: fill ? .fill : .fit)
            } else {
                Image(systemName: failed ? "photo.badge.exclamationmark" : VaultText.symbol(item.itemType))
                    .font(.title2)
                    .foregroundStyle(theme["muted"])
            }
        }
        .frame(maxWidth: .infinity)
        .frame(height: height)
        .clipped()
        .accessibilityHidden(true)
        .task(id: item.asset?.id) {
            guard let asset = item.asset, asset.available, let core = app.core else { return }
            image = nil
            failed = false
            let loaded = await VaultThumbnails.shared.image(core: core, assetId: asset.id, global: store == .global, maxPixel: maxPixel)
            if let loaded { image = loaded } else { failed = true }
        }
    }
}
