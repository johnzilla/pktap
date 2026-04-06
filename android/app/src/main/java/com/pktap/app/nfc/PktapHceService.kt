package com.pktap.app.nfc

import android.nfc.cardemulation.HostApduService
import android.os.Bundle

/**
 * HCE HostApduService for PKTap NFC key exchange.
 *
 * Security / reliability contracts (D-01, D-03, NFC-01, NFC-02, NFC-03):
 * - cachedPayload is pre-built at app start from AppViewModel.publicKeyBytes — zero computation here
 * - processCommandApdu has a blanket try/catch: an unhandled exception permanently kills HCE (Pitfall 1)
 * - NO FFI bridge calls, no crypto, no I/O in processCommandApdu (NFC-03)
 * - Peer payload delivered via NfcExchangeFlow.peerKeyFlow SharedFlow (D-04)
 */
class PktapHceService : HostApduService() {

    companion object {
        /** Pre-built 36-byte NFC payload set by AppViewModel at startup (D-03). */
        @Volatile var cachedPayload: ByteArray? = null

        val SW_OK = byteArrayOf(0x90.toByte(), 0x00)
        val SW_UNKNOWN = byteArrayOf(0x6F.toByte(), 0x00)
        val SW_FILE_NOT_FOUND = byteArrayOf(0x6A.toByte(), 0x82.toByte())

        /** EXCHANGE APDU class byte — used by NfcReader to build the exchange command. */
        const val EXCHANGE_CLA: Byte = 0x90.toByte()

        /** EXCHANGE APDU instruction byte — used by NfcReader to build the exchange command. */
        const val EXCHANGE_INS: Byte = 0x01
    }

    private var selectAidReceived = false

    /**
     * Handle incoming APDU commands.
     *
     * CRITICAL: Wrapped in blanket try/catch. Any uncaught exception here permanently kills
     * the HCE service until device reboot — return SW_UNKNOWN on any failure.
     */
    override fun processCommandApdu(commandApdu: ByteArray, extras: Bundle?): ByteArray {
        return try {
            val result = PktapApduProtocol.handleApdu(commandApdu, cachedPayload, selectAidReceived)
            selectAidReceived = result.newSelectAidState
            result.peerPayload?.let { NfcExchangeFlow.peerKeyFlow.tryEmit(it) }
            result.responseBytes
        } catch (e: Exception) {
            SW_UNKNOWN
        }
    }

    override fun onDeactivated(reason: Int) {
        selectAidReceived = false
    }
}

/**
 * Pure APDU protocol logic, extracted for JVM unit testability.
 *
 * All state is passed in; result carries new state. No Android framework dependencies.
 */
object PktapApduProtocol {

    // AID: F0504B544150 = hex "PKTAP" (6 bytes, proprietary category — D-05)
    private val AID = byteArrayOf(
        0xF0.toByte(), 0x50, 0x4B, 0x54, 0x41, 0x50
    )

    private const val CLA_SELECT: Byte = 0x00
    private const val INS_SELECT: Byte = 0xA4.toByte()
    private const val CLA_EXCHANGE: Byte = 0x90.toByte()
    private const val INS_EXCHANGE: Byte = 0x01

    /** Minimum valid EXCHANGE APDU length: 5 header bytes + 36 data bytes */
    private const val EXCHANGE_APDU_MIN_LENGTH = 41

    /**
     * Handle one APDU command and return a result containing the response bytes,
     * any received peer payload, and the new SELECT AID state.
     *
     * All logic is pure — no side effects. The caller (PktapHceService) applies state
     * changes and emits to SharedFlow.
     */
    fun handleApdu(
        commandApdu: ByteArray,
        cachedPayload: ByteArray?,
        selectAidReceived: Boolean
    ): ApduResult {
        if (commandApdu.size < 4) {
            return ApduResult(PktapHceService.SW_UNKNOWN, null, selectAidReceived)
        }

        val cla = commandApdu[0]
        val ins = commandApdu[1]

        // SELECT AID: CLA=0x00, INS=0xA4 (ISO 7816-4)
        if (cla == CLA_SELECT && ins == INS_SELECT) {
            return ApduResult(PktapHceService.SW_OK, null, newSelectAidState = true)
        }

        // EXCHANGE command: CLA=0x90, INS=0x01 — proprietary PKTap command
        if (cla == CLA_EXCHANGE && ins == INS_EXCHANGE) {
            if (!selectAidReceived) {
                return ApduResult(PktapHceService.SW_UNKNOWN, null, selectAidReceived)
            }
            if (cachedPayload == null) {
                return ApduResult(PktapHceService.SW_FILE_NOT_FOUND, null, selectAidReceived)
            }
            // Validate APDU length: must have at least 5 header bytes + 36 data bytes
            if (commandApdu.size < EXCHANGE_APDU_MIN_LENGTH) {
                return ApduResult(PktapHceService.SW_UNKNOWN, null, selectAidReceived)
            }
            // Extract peer's 36-byte payload from data field (offset 5..41)
            val peerPayload = commandApdu.copyOfRange(5, 41)
            val response = cachedPayload + PktapHceService.SW_OK
            return ApduResult(response, peerPayload, selectAidReceived)
        }

        return ApduResult(PktapHceService.SW_UNKNOWN, null, selectAidReceived)
    }
}

/**
 * Result of an APDU protocol exchange.
 *
 * @param responseBytes  Bytes to return from processCommandApdu
 * @param peerPayload    36-byte peer NFC payload to emit, or null if not an EXCHANGE command
 * @param newSelectAidState  Updated SELECT AID received flag
 */
data class ApduResult(
    val responseBytes: ByteArray,
    val peerPayload: ByteArray?,
    val newSelectAidState: Boolean
) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is ApduResult) return false
        return responseBytes.contentEquals(other.responseBytes) &&
            (peerPayload?.contentEquals(other.peerPayload ?: return false) ?: (other.peerPayload == null)) &&
            newSelectAidState == other.newSelectAidState
    }

    override fun hashCode(): Int {
        var result = responseBytes.contentHashCode()
        result = 31 * result + (peerPayload?.contentHashCode() ?: 0)
        result = 31 * result + newSelectAidState.hashCode()
        return result
    }
}
