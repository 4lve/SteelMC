# Signed Chat Implementation Guide for SteelMC

## Overview

This document outlines the complete implementation plan for adding signed chat functionality to SteelMC, based on analysis of decompiled Minecraft code from `~/workspace/Extractor/decompiled`.

## Current State

As of commit `fd0fcf48` (Impl unsigned chat), SteelMC has:

- ✅ Basic packet structures (`SChat`, `CPlayerChat`, `CSystemChat`)
- ✅ Signature cache infrastructure (`signature_cache.rs`)
- ✅ Message validation framework (`message_validator.rs`)
- ✅ Chat broadcasting in `World`
- ✅ Config flag `enforce_secure_chat: false`

However, the current implementation **skips all signature verification** - messages are sent with `message_signature: None` in `handle_chat()` at `steel-core/src/player/mod.rs:141-170`.

---

## Missing Components for Full Signed Chat

### 1. Key Exchange & Management

**Client → Server Packet:** `ServerboundChatSessionUpdatePacket`
- Sends player's public key + sessionId + Mojang signature
- Currently **not implemented** in the codebase
- Decompiled reference: `net/minecraft/network/protocol/game/ServerboundChatSessionUpdatePacket.java`

**Server-side validation:**
- Verify player's public key using Mojang's signature
- Store in player session (`RemoteChatSession`)
- Current login flow (`steel/src/network/login.rs`) only handles connection encryption, not chat key exchange

### 2. Cryptographic Infrastructure

Required RSA signing/verification utilities:
- **Signing algorithm:** SHA256withRSA
- **Signature size:** 256 bytes
- **Key validation:** Using Mojang's public keys from their services

**Decompiled references:**
- `net/minecraft/util/Crypt.java` - Crypto utilities
- `net/minecraft/util/Signer.java` - RSA signing interface
- `net/minecraft/util/SignatureValidator.java` - RSA verification

**Currently missing in SteelMC** - no RSA signing code exists.

### 3. Message Chain Management

**Decompiled classes that need to be ported:**
- `SignedMessageChain.java` - Encoder/decoder with chain validation
- `SignedMessageLink.java` - Tracks sender UUID + sessionId + index
- `SignedMessageBody.java` - Content + timestamp + salt + lastSeen

**Current state in SteelMC:**
- ✅ `signature_cache.rs` - Tracks signatures
- ✅ `message_validator.rs` - Handles acknowledgments
- ❌ **Missing:** Chain validation (sequence numbers, timestamp ordering)
- ❌ **Missing:** Message body signing logic

### 4. Profile Public Key System

**Decompiled references:**
- `ProfilePublicKey.java` - Public key + expiry + Mojang signature
- `ProfileKeyPair.java` - Private + public key pair (client-side)
- `ProfileKeyPairManager.java` - Key lifecycle management

**In SteelMC:**
- Currently **not implemented** at all
- Login flow doesn't exchange/validate chat keys
- No key expiry tracking (keys expire after 24h with 8h grace period)

### 5. Packet Handlers

**Missing serverbound packets:**
- `ServerboundChatSessionUpdatePacket` - Key exchange
- `ServerboundChatAckPacket` - Acknowledgments (validator exists but no handler)
- `ServerboundChatCommandSignedPacket` - Signed commands

**Current `SChat` packet** (`steel-protocol/src/packets/game/s_chat.rs`):
- ✅ Has signature field (256 bytes)
- ✅ Has timestamp, salt
- ✅ Has message_count, acknowledged, checksum
- ⚠️ But no handler validates these fields

### 6. Signature Verification Logic

**Current `Player::handle_chat()`** (`steel-core/src/player/mod.rs:141`):

Currently does:
1. Accept any message
2. Create `CPlayerChat` with `message_signature: None`
3. Broadcast as unsigned

**For signed chat, needs to:**
1. Validate client has sent `ChatSessionUpdate` with valid key
2. Extract signature from `SChat` packet
3. Reconstruct signature data (per `PlayerChatMessage.updateSignature()`)
4. Verify using player's public key (RSA SHA256)
5. Check timestamp ordering (must be >= previous)
6. Check message not expired (5 min server-side)
7. Validate chain link can advance
8. Only then broadcast with signature included

---

## Cryptographic Details

### Algorithms Used by Minecraft

From `net/minecraft/util/Crypt.java`:

- **Asymmetric:** RSA (1024-bit) for key pairs
- **Symmetric:** AES (128-bit) for session encryption (already implemented)
- **Signing:** SHA256withRSA (256-byte signatures)
- **Hashing:** SHA-1 for authentication (already implemented)

### Signature Construction

From `net/minecraft/network/chat/PlayerChatMessage.java`:

The signature covers the following data in order:
1. **Version byte:** `1`
2. **Link data:**
   - Sender UUID (16 bytes)
   - Session ID (16 bytes)
   - Message index (as bytes)
3. **Body data:**
   - Salt (8 bytes)
   - Timestamp as epoch seconds (8 bytes)
   - Message content length (as bytes)
   - Message content (UTF-8 bytes)
   - LastSeen signatures (all previous signature bytes)

### Key Validation

