// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "QuickBuds",
    platforms: [.macOS(.v14)],
    products: [.executable(name: "QuickBuds", targets: ["QuickBuds"])],
    targets: [
        .target(name: "BudsProtocol"),
        .executableTarget(name: "QuickBuds", dependencies: ["BudsProtocol"],
                          resources: [.copy("Resources")],
                          linkerSettings: [.linkedFramework("IOBluetooth")]),
        .testTarget(name: "BudsProtocolTests", dependencies: ["BudsProtocol"])
    ]
)
