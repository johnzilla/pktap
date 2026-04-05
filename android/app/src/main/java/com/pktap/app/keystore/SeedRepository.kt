package com.pktap.app.keystore

import android.content.Context
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.security.SecureRandom

/**
 * Manages the encrypted HKDF seed stored in plain SharedPreferences.
 *
 * The seed is encrypted by [KeystoreManager] using a non-extractable AES-256-GCM key.
 * Only the encrypted blob lives on disk — the raw seed bytes exist only in memory
 * during active use and must be zeroed by the caller in a finally block (T-04-01, T-04-06).
 *
 * Security properties (D-01, D-07, D-08, KEY-03, T-04-04, T-04-06):
 * - No seed ByteArray field on this class (T-04-06 — no class-level secret material)
 * - KeyPermanentlyInvalidatedException caught and prefs cleared to trigger re-setup (T-04-04)
 * - mnemonic_acknowledged flag tracks whether the user has seen their backup phrase (D-08)
 */
class SeedRepository(context: Context) {

    private companion object {
        const val TAG = "SeedRepository"
        const val PREFS_FILE = "pktap_secure_prefs"
        const val KEY_SEED_ENCRYPTED = "seed_encrypted"
        const val KEY_MNEMONIC_ACKNOWLEDGED = "mnemonic_acknowledged"
    }

    private val prefs = context.getSharedPreferences(PREFS_FILE, Context.MODE_PRIVATE)

    /**
     * Returns true if an encrypted seed is stored (D-07 detection).
     * Check this before calling [decryptSeed] to avoid IllegalStateException.
     */
    fun hasSeed(): Boolean = prefs.contains(KEY_SEED_ENCRYPTED)

    /**
     * Returns true if the user has acknowledged their BIP-39 mnemonic backup phrase (D-08).
     */
    fun isMnemonicAcknowledged(): Boolean =
        prefs.getBoolean(KEY_MNEMONIC_ACKNOWLEDGED, false)

    /**
     * Mark that the user has seen and acknowledged their mnemonic backup phrase.
     */
    fun setMnemonicAcknowledged() {
        prefs.edit().putBoolean(KEY_MNEMONIC_ACKNOWLEDGED, true).apply()
    }

    /**
     * Generate a fresh 32-byte HKDF seed, encrypt it, store in SharedPreferences.
     *
     * Resets mnemonic_acknowledged to false (Pitfall 6 — new seed = new mnemonic to show).
     * Runs on [Dispatchers.IO] (Keystore operations are slow).
     *
     * @return The raw 32-byte seed — caller MUST zero it in a finally block (T-04-01)
     */
    suspend fun generateAndStoreSeed(): ByteArray = withContext(Dispatchers.IO) {
        val seed = ByteArray(32).also { SecureRandom().nextBytes(it) }
        val key = KeystoreManager.generateOrGetKey(KeystoreManager.SEED_KEY_ALIAS)
        val encrypted = KeystoreManager.encrypt(seed, key)
        prefs.edit()
            .putString(KEY_SEED_ENCRYPTED, encrypted)
            .putBoolean(KEY_MNEMONIC_ACKNOWLEDGED, false)
            .apply()
        Log.d(TAG, "Seed generated and stored")
        seed
    }

    /**
     * Decrypt and return the stored seed.
     *
     * Runs on [Dispatchers.IO] (Keystore operations are slow).
     *
     * @return The raw 32-byte seed — caller MUST zero it in a finally block (T-04-01)
     * @throws IllegalStateException if no seed is stored (call [hasSeed] first — D-07)
     * @throws SeedInvalidatedException if the Keystore key was permanently invalidated
     *         (factory reset or re-enrollment) — caller must trigger re-setup flow (T-04-04)
     */
    suspend fun decryptSeed(): ByteArray = withContext(Dispatchers.IO) {
        val encrypted = prefs.getString(KEY_SEED_ENCRYPTED, null)
            ?: throw IllegalStateException("No seed stored — call hasSeed() before decryptSeed()")
        val key = try {
            KeystoreManager.generateOrGetKey(KeystoreManager.SEED_KEY_ALIAS)
        } catch (e: KeyPermanentlyInvalidatedException) {
            Log.w(TAG, "Keystore key permanently invalidated — clearing prefs for re-setup")
            prefs.edit().clear().apply()
            throw SeedInvalidatedException("Keystore key invalidated — re-setup required", e)
        }
        try {
            KeystoreManager.decrypt(encrypted, key)
        } catch (e: KeyPermanentlyInvalidatedException) {
            Log.w(TAG, "Keystore key permanently invalidated during decrypt — clearing prefs")
            prefs.edit().clear().apply()
            throw SeedInvalidatedException("Keystore key invalidated — re-setup required", e)
        }
    }
}

/**
 * Thrown when the Android Keystore key has been permanently invalidated
 * (factory reset, biometric re-enrollment on key bound to biometrics, etc.).
 *
 * The caller must clear app state and guide the user through re-setup (T-04-04).
 */
class SeedInvalidatedException(message: String, cause: Throwable? = null) :
    Exception(message, cause)
