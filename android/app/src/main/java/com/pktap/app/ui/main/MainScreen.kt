package com.pktap.app.ui.main

import android.content.Intent
import android.provider.Settings
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.pktap.app.AppViewModel

/**
 * Main screen for PKTap. Shows the cached Ed25519 public key and NFC exchange readiness.
 *
 * D-06: When NFC hardware is present but disabled, shows a [Card] with a link to
 * [Settings.ACTION_NFC_SETTINGS] so the user can enable NFC without leaving the app.
 *
 * @param isNfcAvailable  True if the device has NFC hardware
 * @param isNfcEnabled    True if NFC is currently enabled
 */
@Composable
fun MainScreen(
    isNfcAvailable: Boolean = true,
    isNfcEnabled: Boolean = true
) {
    val context = LocalContext.current
    val appViewModel: AppViewModel = viewModel(
        factory = AppViewModel.factory(context)
    )
    val publicKeyHex by appViewModel.publicKeyHex.collectAsState()

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(32.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        // D-06: NFC disabled card — shown when hardware present but NFC is off
        if (isNfcAvailable && !isNfcEnabled) {
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(bottom = 24.dp),
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.errorContainer
                )
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp, vertical = 8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Text(
                        text = "NFC is off",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onErrorContainer
                    )
                    TextButton(
                        onClick = {
                            context.startActivity(Intent(Settings.ACTION_NFC_SETTINGS))
                        }
                    ) {
                        Text(
                            text = "Enable",
                            color = MaterialTheme.colorScheme.onErrorContainer
                        )
                    }
                }
            }
        }

        Text(
            text = "PKTap",
            style = MaterialTheme.typography.headlineLarge
        )

        Spacer(modifier = Modifier.height(24.dp))

        Text(
            text = "Your public key:",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )

        Spacer(modifier = Modifier.height(8.dp))

        val displayKey = if (publicKeyHex.length > 16) {
            "${publicKeyHex.take(8)}…${publicKeyHex.takeLast(8)}"
        } else {
            publicKeyHex.ifEmpty { "Deriving…" }
        }

        Text(
            text = displayKey,
            style = MaterialTheme.typography.bodySmall,
            fontFamily = FontFamily.Monospace,
            color = MaterialTheme.colorScheme.primary
        )

        Spacer(modifier = Modifier.height(24.dp))

        Text(
            text = if (isNfcEnabled) "Ready for NFC exchange" else "Enable NFC to exchange contacts",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}
