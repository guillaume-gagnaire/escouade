package dev.escouade.study.capacitor

import com.getcapacitor.JSObject
import com.getcapacitor.Plugin
import com.getcapacitor.PluginCall
import com.getcapacitor.PluginMethod
import com.getcapacitor.annotation.CapacitorPlugin
import uniffi.escouade_mobile_core.benchRoundtrips
import uniffi.escouade_mobile_core.coreVersion
import uniffi.escouade_mobile_core.open as coreOpen
import uniffi.escouade_mobile_core.seal as coreSeal

/**
 * ESC-445 : le pont de Capacitor vers le Rust commun (mobile-study/shared/core), par les liaisons Kotlin d'UniFFI
 * (générées par build-android.sh dans app/src/main/java/uniffi, chargées par JNA).
 */
@CapacitorPlugin(name = "RustCore")
class RustCorePlugin : Plugin() {
    private fun ok(call: PluginCall, value: Any) = call.resolve(JSObject().put("value", value))

    @PluginMethod
    fun version(call: PluginCall) = ok(call, coreVersion())

    @PluginMethod
    fun open(call: PluginCall) {
        try {
            ok(call, coreOpen(call.getString("key") ?: "", call.getString("sealed") ?: ""))
        } catch (e: Exception) {
            call.reject(e.message ?: e.toString())
        }
    }

    @PluginMethod
    fun seal(call: PluginCall) {
        try {
            ok(call, coreSeal(call.getString("key") ?: "", call.getString("plain") ?: ""))
        } catch (e: Exception) {
            call.reject(e.message ?: e.toString())
        }
    }

    @PluginMethod
    fun benchInRust(call: PluginCall) =
        ok(call, benchRoundtrips((call.getInt("n") ?: 1000).toUInt(), (call.getInt("size") ?: 256).toUInt()).toLong())
}
