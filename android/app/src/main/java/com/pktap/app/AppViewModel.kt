package com.pktap.app

import android.content.Context
import android.util.Log
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import com.pktap.app.keystore.SeedRepository
import com.pktap.bridge.PktapBridge
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * Application-scoped ViewModel that caches the Ed25519 public key derived from the seed.
 *
 * Security properties (D-06, KEY-01):
 * - The public key is NOT secret — caching is safe.
 * - The seed is decrypted on IO dispatcher, passed to PktapBridge (which zeros it),
 *   local copy zeroed in finally. Seed is NEVER assigned to a ViewModel field.
 * - [publicKeyBytes] cached here for NFC/DHT use in Phase 5+.
 */
class AppViewModel(private val seedRepository: SeedRepository) : ViewModel() {

    private companion object {
        const val TAG = "AppViewModel"
    }

    private val _publicKeyHex = MutableStateFlow("")
    val publicKeyHex: StateFlow<String> = _publicKeyHex.asStateFlow()

    // Public key bytes cached for NFC/DHT use — not secret (D-06)
    private var _publicKeyBytes: ByteArray? = null
    val publicKeyBytes: ByteArray? get() = _publicKeyBytes

    init {
        viewModelScope.launch(Dispatchers.IO) {
            if (!seedRepository.hasSeed()) {
                Log.d(TAG, "No seed yet — public key will be derived after first-launch setup")
                return@launch
            }
            try {
                val seed = seedRepository.decryptSeed()
                val pubKey: ByteArray = try {
                    // derivePublicKey zeros its own copy; zero our original in finally (D-06)
                    PktapBridge.derivePublicKey(seed.copyOf())
                } finally {
                    seed.fill(0)
                }
                _publicKeyBytes = pubKey
                _publicKeyHex.value = pubKey.joinToString("") { "%02x".format(it) }
                Log.d(TAG, "Public key derived and cached")
            } catch (e: Exception) {
                Log.e(TAG, "Failed to derive public key: ${e.javaClass.simpleName}")
            }
        }
    }

    companion object {
        fun factory(context: Context) = viewModelFactory {
            initializer { AppViewModel(SeedRepository(context.applicationContext)) }
        }
    }
}
