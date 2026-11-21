# UniFFI's bindings reach the library through JNA, by reflection: R8 must keep them (MO-1 turns R8 on).
-keep class com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.** { public *; }
-keep class uniffi.escouade_mobile.** { *; }
-dontwarn java.awt.**