From `net/minecraft/world/entity/player/ProfilePublicKey.java`:

Player public keys must be:
1. Signed by Mojang's services key
2. Not expired (with 8-hour grace period)
3. Not older than the previous key

The signature covers:
- Player UUID
- Key expiration timestamp
- RSA public key bytes

---

## Message Chain System

### Chain Structure

From `net/minecraft/network/chat/SignedMessageChain.java`:

Each player maintains a message chain with:
- **Current link:** `SignedMessageLink` with sender UUID + sessionId + index
- **Last timestamp:** For ordering validation
- **Chain state:** Can be broken if validation fails

### Chain Validation Rules

1. **Signature must exist** (unless unsigned mode)
2. **Key must not be expired** (with grace period)
3. **Timestamp must be >= previous** message
4. **Signature must verify** using player's public key
5. **Message must not be expired:**
   - Server-side: 5 minutes
   - Client-side: 7 minutes (5 + 2 grace)
6. **Index must advance sequentially**

If any check fails, the chain is broken and all subsequent messages are rejected.

---

## Implementation Roadmap

### Phase 1: Infrastructure (No Behavior Change)

**Goal:** Add crypto infrastructure and data structures without changing chat behavior.

#### Tasks:
1. **Create `steel-crypto` crate**
   - Add RSA key pair generation
   - Add SHA256withRSA signing/verification
   - Add PEM encoding/decoding utilities
   - Port relevant functions from `Crypt.java`

2. **Add profile key data structures**
   - Create `steel-core/src/player/profile_key.rs`
   - Implement `ProfilePublicKey` (public key + expiry + signature)
   - Implement `RemoteChatSession` (sessionId + public key)
   - Add optional fields to `Player` struct

3. **Add message chain structures**
   - Create `steel-core/src/player/message_chain.rs`
   - Implement `SignedMessageLink` (sender UUID + sessionId + index)
   - Implement `SignedMessageBody` (content + timestamp + salt + lastSeen)
   - Implement chain state tracking

4. **Add new packet definitions**
   - `steel-protocol/src/packets/game/s_chat_session_update.rs`
   - `steel-protocol/src/packets/game/s_chat_ack.rs`
   - `steel-protocol/src/packets/game/s_chat_command_signed.rs`

**Testing:** Compile successfully, all existing tests pass, unsigned chat still works.

---

### Phase 2: Key Exchange (Optional Signing)

**Goal:** Implement key exchange protocol but don't enforce signatures yet.

#### Tasks:
1. **Implement Mojang key validation**
   - Add Mojang services public key constants
   - Implement `validate_profile_key()` function
   - Add API call to verify with Mojang servers (if needed)

