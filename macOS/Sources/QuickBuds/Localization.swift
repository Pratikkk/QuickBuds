import Foundation
import BudsProtocol

private enum Resources {
    static let strings: [String: [String: String]] = load("strings") ?? [:]
    static func load<T: Decodable>(_ name: String) -> T? {
        let packaged = Bundle.main.resourceURL?.appendingPathComponent("QuickBuds_QuickBuds.bundle")
        let bundle = packaged.flatMap { Bundle(url: $0) } ?? Bundle.module
        guard let url = bundle.url(forResource: name, withExtension: "json", subdirectory: "Resources"),
              let data = try? Data(contentsOf: url) else { return nil }
        return try? JSONDecoder().decode(T.self, from: data)
    }
}

func text(_ key: String) -> String {
    let available = Array(Resources.strings.keys)
    let locale = Bundle.preferredLocalizations(from: available).first ?? "en"
    return Resources.strings[locale]?[key] ?? Resources.strings["en"]?[key] ?? key
}

func featureError(_ key: String) -> String {
    text(key).replacingOccurrences(of: "%1$s", with: text("anc_section"))
}

struct Catalog: Decodable {
    let whiteList: [EarbudModel]
    static let models: [EarbudModel] = Resources.load("models").map { (catalog: Catalog) in catalog.whiteList } ?? []
}
