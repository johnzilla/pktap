package com.pktap.app.nfc

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

/**
 * JVM unit tests for NfcViewModel state transitions.
 *
 * NfcViewModel accepts injectable lambdas for decryptSeed, ecdhEncrypt, and publish
 * so no Android context or Rust FFI is needed in JVM tests.
 *
 * Pattern: advanceUntilIdle() BEFORE tryEmit to let the init collector subscribe first
 * (StandardTestDispatcher does not run coroutines eagerly), then advanceUntilIdle() AFTER
 * to drain all pending work.
 */
@OptIn(ExperimentalCoroutinesApi::class)
class NfcViewModelTest {

    private val testDispatcher = StandardTestDispatcher()

    private val validPubKey = ByteArray(32) { it.toByte() }
    private val validPayload: ByteArray by lazy {
        NfcPayloadBuilder.buildNfcPayload(validPubKey)
    }

    private val fakeSeed = ByteArray(32) { 0x42.toByte() }
    private var seedDecryptCount = 0

    private fun makeSeedProvider(): suspend () -> ByteArray = {
        seedDecryptCount++
        fakeSeed.copyOf()
    }

    @Before
    fun setUp() {
        Dispatchers.setMain(testDispatcher)
        seedDecryptCount = 0
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    // ─── Helper ────────────────────────────────────────────────────────��───────

    private fun buildViewModel(
        encryptResult: ByteArray = ByteArray(64) { 0xAA.toByte() },
        encryptThrows: Exception? = null,
        publishThrows: Exception? = null
    ): NfcViewModel {
        val encrypt: suspend (ByteArray, ByteArray, String) -> ByteArray = { seed, _, _ ->
            if (encryptThrows != null) throw encryptThrows
            encryptResult
        }
        val pub: suspend (ByteArray, ByteArray) -> Unit = { _, _ ->
            if (publishThrows != null) throw publishThrows
        }
        return NfcViewModel(
            decryptSeed = makeSeedProvider(),
            ecdhEncrypt = encrypt,
            publish = pub,
            ioDispatcher = testDispatcher
        )
    }

    // ─── Tests ─────────────────────────────────────────────────────────────────

    @Test
    fun `initial state is Idle`() {
        val vm = buildViewModel()
        assertEquals(PostTapState.Idle, vm.postTapState.value)
    }

    @Test
    fun `peerPubKeyHex is null initially`() {
        val vm = buildViewModel()
        assertEquals(null, vm.peerPubKeyHex.value)
    }

    @Test
    fun `receiving valid peer payload transitions to Done`() = runTest {
        val vm = buildViewModel()
        advanceUntilIdle()  // let init collector subscribe
        NfcExchangeFlow.peerKeyFlow.tryEmit(validPayload)
        advanceUntilIdle()  // drain all coroutines

        assertEquals(PostTapState.Done, vm.postTapState.value)
    }

    @Test
    fun `state reaches Done after full state machine run`() = runTest {
        val encrypt: suspend (ByteArray, ByteArray, String) -> ByteArray = { _, _, _ -> ByteArray(64) }
        val pub: suspend (ByteArray, ByteArray) -> Unit = { _, _ -> }
        val vm = NfcViewModel(
            decryptSeed = makeSeedProvider(),
            ecdhEncrypt = encrypt,
            publish = pub,
            ioDispatcher = testDispatcher
        )
        advanceUntilIdle()
        NfcExchangeFlow.peerKeyFlow.tryEmit(validPayload)
        advanceUntilIdle()

        assertEquals(PostTapState.Done, vm.postTapState.value)
    }

    @Test
    fun `peerPubKeyHex is set to hex of peer public key after valid payload`() = runTest {
        val vm = buildViewModel()
        advanceUntilIdle()
        NfcExchangeFlow.peerKeyFlow.tryEmit(validPayload)
        advanceUntilIdle()

        val hex = vm.peerPubKeyHex.value
        assertNotNull(hex)
        assertEquals(64, hex!!.length)  // 32 bytes = 64 hex chars
        assertTrue(hex.startsWith("00"))  // first byte of validPubKey is 0x00
    }

    @Test
    fun `ecdhAndEncrypt exception transitions to Error state`() = runTest {
        val vm = buildViewModel(encryptThrows = RuntimeException("crypto failed"))
        advanceUntilIdle()
        NfcExchangeFlow.peerKeyFlow.tryEmit(validPayload)
        advanceUntilIdle()

        val state = vm.postTapState.value
        assertTrue("Expected Error but got $state", state is PostTapState.Error)
        assertEquals("crypto failed", (state as PostTapState.Error).message)
    }

    @Test
    fun `publish exception transitions to Error state`() = runTest {
        val vm = buildViewModel(publishThrows = RuntimeException("dht unavailable"))
        advanceUntilIdle()
        NfcExchangeFlow.peerKeyFlow.tryEmit(validPayload)
        advanceUntilIdle()

        val state = vm.postTapState.value
        assertTrue("Expected Error but got $state", state is PostTapState.Error)
        assertEquals("dht unavailable", (state as PostTapState.Error).message)
    }

    @Test
    fun `seed is zeroed after use in finally block`() = runTest {
        val seedRef = arrayOfNulls<ByteArray>(1)
        val seedProvider: suspend () -> ByteArray = {
            val s = ByteArray(32) { 0x55.toByte() }
            seedRef[0] = s
            s
        }
        val encrypt: suspend (ByteArray, ByteArray, String) -> ByteArray = { _, _, _ -> ByteArray(64) }
        val pub: suspend (ByteArray, ByteArray) -> Unit = { _, _ -> }
        val vm = NfcViewModel(
            decryptSeed = seedProvider,
            ecdhEncrypt = encrypt,
            publish = pub,
            ioDispatcher = testDispatcher
        )

        advanceUntilIdle()
        NfcExchangeFlow.peerKeyFlow.tryEmit(validPayload)
        advanceUntilIdle()

        val seedAfterUse = seedRef[0]
        assertNotNull("Seed should have been decrypted", seedAfterUse)
        assertTrue(
            "Seed should be zeroed after use (finally block)",
            seedAfterUse!!.all { it == 0.toByte() }
        )
    }

    @Test
    fun `post-tap coroutine is invoked on injected dispatcher`() = runTest {
        var encryptWasCalled = false
        val encrypt: suspend (ByteArray, ByteArray, String) -> ByteArray = { _, _, _ ->
            encryptWasCalled = true
            ByteArray(64)
        }
        val pub: suspend (ByteArray, ByteArray) -> Unit = { _, _ -> }
        val vm = NfcViewModel(
            decryptSeed = makeSeedProvider(),
            ecdhEncrypt = encrypt,
            publish = pub,
            ioDispatcher = testDispatcher
        )

        advanceUntilIdle()
        NfcExchangeFlow.peerKeyFlow.tryEmit(validPayload)
        advanceUntilIdle()

        assertTrue("ecdhEncrypt should have been called — post-tap coroutine did not run", encryptWasCalled)
    }

    @Test
    fun `invalid peer payload is ignored — state stays Idle`() = runTest {
        val vm = buildViewModel()
        advanceUntilIdle()

        val badPayload = ByteArray(36) { 0xFF.toByte() }
        NfcExchangeFlow.peerKeyFlow.tryEmit(badPayload)
        advanceUntilIdle()

        assertEquals(PostTapState.Idle, vm.postTapState.value)
    }

    @Test
    fun `resetState returns to Idle and clears peerPubKeyHex`() = runTest {
        val vm = buildViewModel()
        advanceUntilIdle()
        NfcExchangeFlow.peerKeyFlow.tryEmit(validPayload)
        advanceUntilIdle()

        assertEquals(PostTapState.Done, vm.postTapState.value)
        assertNotNull(vm.peerPubKeyHex.value)

        vm.resetState()

        assertEquals(PostTapState.Idle, vm.postTapState.value)
        assertEquals(null, vm.peerPubKeyHex.value)
    }
}
