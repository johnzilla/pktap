# Phase 5: NFC HCE Module - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-05
**Phase:** 05-nfc-hce-module
**Areas discussed:** APDU protocol design, HCE service lifecycle, OEM compatibility, Post-tap processing flow

---

## APDU Protocol Design

| Option | Description | Selected |
|--------|-------------|----------|
| Reader sends key in command, HCE responds | Single round-trip, both get each other's key | ✓ |
| SELECT AID returns HCE key, follow-up sends reader key | Two round-trips | |

**User's choice:** Single APDU round-trip

| Option | Description | Selected |
|--------|-------------|----------|
| Both roles always active | No user action to pick role | ✓ |
| Manual role selection | UI toggle, adds friction | |

**User's choice:** Both roles always active

---

## HCE Service Lifecycle

| Option | Description | Selected |
|--------|-------------|----------|
| Pre-cache payload on app start | Zero computation in processCommandApdu | ✓ |
| Build lazily on first tap | Slightly delayed first tap | |

**User's choice:** Pre-cache on app start

| Option | Description | Selected |
|--------|-------------|----------|
| SharedFlow | Clean Compose-era pattern | ✓ |
| LocalBroadcast | Traditional, more boilerplate | |

**User's choice:** SharedFlow for peer key delivery

---

## OEM Compatibility

| Option | Description | Selected |
|--------|-------------|----------|
| Category 'other' with proprietary AID | Avoids payment conflicts, reliable routing | ✓ |
| Category 'payment' | Highest priority but conflicts with Pay apps | |

**User's choice:** Category 'other', AID = F0504B544150

| Option | Description | Selected |
|--------|-------------|----------|
| Prompt to enable NFC | Dialog linking to settings | ✓ |
| Just show error | Toast, easily missed | |

**User's choice:** Prompt with settings link

---

## Post-Tap Processing

| Option | Description | Selected |
|--------|-------------|----------|
| ViewModel scope after UI receives key | Survives config changes, has UI feedback | ✓ |
| HCE onDeactivated() | No UI feedback channel | |
| WorkManager | Most reliable but heaviest | |

**User's choice:** ViewModel scope on Dispatchers.IO

| Option | Description | Selected |
|--------|-------------|----------|
| Status in post-tap screen | Encrypting → Publishing → Done/Error | ✓ |
| Toast notification | Simple but easily missed | |

**User's choice:** Post-tap status screen

---

## Claude's Discretion

- APDU byte layout details
- SharedFlow vs Channel
- NFC reader dispatch config
- CRC-16 implementation location
- Post-tap screen design

## Deferred Ideas

None.
