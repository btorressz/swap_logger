# swap_logger

A Solana program built with Anchor that provides comprehensive trade logging functionality for DeFi applications. This program allows users and authorized administrators to log swap transactions with detailed metadata while maintaining security through token whitelisting and access controls.

**NOTE: THIS PROJECT WAS DEVELOPED IN SOLANA PLAYGROUND AND WILL BE EXPORTED TO VSCODE**


## ✨ Features

- **Admin-Configurable Whitelist**  
  Only allow trades between whitelisted tokens, settable by an admin during `initializeConfig`.

- **User Trade Tracking**  
  Each user has a unique `UserState` account that tracks how many trades they've logged. This enables PDA derivation and trade history organization.

- **Trade Logging**  
  Each trade is:
  - Validated (non-zero amount, whitelisted tokens)
  - Uniquely identified by a hash (`trade_id`)
  - Stored as a `TradeRecord` PDA derived from the user and their trade count

- **Event Emission**  
  Emits a `TradeEvent` for every trade, making it easy for off-chain indexers, UIs, and analytics engines to consume on-chain activity in real-time.

- **Batch Logging (Stub)**  
  Entry point for future batch logging of multiple trades in a single transaction—scaffolded for high-frequency trading (HFT) use cases.

- **Anchor Unit Tests**  
  Includes Rust-based tests for:
  - PDA derivation
  - Trade ID hashing consistency
  - Whitelist inclusion logic

---

## 📦 Program Accounts

| Account      | Description                                            |
|--------------|--------------------------------------------------------|
| `Config`     | Stores the admin wallet, token whitelist, and protocol version for upgradeability |
| `UserState`  | Tracks individual user's trade count and identity     |
| `TradeRecord`| Stores the details of each trade: token pair, amount, price, timestamp, tag, etc. |