2. **Add `ChatSessionUpdate` packet handler**
   - In `steel/src/network/` or appropriate location
   - Receive client's public key
   - Validate key signature
   - Store in player's `RemoteChatSession`
   - Log success/failure (don't kick yet)

3. **Update login flow**
   - After encryption handshake in `steel/src/network/login.rs`
   - Be ready to receive `ChatSessionUpdate` packet
   - Initialize player's message chain state

4. **Add session tracking to Player**
   - Store `Option<RemoteChatSession>` in `Player` struct
   - Initialize message chain when session received
   - Track last message timestamp and index

**Testing:**
- Server accepts and validates chat session updates
- Keys are stored correctly
- Unsigned chat still works
- Server logs when clients send valid/invalid keys

---

### Phase 3: Signature Verification (With Fallback)

**Goal:** Verify signatures when present, but allow unsigned messages.

#### Tasks:
1. **Implement signature reconstruction**
   - In `steel-core/src/player/message_chain.rs`
   - Function to build signature data from message components
   - Match byte layout from `PlayerChatMessage.updateSignature()`

2. **Implement RSA verification**
   - Use player's public key from their session
   - Verify 256-byte signature against reconstructed data
   - Return clear error messages for debugging

3. **Update `Player::handle_chat()`**
   - Check if player has valid `RemoteChatSession`
   - If signature present, verify it
   - Validate timestamp ordering
   - Validate chain link advancement
   - Check message not expired (5 min)
   - Log verification results
   - **Always accept** message (fallback to unsigned if verification fails)

4. **Update broadcast logic**
   - Include signature in `CPlayerChat` if valid
   - Set `unsigned_content` if server modified message
   - Update signature cache for all recipients

5. **Implement `ChatAck` handler**
   - Process client acknowledgments
   - Update `LastSeenMessagesValidator` state
   - Track which messages clients have seen

**Testing:**
- Modern clients send signed messages that verify correctly
- Signatures are included in broadcasts
- Signature cache works correctly
- Legacy/modified clients can still send unsigned messages
- No players are kicked

---

### Phase 4: Full Enforcement (Production Ready)

**Goal:** Enforce signed chat when `enforce_secure_chat: true` in config.

#### Tasks:
1. **Add enforcement logic**
   - Check `STEEL_CONFIG.enforce_secure_chat` flag
   - If true and signature verification fails:
     - Kick player with appropriate error message
     - Log security violation
   - If false, fall back to unsigned (Phase 3 behavior)

2. **Implement key expiry handling**
   - Check key expiration timestamps
   - Track 8-hour grace period
   - Kick players with expired keys (when enforcing)
   - Log expiry events

3. **Add chain break detection**
   - If message chain breaks (validation fails), mark chain as broken
   - Reject all subsequent messages until chain resets
   - Allow chain reset on reconnect

4. **Implement signed commands**
   - Add handler for `ServerboundChatCommandSignedPacket`
   - Verify command argument signatures
   - Integrate with command execution system

5. **Add comprehensive logging**
   - Log all signature verification attempts
   - Log chain breaks and resets
   - Log key exchanges and expirations
   - Add metrics for monitoring

6. **Security hardening**
   - Add rate limiting for failed verifications
   - Detect and ban signature spam attempts
   - Add replay attack prevention
   - Implement proper error messages matching vanilla

**Testing:**
- Enable enforcement and verify signed messages work
- Verify unsigned messages are rejected when enforcing
- Test key expiration handling
- Test chain break and recovery
- Test with vanilla clients at various versions
- Security testing: replay attacks, expired keys, invalid signatures

---

## Key Files to Create

### New Files

```
steel-crypto/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── rsa.rs              # RSA key pair, signing, verification
    └── validation.rs       # Mojang key validation

steel-core/src/player/
├── profile_key.rs          # ProfilePublicKey, RemoteChatSession
├── message_chain.rs        # SignedMessageLink, SignedMessageBody, chain validation
└── chat_validator.rs       # High-level signature verification logic

steel-protocol/src/packets/game/
├── s_chat_session_update.rs
├── s_chat_ack.rs
└── s_chat_command_signed.rs

steel/src/network/
└── chat_session.rs         # Chat session packet handlers
```

### Files to Modify

```
steel-core/src/player/mod.rs
  - Update Player struct with session and chain fields
  - Rewrite handle_chat() with signature verification

steel/src/network/login.rs
  - Add chat session initialization after encryption

steel-core/src/player/signature_cache.rs
  - Integrate with chain validation

steel-core/src/world/mod.rs
  - Update broadcast logic to include signatures

steel/Cargo.toml
  - Add steel-crypto dependency

Cargo.toml (workspace root)
  - Add steel-crypto to workspace members
```

---

## Configuration

The existing config flag `enforce_secure_chat: false` will control behavior:

- **`false` (default):** Accept both signed and unsigned messages (Phase 3 behavior)
- **`true`:** Require valid signatures, kick players with invalid signatures (Phase 4 behavior)

This allows gradual rollout and testing without breaking existing functionality.

---

## Testing Strategy

### Unit Tests
- RSA signing and verification
- Signature data reconstruction
- Chain link advancement
- Timestamp validation
- Key expiry checks

### Integration Tests
- Full message signing flow
- Key exchange protocol
- Chain break and recovery
- Acknowledgment processing

### Manual Testing
- Test with vanilla Minecraft client
- Test with modified clients (if available)
- Test key expiration behavior
- Test enforcement on/off

### Security Testing
- Replay attack attempts
- Signature tampering
- Expired key usage
- Chain break attempts
- Rate limiting verification

---

## References

All implementation details are based on decompiled Minecraft code in:
`/home/tom/workspace/Extractor/decompiled/source/net/minecraft/`

Key reference files:
- `network/chat/PlayerChatMessage.java` - Complete message structure
- `network/chat/SignedMessageChain.java` - Chain validation
- `network/protocol/game/ServerboundChatSessionUpdatePacket.java` - Key exchange
- `util/Crypt.java` - Cryptographic utilities
- `world/entity/player/ProfilePublicKey.java` - Key management
- `server/network/ServerGamePacketListenerImpl.java:1383-1993` - Server-side handling

---

## Timeline Estimate

- **Phase 1:** 2-3 days (infrastructure)
- **Phase 2:** 2-3 days (key exchange)
- **Phase 3:** 3-4 days (verification with fallback)
- **Phase 4:** 2-3 days (enforcement and hardening)

**Total:** ~2 weeks for full implementation and testing

---

## Security Considerations

1. **Always validate Mojang signatures on public keys** - prevents key spoofing
2. **Enforce timestamp ordering** - prevents replay attacks
3. **Track message chain state** - prevents out-of-order message injection
4. **Rate limit verification failures** - prevents DoS through invalid signatures
5. **Log security events** - enables monitoring and incident response
6. **Use constant-time comparison** where possible - prevents timing attacks
7. **Validate key expiry** - ensures keys can't be used indefinitely

---

## Future Enhancements

- **Message reporting system:** Allow players to report signed messages to moderators
- **Signature verification metrics:** Track verification success rates
- **Advanced chain recovery:** Allow chain reset without full reconnect
- **Custom key management:** Support server-specific signing keys (if needed)
- **Performance optimization:** Signature verification caching

---

## Notes

- The decompiled code shows Minecraft has `SignatureValidator.NO_VALIDATION` for testing purposes
- Current SteelMC implementation already has good foundation with signature cache and validator
- The modular phase approach allows incremental development without breaking existing features
- Config flag allows operators to choose their security posture
