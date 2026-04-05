package com.pktap.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.material3.MaterialTheme
import com.pktap.app.keystore.SeedRepository
import com.pktap.app.navigation.AppNavigation
import com.pktap.app.navigation.Main
import com.pktap.app.navigation.MnemonicSetup

/**
 * Single-activity entry point for PKTap.
 *
 * Determines the start destination synchronously from SharedPreferences before setContent:
 * - No seed → [MnemonicSetup] (first launch — generate seed, show mnemonic)
 * - Seed present, mnemonic NOT acknowledged → [MnemonicSetup] (interrupted setup, D-08)
 * - Seed present, mnemonic acknowledged → [Main] (returning user, D-07)
 *
 * SharedPreferences boolean reads are synchronous and fast — no splash screen needed.
 */
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val seedRepository = SeedRepository(applicationContext)

        // Determine start destination before setContent to avoid flashing wrong screen (D-07, D-08)
        val startDestination: Any = when {
            !seedRepository.hasSeed() -> MnemonicSetup
            !seedRepository.isMnemonicAcknowledged() -> MnemonicSetup
            else -> Main
        }

        setContent {
            MaterialTheme {
                AppNavigation(startDestination = startDestination)
            }
        }
    }
}
