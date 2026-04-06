package com.pktap.app.nfc

import android.nfc.NfcAdapter
import android.nfc.Tag
import android.nfc.tech.IsoDep
import androidx.activity.ComponentActivity
import java.io.IOException

/**
 * Manages NFC reader mode for the PKTap exchange protocol (D-01, D-02, D-06).
 *
 * Uses [NfcAdapter.enableReaderMode] (NOT enableForegroundDispatch — Pitfall 2).
 * Uses [NfcAdapter.FLAG_READER_SKIP_NDEF_CHECK] to skip NDEF polling (Pitfall 5).
 *
 * The reader role:
 * 1. SELECT AID F0504B544150 (NFC-05)
 * 2. EXCHANGE APDU — send our 36-byte payload, receive peer's 36-byte payload
 * 3. Emit peer payload to [NfcExchangeFlow.peerKeyFlow]
 *
 * [onTagDiscovered] runs on a dedicated background thread provided by the NFC stack —
 * all IsoDep I/O is safe here.
 */
class NfcReader(private val activity: ComponentActivity) : NfcAdapter.ReaderCallback {

    private val nfcAdapter: NfcAdapter? = NfcAdapter.getDefaultAdapter(activity)

    /** True if this device has NFC hardware. */
    fun isNfcAvailable(): Boolean = nfcAdapter != null

    /** True if NFC is available AND currently enabled by the user. */
    fun isNfcEnabled(): Boolean = nfcAdapter?.isEnabled == true

    /**
     * Activate reader mode. Call from [ComponentActivity.onResume].
     *
     * Uses NFC_A flag only — PKTap HCE uses ISO-DEP over NFC-A.
     * FLAG_READER_SKIP_NDEF_CHECK prevents the OS from also doing NDEF discovery,
     * which would interfere with our IsoDep exchange (Pitfall 5).
     */
    fun enableReaderMode() {
        nfcAdapter?.enableReaderMode(
            activity,
            this,
            NfcAdapter.FLAG_READER_NFC_A or NfcAdapter.FLAG_READER_SKIP_NDEF_CHECK,
            null  // No extra options bundle
        )
    }

    /**
     * Deactivate reader mode. Call from [ComponentActivity.onPause].
     */
    fun disableReaderMode() {
        nfcAdapter?.disableReaderMode(activity)
    }

    /**
     * Called by the NFC stack on a background thread when a tag is discovered.
     *
     * Opens IsoDep channel, performs SELECT AID + EXCHANGE, emits peer payload.
     * IOException (tag lost, comm error) is silently swallowed — user retries tap (T-05-09).
     */
    override fun onTagDiscovered(tag: Tag) {
        val isoDep = IsoDep.get(tag) ?: return
        try {
            isoDep.connect()
            performExchange(isoDep)
        } catch (e: IOException) {
            // Tag lost or communication error — user will retry tap (T-05-09 accept)
        } finally {
            try { isoDep.close() } catch (_: IOException) {}
        }
    }

    /**
     * Execute the two-step APDU exchange over an open [IsoDep] channel.
     *
     * Step 1: SELECT AID — authenticates that the peer is running PKTap.
     * Step 2: EXCHANGE — sends our payload, receives peer's payload.
     *
     * T-05-06 mitigation: response size validated (== 38) and SW checked (90 00)
     * before extracting and trusting the peer payload.
     */
    private fun performExchange(isoDep: IsoDep) {
        // Step 1: SELECT AID F0504B544150 (NFC-05)
        val selectResponse = isoDep.transceive(buildSelectAid())
        if (!isSwOk(selectResponse)) return

        // Step 2: EXCHANGE — send our 36-byte payload (D-01)
        val ourPayload = PktapHceService.cachedPayload ?: return
        val exchangeApdu = buildExchangeApdu(ourPayload)
        val response = isoDep.transceive(exchangeApdu)

        // Step 3: Parse response — expect 36 data bytes + SW 90 00 = 38 bytes total (T-05-06)
        if (response.size == 38 && isSwOk(response)) {
            val peerPayload = response.copyOfRange(0, 36)
            NfcExchangeFlow.peerKeyFlow.tryEmit(peerPayload)
        }
    }

    private fun buildSelectAid(): ByteArray {
        val aid = byteArrayOf(0xF0.toByte(), 0x50, 0x4B, 0x54, 0x41, 0x50)
        return byteArrayOf(0x00, 0xA4.toByte(), 0x04, 0x00, aid.size.toByte()) + aid
    }

    private fun buildExchangeApdu(payload: ByteArray): ByteArray {
        // CLA=0x90, INS=0x01, P1=0x00, P2=0x00, LC=0x24 (36 decimal)
        return byteArrayOf(
            PktapHceService.EXCHANGE_CLA,
            PktapHceService.EXCHANGE_INS,
            0x00, 0x00, 0x24
        ) + payload
    }

    private fun isSwOk(response: ByteArray): Boolean =
        response.size >= 2 &&
        response[response.size - 2] == 0x90.toByte() &&
        response[response.size - 1] == 0x00.toByte()
}
