package com.pktap.bridge

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Instrumented tests proving the Rust-to-Kotlin FFI pipeline works.
 * Must run on Android emulator/device — local JVM cannot load .so files.
 */
@RunWith(AndroidJUnit4::class)
class PktapBridgeInstrumentedTest {

    /**
     * FFI-02: Hello-world pipeline test.
     * Calls pktap_ping() in Rust via UniFFI and verifies the return value.
     * This proves: cargo-ndk compiled the .so, uniffi-bindgen generated the bindings,
     * JNA loaded the native library, and the function call round-tripped successfully.
     */
    @Test
    fun pktapPingReturnsPktapOk() {
        val result = PktapBridge.ping()
        assertEquals("pktap-ok", result)
    }

    /**
     * FFI-03 / D-06: Verify that seedBytes is zeroed after ecdhAndEncrypt call.
     * This is a code-level verification that PktapBridge.kt fulfills the zeroing contract.
     *
     * Note: We pass intentionally invalid inputs — the test verifies zeroing behavior,
     * not encryption correctness (that's tested in Rust unit tests).
     *
     * DEPENDENCY: This test expects invalid key inputs to throw a PktapException from the Rust
     * side (CRYPTO-01, D-07). If Rust panics instead of returning PktapError, the zeroing
     * in the finally block still executes, but the test's catch block behavior may differ.
     */
    @Test
    fun ecdhAndEncryptZeroesSeedBytes() {
        val seedBytes = ByteArray(32) { 0x42 }  // Non-zero seed
        val peerKey = ByteArray(32) { 0x01 }     // Dummy peer key

        try {
            PktapBridge.ecdhAndEncrypt(seedBytes, peerKey, "{}")
        } catch (_: Exception) {
            // We expect this may throw (invalid key) — that's fine.
            // The important thing is that seedBytes is zeroed regardless.
        }

        // D-06: seedBytes must be zeroed after the call (even if it threw)
        assertTrue(
            "seedBytes must be all zeros after ecdhAndEncrypt (D-06)",
            seedBytes.all { it == 0.toByte() }
        )
    }

    /**
     * FFI-03 / D-06: Verify that seedBytes is zeroed after decryptAndVerify call.
     *
     * DEPENDENCY: Same as ecdhAndEncryptZeroesSeedBytes — depends on Phase 1's
     * PktapException handling (CRYPTO-01, D-07) for invalid inputs.
     */
    @Test
    fun decryptAndVerifyZeroesSeedBytes() {
        val seedBytes = ByteArray(32) { 0x42 }
        val peerKey = ByteArray(32) { 0x01 }
        val signature = ByteArray(64) { 0x00 }
        val record = ByteArray(50) { 0x01 }

        try {
            PktapBridge.decryptAndVerify(seedBytes, peerKey, signature, record)
        } catch (_: Exception) {
            // Expected to throw — we're testing zeroing, not decryption
        }

        assertTrue(
            "seedBytes must be all zeros after decryptAndVerify (D-06)",
            seedBytes.all { it == 0.toByte() }
        )
    }
}
