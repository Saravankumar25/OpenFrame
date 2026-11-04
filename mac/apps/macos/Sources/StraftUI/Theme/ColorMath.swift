// Colour maths for the theme engine (CSS Color 4 formulas): OKLCH, OKLab, linear sRGB, gamma sRGB.
// Colours are held in OKLab without clamping, as browsers do while mixing; the clamp to the sRGB
// gamut happens only when a colour is turned into display values.
import Foundation

/// A colour in OKLab with alpha. Components are not clamped.
struct OKLab: Sendable, Equatable {
    var l: Double
    var a: Double
    var b: Double
    var alpha: Double = 1

    static let black = OKLab(l: 0, a: 0, b: 0)
    /// `transparent` is rgba(0 0 0 / 0).
    static let transparent = OKLab(l: 0, a: 0, b: 0, alpha: 0)

    /// From OKLCH: lightness 0 to 1, chroma, hue in degrees.
    init(oklchL l: Double, c: Double, h: Double, alpha: Double = 1) {
        let rad = h * .pi / 180
        self.init(l: l, a: c * cos(rad), b: c * sin(rad), alpha: alpha)
    }

    init(l: Double, a: Double, b: Double, alpha: Double = 1) {
        self.l = l
        self.a = a
        self.b = b
        self.alpha = alpha
    }

    /// From gamma-encoded sRGB components in 0 to 1.
    init(srgb r: Double, _ g: Double, _ b: Double, alpha: Double = 1) {
        let lr = ColorMath.toLinear(r), lg = ColorMath.toLinear(g), lb = ColorMath.toLinear(b)
        let l = cbrt(0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb)
        let m = cbrt(0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb)
        let s = cbrt(0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb)
        self.init(
            l: 0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
            a: 1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
            b: 0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
            alpha: alpha)
    }

    /// Linear-light sRGB, not clamped.
    var linearRGB: (r: Double, g: Double, b: Double) {
        let l_ = l + 0.3963377774 * a + 0.2158037573 * b
        let m_ = l - 0.1055613458 * a - 0.0638541728 * b
        let s_ = l - 0.0894841775 * a - 1.2914855480 * b
        let l3 = l_ * l_ * l_, m3 = m_ * m_ * m_, s3 = s_ * s_ * s_
        return (
            4.0767416621 * l3 - 3.3077115913 * m3 + 0.2309699292 * s3,
            -1.2684380046 * l3 + 2.6097574011 * m3 - 0.3413193965 * s3,
            -0.0041960863 * l3 - 0.7034186147 * m3 + 1.7076147010 * s3
        )
    }

    /// Gamma-encoded sRGB clamped into the gamut, components 0 to 1.
    var srgb: (r: Double, g: Double, b: Double) {
        let lin = linearRGB
        func enc(_ v: Double) -> Double { min(1, max(0, ColorMath.fromLinear(min(1, max(0, v))))) }
        return (enc(lin.r), enc(lin.g), enc(lin.b))
    }

    /// OKLCH of this colour (lightness 0 to 1, chroma, hue 0 to 360).
    var lch: (l: Double, c: Double, h: Double) {
        let c = (a * a + b * b).squareRoot()
        var h = atan2(b, a) * 180 / .pi
        if h < 0 { h += 360 }
        return (l, c, c < 1e-6 ? 0 : h)
    }
}

enum ColorMath {
    static func toLinear(_ v: Double) -> Double {
        let s = v < 0 ? -1.0 : 1.0
        let x = abs(v)
        return s * (x <= 0.04045 ? x / 12.92 : pow((x + 0.055) / 1.055, 2.4))
    }

    static func fromLinear(_ v: Double) -> Double {
        let s = v < 0 ? -1.0 : 1.0
        let x = abs(v)
        return s * (x <= 0.0031308 ? x * 12.92 : 1.055 * pow(x, 1 / 2.4) - 0.055)
    }

    /// `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa`.
    static func parseHex(_ text: String) -> OKLab? {
        guard text.hasPrefix("#") else { return nil }
        var digits = Array(text.dropFirst())
        guard digits.allSatisfy({ $0.isHexDigit }) else { return nil }
        if digits.count == 3 || digits.count == 4 {
            digits = digits.flatMap { [$0, $0] }
        }
        guard digits.count == 6 || digits.count == 8 else { return nil }
        func byte(_ i: Int) -> Double {
            Double(UInt8(String(digits[i * 2...i * 2 + 1]), radix: 16) ?? 0) / 255
        }
        let alpha = digits.count == 8 ? byte(3) : 1
        return OKLab(srgb: byte(0), byte(1), byte(2), alpha: alpha)
    }

    /// `color-mix(in oklab, a p1, b p2)` with premultiplied alpha. Percentages are normalised to 100.
    static func mix(_ a: OKLab, _ pa: Double, _ b: OKLab, _ pb: Double) -> OKLab {
        let total = pa + pb
        guard total > 0 else { return a }
        let wa = pa / total, wb = pb / total
        let alpha = wa * a.alpha + wb * b.alpha
        guard alpha > 0 else { return .transparent }
        func channel(_ x: Double, _ y: Double) -> Double {
            (wa * a.alpha * x + wb * b.alpha * y) / alpha
        }
        return OKLab(
            l: channel(a.l, b.l), a: channel(a.a, b.a), b: channel(a.b, b.b), alpha: alpha)
    }

    /// WCAG relative luminance of an opaque colour.
    static func luminance(_ c: OKLab) -> Double {
        let rgb = OKLab(l: c.l, a: c.a, b: c.b).srgb
        let r = toLinear(rgb.r), g = toLinear(rgb.g), b = toLinear(rgb.b)
        return 0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    /// WCAG contrast ratio between two colours.
    static func contrast(_ x: OKLab, _ y: OKLab) -> Double {
        let lx = luminance(x), ly = luminance(y)
        return (max(lx, ly) + 0.05) / (min(lx, ly) + 0.05)
    }
}
