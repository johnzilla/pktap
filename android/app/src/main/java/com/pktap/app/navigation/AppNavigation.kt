package com.pktap.app.navigation

import androidx.compose.runtime.Composable
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import com.pktap.app.ui.main.MainScreen
import com.pktap.app.ui.onboarding.MnemonicScreen
import kotlinx.serialization.Serializable

@Serializable
object MnemonicSetup

@Serializable
object Main

/**
 * Root navigation graph for PKTap.
 *
 * [startDestination] is computed in MainActivity before setContent to avoid flashing
 * the wrong screen (D-07, D-08).
 *
 * Routes:
 * - [MnemonicSetup] — shown on first launch or interrupted setup
 * - [Main] — shown to returning users who have acknowledged their mnemonic
 */
@Composable
fun AppNavigation(startDestination: Any) {
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
            MainScreen()
        }
    }
}
