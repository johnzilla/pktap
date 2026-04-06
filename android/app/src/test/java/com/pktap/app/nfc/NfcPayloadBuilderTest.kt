package com.pktap.app.nfc

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class NfcPayloadBuilderTest {

    private val validPubKey = ByteArray(32) { it.toByte() }

    @Test
    fun `buildNfcPayload returns exactly 36 bytes`() {
        val payload = NfcPayloadBuilder.buildNfcPayload(validPubKey)
        assertEquals(36, payload.size)
    }

    @Test
    fun `buildNfcPayload byte 0 is version 0x01`() {
        val payload = NfcPayloadBuilder.buildNfcPayload(validPubKey)
        assertEquals(0x01.toByte(), payload[0])
    }

    @Test
    fun `buildNfcPayload byte 1 is flags 0x00`() {
        val payload = NfcPayloadBuilder.buildNfcPayload(validPubKey)
        assertEquals(0x00.toByte(), payload[1])
    }

    @Test
    fun `buildNfcPayload bytes 2 to 33 contain pubkey unchanged`() {
        val payload = NfcPayloadBuilder.buildNfcPayload(validPubKey)
        val extracted = payload.copyOfRange(2, 34)
        assertArrayEquals(validPubKey, extracted)
    }

    @Test
    fun `buildNfcPayload CRC-16 of first 34 bytes matches bytes 34 and 35`() {
        val payload = NfcPayloadBuilder.buildNfcPayload(validPubKey)
        val crc = NfcPayloadBuilder.crc16Ccitt(payload, 0, 34)
        val hi = (crc shr 8).toByte()
        val lo = crc.toByte()
        assertEquals(hi, payload[34])
        assertEquals(lo, payload[35])
    }

    @Test
    fun `crc16Ccitt of CCITT check string returns 0x29B1`() {
        val checkBytes = "123456789".toByteArray(Charsets.US_ASCII)
        val result = NfcPayloadBuilder.crc16Ccitt(checkBytes)
        assertEquals(0x29B1, result)
    }

    @Test(expected = IllegalArgumentException::class)
    fun `buildNfcPayload with wrong length pubkey throws IllegalArgumentException`() {
        NfcPayloadBuilder.buildNfcPayload(ByteArray(16))
    }

    @Test(expected = IllegalArgumentException::class)
    fun `buildNfcPayload with empty pubkey throws IllegalArgumentException`() {
        NfcPayloadBuilder.buildNfcPayload(ByteArray(0))
    }

    @Test
    fun `validateNfcPayload returns true for valid payload`() {
        val payload = NfcPayloadBuilder.buildNfcPayload(validPubKey)
        assertTrue(NfcPayloadBuilder.validateNfcPayload(payload))
    }

    @Test
    fun `validateNfcPayload returns false for corrupted CRC`() {
        val payload = NfcPayloadBuilder.buildNfcPayload(validPubKey)
        payload[34] = (payload[34] + 1).toByte() // corrupt high byte of CRC
        assertFalse(NfcPayloadBuilder.validateNfcPayload(payload))
    }

    @Test
    fun `validateNfcPayload returns false for wrong length input`() {
        assertFalse(NfcPayloadBuilder.validateNfcPayload(ByteArray(35)))
        assertFalse(NfcPayloadBuilder.validateNfcPayload(ByteArray(37)))
        assertFalse(NfcPayloadBuilder.validateNfcPayload(ByteArray(0)))
    }

    @Test
    fun `extractPubKey returns original pubkey for valid payload`() {
        val payload = NfcPayloadBuilder.buildNfcPayload(validPubKey)
        val extracted = NfcPayloadBuilder.extractPubKey(payload)
        assertNotNull(extracted)
        assertArrayEquals(validPubKey, extracted)
    }

    @Test
    fun `extractPubKey returns null for invalid payload`() {
        val corrupted = ByteArray(36)
        assertNull(NfcPayloadBuilder.extractPubKey(corrupted))
    }
}
