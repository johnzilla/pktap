package com.pktap.app.ui.onboarding

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
 * ViewModel for the mnemonic display screen.
 *
 * Security properties (T-04-08, T-04-09, T-04-13):
 * - Seed decrypted on IO dispatcher, passed to PktapBridge (which zeros it), local copy zeroed in finally
 * - Seed is NEVER assigned to a ViewModel field — only exists in local coroutine scope
 * - Only operation names logged, never word values (T-04-09)
 */
class MnemonicViewModel(private val seedRepository: SeedRepository) : ViewModel() {

    private companion object {
        const val TAG = "MnemonicViewModel"
    }

    private val _words = MutableStateFlow<List<String>>(emptyList())
    val words: StateFlow<List<String>> = _words.asStateFlow()

    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    init {
        viewModelScope.launch(Dispatchers.IO) {
            try {
                val seed: ByteArray = if (seedRepository.hasSeed()) {
                    seedRepository.decryptSeed()
                } else {
                    seedRepository.generateAndStoreSeed()
                }
                val mnemonic: String = try {
                    // deriveMnemonicFromSeed zeros its own copy of seedBytes internally (D-05);
                    // we pass a copy so we can zero the original in finally regardless.
                    PktapBridge.deriveMnemonicFromSeed(seed.copyOf())
                } finally {
                    // Zero the original seed — copyOf() was passed to bridge which zeros that copy
                    seed.fill(0)
                }
                Log.d(TAG, "Mnemonic derived")  // No word values logged (T-04-09)
                _words.value = mnemonic.split(" ")
            } catch (e: Exception) {
                Log.e(TAG, "Failed to derive mnemonic: ${e.javaClass.simpleName}")
                _words.value = emptyList()
            } finally {
                _isLoading.value = false
            }
        }
    }

    /**
     * Mark that the user has acknowledged their mnemonic backup phrase.
     * Must be called before [onAcknowledged] navigates away.
     */
    fun acknowledge() {
        seedRepository.setMnemonicAcknowledged()
        Log.d(TAG, "Mnemonic acknowledged")
    }

    companion object {
        fun factory(context: Context) = viewModelFactory {
            initializer { MnemonicViewModel(SeedRepository(context.applicationContext)) }
        }
    }
}
