// swift-tools-version: 5.9
// ESC-445 : le Rust commun (mobile-study/shared/core) dans Capacitor sous iOS. Le XCFramework et les liaisons Swift
// sont produits par build-ios.sh (cargo, uniffi-bindgen, xcodebuild -create-xcframework).
import PackageDescription

let package = Package(
    name: "EscouadeRustCore",
    platforms: [.iOS(.v15)],
    products: [.library(name: "EscouadeRustCore", targets: ["RustCorePlugin"])],
    dependencies: [.package(url: "https://github.com/ionic-team/capacitor-swift-pm.git", from: "8.0.0")],
    targets: [
        .binaryTarget(name: "escouade_mobile_coreFFI", path: "ios/escouade_mobile_coreFFI.xcframework"),
        .target(name: "EscouadeMobileCore", dependencies: ["escouade_mobile_coreFFI"], path: "ios/Sources/EscouadeMobileCore"),
        .target(
            name: "RustCorePlugin",
            dependencies: [
                .product(name: "Capacitor", package: "capacitor-swift-pm"),
                .product(name: "Cordova", package: "capacitor-swift-pm"),
                "EscouadeMobileCore",
            ],
            path: "ios/Sources/RustCorePlugin"
        ),
    ]
)
