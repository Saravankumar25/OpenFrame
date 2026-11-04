// A small evaluator for the colour formulas that appear in themes.json and in theme overrides.
//
// Supported forms: `var(--token)`, `#hex`, `black`, `white`, `transparent`,
// `color-mix(in oklab, X p%, Y)`, `oklch(L% C H)` with `var()` and `calc()` in the channels, and
// `oklch(from X l c h)` relative colours. Anything else fails to parse and the token is reported
// as unresolved.
import Foundation

/// A percentage: a literal or a scalar token such as `var(--mix)`.
enum Pct: Sendable {
    case literal(Double)
    case ref(String)
}

/// A channel number inside `oklch(...)`.
indirect enum Num: Sendable {
    case literal(Double, percent: Bool)
    case scalar(String)
    case keyword(String)
    case op(Character, Num, Num)
}

indirect enum Expr: Sendable {
    case color(OKLab)
    case ref(String)
    case mix(Expr, Pct?, Expr, Pct?)
    case oklch(Num, Num, Num)
    case relative(Expr, Num, Num, Num)
}

enum FormulaParser {
    static func parse(_ raw: String) -> Expr? {
        let s = raw.trimmingCharacters(in: .whitespaces)
        let lower = s.lowercased()
        switch lower {
        case "black": return .color(.black)
        case "white": return .color(OKLab(srgb: 1, 1, 1))
        case "transparent": return .color(.transparent)
        default: break
        }
        if s.hasPrefix("#") { return ColorMath.parseHex(s).map(Expr.color) }
        if let name = varName(s) { return .ref(name) }
        if let inner = functionArgs(s, "color-mix") { return parseMix(inner) }
        if let inner = functionArgs(s, "oklch") { return parseOklch(inner) }
        return nil
    }

    // MARK: pieces

    /// `--name` of a whole `var(--name)` string.
    static func varName(_ s: String) -> String? {
        guard let inner = functionArgs(s, "var"), inner.hasPrefix("--") else { return nil }
        let name = String(inner.dropFirst(2))
        let ok = !name.isEmpty && name.allSatisfy { $0.isLetter || $0.isNumber || $0 == "-" }
        return ok ? name : nil
    }

    /// The text between the parentheses when `s` is exactly one call of `name`.
    static func functionArgs(_ s: String, _ name: String) -> String? {
        let lower = s.lowercased()
        guard lower.hasPrefix(name + "("), s.hasSuffix(")") else { return nil }
        let inner = String(s.dropFirst(name.count + 1).dropLast())
        // The opening bracket must close at the very end.
        var depth = 1
        for ch in inner {
            if ch == "(" { depth += 1 }
            if ch == ")" {
                depth -= 1
                if depth == 0 { return nil }
            }
        }
        return depth == 1 ? inner.trimmingCharacters(in: .whitespaces) : nil
    }

    /// Split at top-level separators (outside brackets).
    static func split(_ s: String, on separator: (Character) -> Bool) -> [String] {
        var parts: [String] = []
        var current = ""
        var depth = 0
        for ch in s {
            if ch == "(" { depth += 1 }
            if ch == ")" { depth -= 1 }
            if depth == 0 && separator(ch) {
                if !current.isEmpty { parts.append(current) }
                current = ""
            } else {
                current.append(ch)
            }
        }
        if !current.isEmpty { parts.append(current) }
        return parts
    }

    static func parseMix(_ inner: String) -> Expr? {
        let parts = split(inner) { $0 == "," }.map { $0.trimmingCharacters(in: .whitespaces) }
        guard parts.count == 3, parts[0].lowercased() == "in oklab" else { return nil }
        guard let first = parseMixItem(parts[1]), let second = parseMixItem(parts[2]) else { return nil }
        return .mix(first.0, first.1, second.0, second.1)
    }

    /// `COLOR` or `COLOR p%` or `COLOR var(--mix)`.
    static func parseMixItem(_ item: String) -> (Expr, Pct?)? {
        let words = split(item) { $0 == " " }
        guard !words.isEmpty else { return nil }
        if words.count == 1 { return parse(words[0]).map { ($0, nil) } }
        guard words.count == 2 else { return nil }
        let last = words[words.count - 1]
        let colourText = words.dropLast().joined(separator: " ")
        guard let colour = parse(colourText) else { return nil }
        if last.hasSuffix("%"), let v = Double(last.dropLast()) { return (colour, .literal(v)) }
        if let name = varName(last) { return (colour, .ref(name)) }
        return nil
    }

    static func parseOklch(_ inner: String) -> Expr? {
        var words = split(inner) { $0 == " " }
        var base: Expr?
        if words.first?.lowercased() == "from" {
            guard words.count >= 2, let b = parse(words[1]) else { return nil }
            base = b
            words.removeFirst(2)
        }
        guard words.count == 3 else { return nil }
        guard let l = parseNum(words[0]), let c = parseNum(words[1]), let h = parseNum(words[2]) else { return nil }
        if let base { return .relative(base, l, c, h) }
        return .oklch(l, c, h)
    }

    static func parseNum(_ raw: String) -> Num? {
        let s = raw.trimmingCharacters(in: .whitespaces)
        if let inner = functionArgs(s, "calc") { return parseCalc(inner) }
        return parseOperand(s)
    }

