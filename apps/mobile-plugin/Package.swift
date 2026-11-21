// swift-tools-version: 5.9
// MO-3: the mobile app's Rust (crates/escouade-mobile) in Capacitor under iOS. The XCFramework and the Swift bindings
// are built by scripts/mobile-ios.sh (cargo for the device and the simulator, uniffi-bindgen, xcodebuild
// -create-xcframework), never committed.
import PackageDescription

let package = Package(
    name: "EscouadeCapacitorMobile",
    platforms: [.iOS(.v15)],
    products: [.library(name: "EscouadeCapacitorMobile", targets: ["EscouadeMobilePlugin"])],
    dependencies: [.package(url: "https://github.com/ionic-team/capacitor-swift-pm.git", from: "8.0.0")],
    targets: [
        .binaryTarget(name: "escouade_mobileFFI", path: "ios/escouade_mobileFFI.xcframework"),
        .target(name: "EscouadeMobileCore", dependencies: ["escouade_mobileFFI"], path: "ios/Sources/EscouadeMobileCore"),
        .target(
            name: "EscouadeMobilePlugin",
            dependencies: [
                .product(name: "Capacitor", package: "capacitor-swift-pm"),
                .product(name: "Cordova", package: "capacitor-swift-pm"),
                "EscouadeMobileCore",
            ],
            path: "ios/Sources/EscouadeMobilePlugin"
        ),
    ]
)
