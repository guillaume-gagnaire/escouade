import Capacitor
import EscouadeMobileCore
import Foundation

/// ESC-445 : le pont de Capacitor vers le Rust commun sous iOS, par les liaisons Swift d'UniFFI.
@objc(RustCorePlugin)
public class RustCorePlugin: CAPPlugin, CAPBridgedPlugin {
    public let identifier = "RustCorePlugin"
    public let jsName = "RustCore"
    public let pluginMethods: [CAPPluginMethod] = [
        CAPPluginMethod(name: "version", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "open", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "seal", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "benchInRust", returnType: CAPPluginReturnPromise),
    ]

    @objc func version(_ call: CAPPluginCall) {
        call.resolve(["value": EscouadeMobileCore.coreVersion()])
    }

    @objc func open(_ call: CAPPluginCall) {
        do {
            call.resolve(["value": try EscouadeMobileCore.open(keyB64: call.getString("key") ?? "", sealedB64: call.getString("sealed") ?? "")])
        } catch {
            call.reject("\(error)")
        }
    }

    @objc func seal(_ call: CAPPluginCall) {
        do {
            call.resolve(["value": try EscouadeMobileCore.seal(keyB64: call.getString("key") ?? "", plaintext: call.getString("plain") ?? "")])
        } catch {
            call.reject("\(error)")
        }
    }

    @objc func benchInRust(_ call: CAPPluginCall) {
        let n = UInt32(call.getInt("n") ?? 1000)
        let size = UInt32(call.getInt("size") ?? 256)
        call.resolve(["value": Double(EscouadeMobileCore.benchRoundtrips(n: n, size: size))])
    }
}
