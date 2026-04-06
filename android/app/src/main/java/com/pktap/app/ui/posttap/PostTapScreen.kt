package com.pktap.app.ui.posttap

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.pktap.app.nfc.NfcViewModel
import com.pktap.app.nfc.PostTapState

/**
 * Post-tap status screen showing the NFC key exchange state machine progression (D-08).
 *
 * States displayed:
 * - Idle: "Waiting for tap..." with progress indicator
 * - Encrypting: "Encrypting..." with progress indicator
 * - Publishing: "Publishing..." with progress indicator
 * - Done: checkmark + "Exchange complete" + Done button
 * - Queued: info icon + "Queued for sync" + message
 * - Error: warning icon + error message + Back button
 *
 * @param nfcViewModel  The shared NfcViewModel driving post-tap state
 * @param onDone        Called when user taps Done or Back to return to main screen
 */
@Composable
fun PostTapScreen(nfcViewModel: NfcViewModel, onDone: () -> Unit) {
    val state by nfcViewModel.postTapState.collectAsState()
    val peerHex by nfcViewModel.peerPubKeyHex.collectAsState()

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Text(
            text = "Contact Exchange",
            style = MaterialTheme.typography.headlineMedium
        )
        Spacer(modifier = Modifier.height(16.dp))

        peerHex?.let { hex ->
            Text(
                text = "Peer: ${hex.take(8)}...${hex.takeLast(8)}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Spacer(modifier = Modifier.height(24.dp))
        }

        when (state) {
            is PostTapState.Idle -> {
                CircularProgressIndicator()
                Spacer(modifier = Modifier.height(8.dp))
                Text("Waiting for tap...")
            }
            is PostTapState.Encrypting -> {
                CircularProgressIndicator()
                Spacer(modifier = Modifier.height(8.dp))
                Text("Encrypting...")
            }
            is PostTapState.Publishing -> {
                CircularProgressIndicator()
                Spacer(modifier = Modifier.height(8.dp))
                Text("Publishing...")
            }
            is PostTapState.Done -> {
                Icon(
                    imageVector = Icons.Filled.CheckCircle,
                    contentDescription = "Done",
                    tint = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.size(48.dp)
                )
                Spacer(modifier = Modifier.height(8.dp))
                Text("Exchange complete")
                Spacer(modifier = Modifier.height(16.dp))
                Button(onClick = {
                    nfcViewModel.resetState()
                    onDone()
                }) {
                    Text("Done")
                }
            }
            is PostTapState.Queued -> {
                Icon(
                    imageVector = Icons.Filled.Info,
                    contentDescription = "Queued",
                    tint = MaterialTheme.colorScheme.secondary,
                    modifier = Modifier.size(48.dp)
                )
                Spacer(modifier = Modifier.height(8.dp))
                Text("Queued for sync")
                Text(
                    text = (state as PostTapState.Queued).message,
                    style = MaterialTheme.typography.bodySmall
                )
            }
            is PostTapState.Error -> {
                Icon(
                    imageVector = Icons.Filled.Warning,
                    contentDescription = "Error",
                    tint = MaterialTheme.colorScheme.error,
                    modifier = Modifier.size(48.dp)
                )
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = "Error",
                    color = MaterialTheme.colorScheme.error
                )
                Text(
                    text = (state as PostTapState.Error).message,
                    style = MaterialTheme.typography.bodySmall
                )
                Spacer(modifier = Modifier.height(16.dp))
                Button(onClick = {
                    nfcViewModel.resetState()
                    onDone()
                }) {
                    Text("Back")
                }
            }
        }
    }
}
