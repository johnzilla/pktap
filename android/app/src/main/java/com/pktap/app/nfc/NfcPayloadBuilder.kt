package com.pktap.app.nfc

/**
 * Builds and validates the 36-byte NFC wire format for PKTap key exchange.
 *
 * Wire format (NFC-04):
 *   [0]      version  = 0x01
 *   [1]      flags    = 0x00 (encrypted mode)
 *   [2..33]  Ed25519 public key (32 bytes)
 *   [34..35] CRC-16/CCITT-FALSE of bytes [0..33]
 *
 * CRC-16/CCITT-FALSE: poly=0x1021, init=0xFFFF, no reflection, no XOR out.
 * Check value: crc16Ccitt("123456789") == 0x29B1
 */
object NfcPayloadBuilder {

    const val VERSION: Byte = 0x01
    const val FLAGS_ENCRYPTED: Byte = 0x00
    const val PAYLOAD_SIZE = 36
    const val PUBKEY_SIZE = 32

    /**
     * Build the 36-byte NFC payload from a 32-byte Ed25519 public key.
     *
     * @throws IllegalArgumentException if pubKeyBytes is not exactly 32 bytes
     */
    fun buildNfcPayload(pubKeyBytes: ByteArray): ByteArray {
        require(pubKeyBytes.size == PUBKEY_SIZE) {
            "Ed25519 pubkey must be exactly $PUBKEY_SIZE bytes, got ${pubKeyBytes.size}"
        }
        val buf = ByteArray(PAYLOAD_SIZE)
        buf[0] = VERSION
        buf[1] = FLAGS_ENCRYPTED
        pubKeyBytes.copyInto(buf, destinationOffset = 2)
        val crc = crc16Ccitt(buf, offset = 0, length = 34)
        buf[34] = (crc shr 8).toByte()
        buf[35] = crc.toByte()
        return buf
    }

    /**
     * Validate a 36-byte NFC payload by verifying its CRC-16.
     *
     * @return true if payload is exactly 36 bytes and CRC matches
     */
    fun validateNfcPayload(payload: ByteArray): Boolean {
        if (payload.size != PAYLOAD_SIZE) return false
        val expected = crc16Ccitt(payload, offset = 0, length = 34)
        val actual = ((payload[34].toInt() and 0xFF) shl 8) or (payload[35].toInt() and 0xFF)
        return expected == actual
    }

    /**
     * Extract the 32-byte public key from a validated payload.
     *
     * @return the public key bytes, or null if the payload is invalid
     */
    fun extractPubKey(payload: ByteArray): ByteArray? {
        if (!validateNfcPayload(payload)) return null
        return payload.copyOfRange(2, 34)
    }

    /**
     * CRC-16/CCITT-FALSE checksum.
     *
     * Parameters: poly=0x1021, init=0xFFFF, refIn=false, refOut=false, xorOut=0x0000
     * Check value: crc16Ccitt("123456789".toByteArray()) == 0x29B1
     */
    fun crc16Ccitt(data: ByteArray, offset: Int = 0, length: Int = data.size): Int {
        var crc = 0xFFFF
        for (i in offset until offset + length) {
            crc = crc xor ((data[i].toInt() and 0xFF) shl 8)
            repeat(8) {
                crc = if (crc and 0x8000 != 0) (crc shl 1) xor 0x1021 else crc shl 1
            }
            crc = crc and 0xFFFF
        }
        return crc
    }
}
