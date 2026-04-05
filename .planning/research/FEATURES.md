# Feature Landscape

**Domain:** Decentralized encrypted NFC contact exchange (Android)
**Researched:** 2026-04-04
**Confidence note:** Web search unavailable. All findings from training data (cutoff August 2025) on Popl, Linq, HiHello, Dot, Blinq, Nostr-based contact apps, Signal UX patterns, and DID/Pkarr ecosystem. Confidence levels reflect this limitation.

---

## Table Stakes

Features users expect from any contact exchange app. Missing = product feels broken or untrustworthy.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| NFC tap initiates exchange | The entire category promise; anything else feels like a workaround | Med | HCE bidirectional is harder than tag-read — already accounted for in PROJECT.md |
| Contact preview before saving | Every NFC/QR card app shows a preview screen — users expect to confirm before storing | Low | Show decrypted fields, let user cancel |
| Save / reject received contact | Binary decision after preview — saves to local contact list or discards | Low | Must be explicit; no auto-save without consent |
| Contact list (received contacts) | Users need to find people they tapped later | Low | SQLite-backed, AES-encrypted sensitive columns |
| Profile setup (own card) | User must enter their own info before they can share anything | Low | Name + at least one contact field |
| Field selection before sharing | Users want to share different info in different contexts (work vs personal) | Med | Already in PROJECT.md — critical for trust |
| QR code fallback | NFC fails on some phones or cases; QR is now a universal secondary gesture | Med | Already in PROJECT.md; async handshake pattern is correct |
| Clear error messages on tap failure | NFC fails often (angle, case, distance) — silent failures are the #1 NFC UX complaint | Low | "Hold phones back-to-back" guidance, retry prompts |
| Works without internet | DHT is not guaranteed reachable — graceful degradation matters | Med | Offline queue for publish; show "pending sync" state |
| Onboarding that explains what just happened | First tap is confusing — what was exchanged? Who saw what? | Low | Single-screen explainer post-tap, not a 5-screen tutorial |

**Confidence:** MEDIUM — drawn from competitive analysis of Popl/Linq/HiHello behavior documented in reviews and blog posts through mid-2025. Field selection as table stakes is specific to privacy apps; mainstream NFC card apps often omit it, making it a slight differentiator too (see below).

---

## Differentiators

Features that set PKTap apart. Not expected by mainstream NFC app users, but valued by the privacy-conscious target audience.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| End-to-end encrypted exchange | No server sees contact data; only the two tapping parties can decrypt | High | Core architecture. The entire Rust crypto stack. Already in PROJECT.md |
| No account required | Zero signup friction; no email, no phone number, no Google login | Low | Architecture decision, not a feature to build — just don't build auth |
| BIP-39 seed backup | Self-sovereign identity; user controls key recovery without a cloud account | Med | Tension: non-extractable key + mnemonic display = brief plaintext exposure. Acknowledged in PROJECT.md |
| Deterministic DHT address | Exchange works without a server coordinating the rendezvous — both parties derive the same address | High | Users never see this; the differentiator is that it works with no PKTap infrastructure |
| TTL-based record expiry | Shared contact data doesn't persist indefinitely on the DHT — reduces data permanence risk | Low | Already in PROJECT.md. Communicate expiry to user ("your card expires in 24h") |
| Encrypted-by-default, public as opt-in | Opposite of most NFC card apps (public by default) — privacy is the default state | Low | UX framing decision. Show padlock vs. globe icon prominently |
| Key verification / "last verified" timestamp | Users can see when contact info was last confirmed valid from the DHT | Low | Reduces staleness risk for long-term contacts. Already in PROJECT.md |
| Memory zeroing of secrets | Defense-in-depth: secrets don't linger in RAM after use | High | Invisible to users but important for security-conscious audience who read the README |
| Android Keystore / StrongBox binding | Private key is hardware-bound and non-extractable | Med | Again invisible UX, but a meaningful trust claim in the app store description |
| Rust crypto core | Auditable, memory-safe, portable — not JVM crypto libraries with GC-managed secret lifetimes | High | Developer/security audience differentiator, not end-user-visible |

**Confidence:** HIGH for the "no account" and "encrypted by default" differentiators — these are clearly absent from Popl/Linq/HiHello which all require accounts and cloud infrastructure. MEDIUM for BIP-39 and DHT address as differentiators vs. other DID/Pkarr apps.

---

## Anti-Features

Features to explicitly NOT build. Building these would undermine the core value proposition or waste MVP resources.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Cloud sync / backup | Destroys the "no server" guarantee; creates a honeypot of contact graphs | Let BIP-39 seed restore be the only recovery path |
| Analytics / telemetry | Contradicts privacy promise; even aggregate usage metrics create trust problems with this audience | Ship with zero telemetry; mention it explicitly in privacy policy |
| Social graph / follower model | Turns the app into a network product — requires servers, creates lock-in, misaligns with decentralization | Contacts are a local list, period |
| Profile photos | Pkarr 1000-byte record limit makes this technically impractical; even if worked around, binary blobs in DHT are poor hygiene | Text fields only; document the constraint clearly |
| Centralized relay / TURN server | Even as a "convenience" DHT bootstrap — erodes the no-server story | Use hardcoded public DHT bootstrap nodes (standard Mainline DHT practice) |
| Push notifications via FCM | Requires Google infrastructure, reveals IP, breaks the no-cloud model | Poll on app open; async handshake with 2s polling already addresses QR flow |
| Auto-save received contacts | Consent is load-bearing for trust — silently writing a stranger's data to local storage is a dark pattern | Always show preview; always require explicit save |
| Contact import from phone address book | Increases attack surface, permission creep, scope creep | Stay focused on exchange; let users manually build the PKTap contact list |
| Web dashboard / CRM features | Linq/Popl compete here; PKTap's audience explicitly doesn't want a SaaS | Not this product |
| Mandatory identity linking (phone/email verification) | Breaks pseudonymity; the public key IS the identity | No verification step; trust is established via the tap context, not a third party |
| Expiring contacts without warning | Contacts that silently vanish after TTL feel like data loss | Show "expires in Xh" label; offer manual refresh; warn before expiry |

