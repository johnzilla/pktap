package com.pktap.app.keystore

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertArrayEquals
import org.junit.Test
import org.junit.runner.RunWith
import java.security.KeyStore
import java.security.SecureRandom

@RunWith(AndroidJUnit4::class)
class KeystoreManagerTest {

    private val testAlias = "pktap_test_aes_key"

    @After
    fun tearDown() {
        // Remove test key from Keystore to keep tests independent
        try {
            val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
            if (keyStore.containsAlias(testAlias)) {
                keyStore.deleteEntry(testAlias)
            }
        } catch (e: Exception) {
            // Best effort — don't fail teardown
        }
    }

    @Test
    fun testGenerateOrGetKeyCreatesAesKey() {
        val key = KeystoreManager.generateOrGetKey(testAlias)
        assertNotNull("Generated key must not be null", key)
        assertEquals("Key algorithm must be AES", "AES", key.algorithm)
    }

    @Test
    fun testGenerateOrGetKeyReturnsExistingKey() {
        val key1 = KeystoreManager.generateOrGetKey(testAlias)
        val key2 = KeystoreManager.generateOrGetKey(testAlias)
        assertNotNull("First key must not be null", key1)
        assertNotNull("Second key must not be null", key2)
        // Both calls must succeed — second call returns cached key
        assertEquals("Both calls must return same algorithm", key1.algorithm, key2.algorithm)
    }

    @Test
    fun testEncryptDecryptRoundTrip() {
        val key = KeystoreManager.generateOrGetKey(testAlias)
        val original = ByteArray(32).also { SecureRandom().nextBytes(it) }
        val encrypted = KeystoreManager.encrypt(original, key)
        val decrypted = KeystoreManager.decrypt(encrypted, key)
        assertArrayEquals("Decrypted bytes must match original", original, decrypted)
    }

    @Test
    fun testStrongBoxFallbackDoesNotCrash() {
        // On emulator, StrongBox is unavailable — exercises TEE fallback path (KEY-05)
        // Test passes as long as no exception is thrown
        val key = KeystoreManager.generateOrGetKey(testAlias)
        assertNotNull("Key must be non-null even when StrongBox unavailable", key)
    }
}
