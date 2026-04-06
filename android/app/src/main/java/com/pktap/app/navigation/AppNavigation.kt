package com.pktap.app.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import com.pktap.app.nfc.NfcViewModel
import com.pktap.app.ui.main.MainScreen
import com.pktap.app.ui.onboarding.MnemonicScreen
import com.pktap.app.ui.posttap.PostTapScreen
import kotlinx.serialization.Serializable

@Serializable
object MnemonicSetup

@Serializable
object Main

@Serializable
object PostTap

/**
 * Root navigation graph for PKTap.
 *
 * [startDestination] is computed in MainActivity before setContent to avoid flashing
 * the wrong screen (D-07, D-08).
 *
 * Routes:
 * - [MnemonicSetup] — shown on first launch or interrupted setup
 * - [Main] — shown to returning users who have acknowledged their mnemonic
 * - [PostTap] — shown after NFC tap; displays post-tap crypto status progression (D-08)
 */
@Composable
fun AppNavigation(
    startDestination: Any,
    isNfcAvailable: Boolean = true,
    isNfcEnabled: Boolean = true
) {
    val navController = rememberNavController()
    NavHost(navController = navController, startDestination = startDestination) {
        composable<MnemonicSetup> {
            MnemonicScreen(
                onAcknowledged = {
                    navController.navigate(Main) {
                        popUpTo<MnemonicSetup> { inclusive = true }
                    }
                }
            )
        }
        composable<Main> {
            val nfcViewModel: NfcViewModel = viewModel(
                factory = NfcViewModel.factory(navController.context)
            )
            val peerHex by nfcViewModel.peerPubKeyHex.collectAsState()

            // Auto-navigate to PostTap when a peer key is received via NFC tap
            LaunchedEffect(peerHex) {
                if (peerHex != null) {
                    navController.navigate(PostTap)
                }
            }

            MainScreen(
                isNfcAvailable = isNfcAvailable,
                isNfcEnabled = isNfcEnabled
            )
        }
        composable<PostTap> {
            val nfcViewModel: NfcViewModel = viewModel(
                factory = NfcViewModel.factory(navController.context)
            )
            PostTapScreen(
                nfcViewModel = nfcViewModel,
                onDone = {
                    navController.navigate(Main) {
                        popUpTo<PostTap> { inclusive = true }
                    }
                }
            )
        }
    }
}