**Confidence:** HIGH — anti-features are derivable from the stated architecture constraints and the explicit privacy-first positioning. These are design commitments, not empirical claims.

---

## Feature Dependencies

```
Ed25519 keypair generation
  → NFC HCE exchange (need own public key to send)
    → ECDH key agreement (need both public keys)
      → XChaCha20-Poly1305 encryption (need derived shared key)
        → DHT publish (need encrypted record)
          → DHT resolve (counterparty has done the above)
            → Contact preview (need decrypted fields)
              → Save to SQLite (user confirms preview)
                → Contact list (saved contacts exist)
                  → Key verification / "last verified" refresh

Profile setup (own fields)
  → Field selection UI (need fields to select from)
    → NFC HCE exchange (need something to share)

QR code display
  → QR async handshake polling (Alice polls for Bob's response)
    → DHT resolve (same decrypt/preview/save flow as NFC)

BIP-39 mnemonic display
  → HKDF seed reconstruction (seed is the recovery path)
    → Key restore on new device (future scope, but dependency exists)

TTL-based record expiry
  → "Expires in Xh" UI label (user needs to know)
    → Manual refresh action (contact list item → re-resolve from DHT)

Public mode (opt-in)
  → Field selection (user chooses what goes public)
    → Plaintext DHT publish (different record type)
      → Public profile resolve (anyone with public key can read)
```

---

## MVP Recommendation

Prioritize (must ship for the core loop to work):

1. Ed25519 keypair generation + Keystore storage — identity foundation
2. BIP-39 mnemonic display — recovery and trust signal at onboarding
3. Profile setup with field selection — own card, choosable fields
4. NFC HCE bidirectional key exchange — the demo moment
5. ECDH + encryption + DHT publish/resolve — the actual exchange
6. Contact preview screen with save/reject — consent flow
7. Contact list — find people tapped later
8. QR code + async handshake — NFC fallback, essential for real-world reliability
9. Error/failure UX for NFC — silent failures kill NFC apps

Defer to v0.2+ (already documented in PROJECT.md as Out of Scope):
- Multi-context profiles (work vs personal HKDF derivation)
- Forward secrecy via ephemeral keys
- Background auto-republish for public mode
- Contact staleness indicators (show expiry, warn — consider pulling this into MVP)
- Export as vCard
- NFC tag programming (sticker write)

One reconsideration: "Expires in Xh" label and manual refresh belong in MVP. A contact that silently vanishes after 24h with no UI indicator is a confusing failure mode that will generate user reports. The complexity is Low.

---

## Competitive Landscape Notes

**Mainstream NFC card apps (Popl, Linq, HiHello, Dot, Blinq):**
- All require accounts (email or social login)
- All store contact data on vendor servers
- All offer analytics (who viewed your card, when)
- Exchange model: person A has the "card," person B receives it — unidirectional
- Monetization via team/enterprise subscriptions
- PKTap's bidirectional, accountless, encrypted model is categorically different

**Decentralized identity apps (Nostr-based, DID-based):**
- Most use QR codes or copy-paste for key exchange — NFC tap is rare
- Contact follow/mute model (social graph) not address book model
- Key backup via nsec/mnemonic is normalized in this space — users expect it
- Discovery (finding people) is a solved problem for social, unsolved for private exchange
- PKTap's DHT rendezvous is novel: no discovery needed, just physical proximity

**Signal-like patterns applicable to PKTap:**
- "Safety numbers" / key verification is a trust primitive users understand
- Ephemeral messages / disappearing content (TTL expiry maps to this mental model)
- No phone number required is a strong positive signal for this audience
- "Sealed sender" / metadata minimization is valued — PKTap's deterministic address is opaque to observers

**Confidence:** MEDIUM — based on publicly documented features of these apps through mid-2025. Feature sets evolve; verify specific claims before using in marketing copy.

---

## Sources

- Training data knowledge of Popl, Linq, HiHello, Dot, Blinq feature sets (through August 2025) — MEDIUM confidence
- PROJECT.md constraints and decisions — HIGH confidence (first-party)
- Signal UX patterns (key verification, disappearing messages, safety numbers) — HIGH confidence (well-documented)
- Mainline DHT / Pkarr record constraints (1000 bytes) — HIGH confidence (from PROJECT.md)
- Nostr contact/identity UX patterns — MEDIUM confidence (training data through August 2025)
- Web search unavailable — no live verification performed
