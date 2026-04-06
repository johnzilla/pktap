package com.pktap.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.material3.MaterialTheme
import com.pktap.app.keystore.SeedRepository
import com.pktap.app.navigation.AppNavigation
import com.pktap.app.navigation.Main
import com.pktap.app.navigation.MnemonicSetup
import com.pktap.app.nfc.NfcReader

/**
 * Single-activity entry point for PKTap.
 *
 * Determines the start destination synchronously from SharedPreferences before setContent:
 * - No seed → [MnemonicSetup] (first launch — generate seed, show mnemonic)
 * - Seed present, mnemonic NOT acknowledged → [MnemonicSetup] (interrupted setup, D-08)
 * - Seed present, mnemonic acknowledged → [Main] (returning user, D-07)
 *
 * NFC reader mode is managed here via [NfcReader]:
 * - enableReaderMode() in onResume — active only while app is in foreground (D-02, avoids Pitfall 3)
 * - disableReaderMode() in onPause — prevents background NFC reads
 *
 * SharedPreferences boolean reads are synchronous and fast — no splash screen needed.
 */
class MainActivity : ComponentActivity() {

    private lateinit var nfcReader: NfcReader

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        nfcReader = NfcReader(this)

        val seedRepository = SeedRepository(applicationContext)

        // Determine start destination before setContent to avoid flashing wrong screen (D-07, D-08)
        val startDestination: Any = when {
            !seedRepository.hasSeed() -> MnemonicSetup
            !seedRepository.isMnemonicAcknowledged() -> MnemonicSetup
            else -> Main
        }

        setContent {
            MaterialTheme {
                AppNavigation(
                    startDestination = startDestination,
                    isNfcAvailable = nfcReader.isNfcAvailable(),
                    isNfcEnabled = nfcReader.isNfcEnabled()
                )
            }
        }
    }

    override fun onResume() {
        super.onResume()
        // D-02: enable reader mode only if NFC is on — avoids exception on disabled NFC
        if (nfcReader.isNfcEnabled()) {
            nfcReader.enableReaderMode()
        }
        // D-06: NFC disabled state is surfaced via isNfcEnabled param to AppNavigation/MainScreen
    }

    override fun onPause() {
        super.onPause()
        nfcReader.disableReaderMode()
    }
}
