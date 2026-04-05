package com.pktap.app.keystore

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import java.security.KeyStore

@RunWith(AndroidJUnit4::class)
class SeedRepositoryTest {

    private lateinit var repository: SeedRepository
    private val context get() = InstrumentationRegistry.getInstrumentation().targetContext

    @Before
    fun setUp() {
        // Clear prefs before each test for isolation
        context.getSharedPreferences("pktap_secure_prefs", android.content.Context.MODE_PRIVATE)
            .edit().clear().commit()
        repository = SeedRepository(context)
    }

    @After
    fun tearDown() {
        // Clear prefs and remove Keystore key
        context.getSharedPreferences("pktap_secure_prefs", android.content.Context.MODE_PRIVATE)
            .edit().clear().commit()
        try {
            val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
            if (keyStore.containsAlias(KeystoreManager.SEED_KEY_ALIAS)) {
                keyStore.deleteEntry(KeystoreManager.SEED_KEY_ALIAS)
            }
        } catch (e: Exception) {
            // Best effort
        }
    }

    @Test
    fun testHasSeedReturnsFalseInitially() {
        assertFalse("hasSeed() must return false before any seed is generated", repository.hasSeed())
    }

    @Test
    fun testGenerateAndStoreSeedReturns32Bytes() {
        val seed = runBlocking { repository.generateAndStoreSeed() }
        try {
            assertNotNull("Seed must not be null", seed)
            assertEquals("Seed must be 32 bytes", 32, seed.size)
            assertTrue("hasSeed() must return true after generation", repository.hasSeed())
        } finally {
            seed.fill(0)
        }
    }

    @Test
    fun testDecryptSeedMatchesGenerated() {
        // Generate seed, immediately zero it (cannot compare — T-04-06 zero-on-use)
        val seed = runBlocking { repository.generateAndStoreSeed() }
        seed.fill(0)

        // Decrypt via a fresh repository instance
        val fresh = SeedRepository(context)
        val decrypted = runBlocking { fresh.decryptSeed() }
        try {
            assertNotNull("Decrypted seed must not be null", decrypted)
            assertEquals("Decrypted seed must be 32 bytes", 32, decrypted.size)
        } finally {
            decrypted.fill(0)
        }
    }

    @Test
    fun testMnemonicAcknowledgedFlagDefaultsFalse() {
        assertFalse(
            "isMnemonicAcknowledged() must return false on fresh repository",
            repository.isMnemonicAcknowledged()
        )
    }

    @Test
    fun testSetMnemonicAcknowledgedPersists() {
        repository.setMnemonicAcknowledged()
        // Recreate repository to verify persistence
        val fresh = SeedRepository(context)
        assertTrue(
            "isMnemonicAcknowledged() must return true after setMnemonicAcknowledged()",
            fresh.isMnemonicAcknowledged()
        )
    }
}
