package com.pktap.app.nfc

import android.content.Context
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import com.pktap.app.keystore.SeedRepository
import com.pktap.bridge.PktapBridge
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * Post-tap state machine for the NFC key exchange flow (D-08).
 */
sealed class PostTapState {
    object Idle : PostTapState()
    object Encrypting : PostTapState()
    object Publishing : PostTapState()
    object Done : PostTapState()
    data class Queued(val message: String) : PostTapState()
    data class Error(val message: String) : PostTapState()
}

/**
 * ViewModel that collects peer keys from [NfcExchangeFlow] and drives the post-tap
 * crypto + publish state machine on [Dispatchers.IO] (NFC-06).
 *
 * Dependencies are injected as function parameters to allow JVM-only unit testing
 * without Android context or Rust FFI.
 *
 * @param decryptSeed     Suspending function that returns a fresh 32-byte seed — caller MUST zero after use
 * @param ecdhEncrypt     ECDH + encrypt — zeroes seed copy internally (matches PktapBridge contract)
 * @param publish         DHT publish — Phase 5: no-op stub (DhtClient not yet exposed via UniFFI);
 *                        Phase 6 will wire the real DhtClient here
 * @param ioDispatcher    Dispatcher for post-tap crypto coroutine — injectable for testing
 */
class NfcViewModel(
    private val decryptSeed: suspend () -> ByteArray,
    private val ecdhEncrypt: suspend (seed: ByteArray, peerPubKey: ByteArray, json: String) -> ByteArray =
        { s, p, j -> PktapBridge.ecdhAndEncrypt(s, p, j) },
    private val publish: suspend (encrypted: ByteArray, peerPubKey: ByteArray) -> Unit =
        { _, _ ->
            // Phase 6 will wire DhtClient here; DhtClient not yet exposed via UniFFI.
        },
    private val ioDispatcher: CoroutineDispatcher = Dispatchers.IO
) : ViewModel() {

    private val _postTapState = MutableStateFlow<PostTapState>(PostTapState.Idle)
    val postTapState: StateFlow<PostTapState> = _postTapState.asStateFlow()

    private val _peerPubKeyHex = MutableStateFlow<String?>(null)
    val peerPubKeyHex: StateFlow<String?> = _peerPubKeyHex.asStateFlow()

    init {
        viewModelScope.launch {
            NfcExchangeFlow.peerKeyFlow.collect { peerRawPayload ->
                if (NfcPayloadBuilder.validateNfcPayload(peerRawPayload)) {
                    val peerPubKey = NfcPayloadBuilder.extractPubKey(peerRawPayload)
                    if (peerPubKey != null) {
                        _peerPubKeyHex.value = peerPubKey.joinToString("") { "%02x".format(it) }
                        launchPostTapCrypto(peerPubKey)
                    }
                }
            }
        }
    }

    private fun launchPostTapCrypto(peerPubKey: ByteArray) {
        viewModelScope.launch(ioDispatcher) {  // NFC-06: must run on background thread
            _postTapState.value = PostTapState.Encrypting
            try {
                val seed = decryptSeed()
                val encrypted: ByteArray = try {
                    // D-07: ecdhAndEncrypt via PktapBridge (zeros its own seed copy)
                    // Phase 6 provides real contact JSON; "{}" is a stub for Phase 5
                    ecdhEncrypt(seed.copyOf(), peerPubKey, "{}")
                } finally {
                    seed.fill(0)  // Belt-and-suspenders zeroing (T-05-08)
                }
                _postTapState.value = PostTapState.Publishing
                publish(encrypted, peerPubKey)
                _postTapState.value = PostTapState.Done
            } catch (e: Exception) {
                _postTapState.value = PostTapState.Error(e.message ?: "Unknown error")
            }
        }
    }

    fun resetState() {
        _postTapState.value = PostTapState.Idle
        _peerPubKeyHex.value = null
    }

    companion object {
        /**
         * ViewModelFactory for production use. Wires real SeedRepository + PktapBridge.
         */
        fun factory(context: Context) = viewModelFactory {
            initializer {
                NfcViewModel(
                    decryptSeed = { SeedRepository(context.applicationContext).decryptSeed() }
                )
            }
        }
    }
}