    static func parseOperand(_ s: String) -> Num? {
        if s == "l" || s == "c" || s == "h" { return .keyword(s) }
        if let name = varName(s) { return .scalar(name) }
        if let inner = functionArgs(s, "calc") { return parseCalc(inner) }
        if s.hasSuffix("%"), let v = Double(s.dropLast()) { return .literal(v, percent: true) }
        if let v = Double(s) { return .literal(v, percent: false) }
        return nil
    }

    /// `a + b`, `a - b`, `a * b`, `a / b` with the usual precedence and no unary minus.
    static func parseCalc(_ inner: String) -> Num? {
        let words = split(inner) { $0 == " " }
        guard words.count % 2 == 1 else { return nil }
        var operands: [Num] = []
        var ops: [Character] = []
        for (i, w) in words.enumerated() {
            if i % 2 == 0 {
                guard let n = parseOperand(w) else { return nil }
                operands.append(n)
            } else {
                guard w.count == 1, let ch = w.first, "+-*/".contains(ch) else { return nil }
                ops.append(ch)
            }
        }
        // First fold * and /, then + and -.
        var terms: [Num] = [operands[0]]
        var addOps: [Character] = []
        for (i, op) in ops.enumerated() {
            let rhs = operands[i + 1]
            if op == "*" || op == "/" {
                terms[terms.count - 1] = .op(op, terms[terms.count - 1], rhs)
            } else {
                addOps.append(op)
                terms.append(rhs)
            }
        }
        var result = terms[0]
        for (i, op) in addOps.enumerated() { result = .op(op, result, terms[i + 1]) }
        return result
    }
}

/// Evaluates the formulas of one theme in one mode. Token values are lazy, so an override of
/// `accent` also changes every token defined from `var(--accent)`, exactly as in the CSS cascade.
final class FormulaResolver {
    var exprs: [String: Expr] = [:]
    var scalars: [String: (value: Double, percent: Bool)] = [:]
    private var memo: [String: OKLab] = [:]
    private var visiting: Set<String> = []
    private(set) var failed: Set<String> = []
    /// Used for a token that cannot be evaluated; set by the engine to a neutral token.
    var fallback = OKLab(l: 0.5, a: 0, b: 0)

    func value(of token: String) -> OKLab {
        if let hit = memo[token] { return hit }
        guard let expr = exprs[token], !visiting.contains(token) else {
            failed.insert(token)
            return fallback
        }
        visiting.insert(token)
        defer { visiting.remove(token) }
        let out: OKLab
        if let v = eval(expr) {
            out = v
        } else {
            failed.insert(token)
            out = fallback
        }
        memo[token] = out
        return out
    }

    func eval(_ expr: Expr) -> OKLab? {
        switch expr {
        case .color(let c): return c
        case .ref(let name):
            guard exprs[name] != nil else { return nil }
            let v = value(of: name)
            return failed.contains(name) ? nil : v
        case .mix(let a, let pa, let b, let pb):
            guard let ca = eval(a), let cb = eval(b) else { return nil }
            let wa = pa.flatMap(pct), wb = pb.flatMap(pct)
            if (pa != nil && wa == nil) || (pb != nil && wb == nil) { return nil }
            let (x, y): (Double, Double)
            switch (wa, wb) {
            case (nil, nil): (x, y) = (50, 50)
            case (let p?, nil): (x, y) = (p, 100 - p)
            case (nil, let q?): (x, y) = (100 - q, q)
            case (let p?, let q?): (x, y) = (p, q)
            }
            return ColorMath.mix(ca, x, cb, y)
        case .oklch(let l, let c, let h):
            guard let lv = number(l, lightness: true, base: nil), let cv = number(c, lightness: false, base: nil),
                let hv = number(h, lightness: false, base: nil)
            else { return nil }
            return OKLab(oklchL: lv, c: cv, h: hv)
        case .relative(let baseExpr, let l, let c, let h):
            guard let base = eval(baseExpr) else { return nil }
            let lch = base.lch
            guard let lv = number(l, lightness: true, base: lch), let cv = number(c, lightness: false, base: lch),
                let hv = number(h, lightness: false, base: lch)
            else { return nil }
            return OKLab(oklchL: lv, c: cv, h: hv, alpha: base.alpha)
        }
    }

    private func pct(_ p: Pct) -> Double? {
        switch p {
        case .literal(let v): return v
        case .ref(let name): return scalars[name]?.value
        }
    }

    /// A channel value. Lightness written as a percentage becomes 0 to 1.
    private func number(_ n: Num, lightness: Bool, base: (l: Double, c: Double, h: Double)?) -> Double? {
        guard let r = raw(n, base: base) else { return nil }
        return lightness && r.percent ? r.value / 100 : r.value
    }

    private func raw(_ n: Num, base: (l: Double, c: Double, h: Double)?) -> (value: Double, percent: Bool)? {
        switch n {
        case .literal(let v, let p): return (v, p)
        case .scalar(let name): return scalars[name].map { ($0.value, $0.percent) }
        case .keyword(let k):
            guard let base else { return nil }
            switch k {
            case "l": return (base.l, false)
            case "c": return (base.c, false)
            default: return (base.h, false)
            }
        case .op(let op, let a, let b):
            guard let x = raw(a, base: base), let y = raw(b, base: base) else { return nil }
            let pc = x.percent || y.percent
            switch op {
            case "+": return (x.value + y.value, pc)
            case "-": return (x.value - y.value, pc)
            case "*": return (x.value * y.value, pc)
            default: return y.value == 0 ? nil : (x.value / y.value, pc)
            }
        }
    }
}
