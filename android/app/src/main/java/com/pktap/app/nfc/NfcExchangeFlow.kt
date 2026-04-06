package com.pktap.app.nfc

import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow

/**
 * Singleton SharedFlow for delivering received peer NFC public keys from the HCE service
 * to the ViewModel layer (D-04).
 *
 * Design choices:
 * - replay=0: no stale key delivered to late subscribers
 * - extraBufferCapacity=1: tryEmit from processCommandApdu (main thread, no suspend) succeeds
 * - DROP_OLDEST: if ViewModel lags, drop the oldest unprocessed key; no security impact (T-05-05)
 */
object NfcExchangeFlow {
    val peerKeyFlow = MutableSharedFlow<ByteArray>(
        replay = 0,
        extraBufferCapacity = 1,
        onBufferOverflow = BufferOverflow.DROP_OLDEST
    )
}
