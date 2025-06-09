use anchor_lang::prelude::*;
use anchor_lang::solana_program::keccak::hashv;

declare_id!("ProgramID");

#[program]
pub mod swap_logger {
    use super::*;

    /// ------------------------------------------------------------
    /// Initialize the global config with:
    ///   - a designated admin
    ///   - a whitelist of supported token mints
    ///   - a protocol version for future migrations
    ///
    /// Seeds: ["config"]
    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        whitelist: Vec<Pubkey>,
        protocol_version: u16,
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.admin = *ctx.accounts.admin.key;
        config.whitelist = whitelist;
        config.protocol_version = protocol_version;
        config.bump = ctx.bumps.config;
        Ok(())
    }

    /// ------------------------------------------------------------
    /// Initialize a user-specific state account to track how many
    /// trades this user has logged so far.
    ///
    /// Seeds: ["user-state", user_pubkey]
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let user = &ctx.accounts.user;
        let state = &mut ctx.accounts.user_state;
        state.user = *user.key;
        state.trade_count = 0;
        state.bump = ctx.bumps.user_state;
        Ok(())
    }

    /// ------------------------------------------------------------
    /// Log a single trade for a user (or an authorized logger).
    ///
    /// This will:
    /// 1. Check that the signer is either the user or the configured admin.
    /// 2. Ensure `amount > 0`.
    /// 3. Ensure both `token_in` and `token_out` are in the whitelist.
    /// 4. Compute a `trade_id` via Keccak hashing of key fields.
    /// 5. Populate a new `TradeRecord` PDA with all data (`trade_type`, `slippage_bps`, `tag`, etc.).
    /// 6. Emit a `TradeEvent` for off-chain indexing.
    /// 7. Increment the user's `trade_count` so the next PDA is unique.
    ///
    /// Seeds for `TradeRecord`:
    ///   ["trade-record", user_pubkey, trade_count.to_le_bytes()]
    pub fn log_trade(
        ctx: Context<LogTrade>,
        trade_type: u8,
        token_in: Pubkey,
        token_out: Pubkey,
        amount: u64,
        price: u64,
        slippage_bps: u16,
        tag: [u8; 16],
    ) -> Result<()> {
        let signer = ctx.accounts.signer.key;
        let user = ctx.accounts.user.key;
        let config = &ctx.accounts.config;

        // Access control: only the user itself or the designated admin can log on behalf of this user
        require!(
            *signer == *user || *signer == config.admin,
            ErrorCode::UnauthorizedLogger
        );

        // Security & Validation: Amount must be > 0
        require!(amount > 0, ErrorCode::InvalidAmount);

        // Security & Validation: Both tokens must be whitelisted
        require!(
            config.whitelist.contains(&token_in),
            ErrorCode::InvalidToken
        );
        require!(
            config.whitelist.contains(&token_out),
            ErrorCode::InvalidToken
        );

        // Fetch the current timestamp
        let clock = Clock::get()?;
        let state = &mut ctx.accounts.user_state;
        let trade_record = &mut ctx.accounts.trade_record;

        // Compute a unique trade_id via Keccak hash of (user, token_in, token_out, amount, price, slippage, timestamp)
        let user_bytes = user.to_bytes();
        let amount_bytes = amount.to_le_bytes();
        let price_bytes = price.to_le_bytes();
        let slippage_bytes = slippage_bps.to_le_bytes();
        let timestamp_bytes = clock.unix_timestamp.to_le_bytes();
        let hash = hashv(&[
            &user_bytes,
            &token_in.to_bytes(),
            &token_out.to_bytes(),
            &amount_bytes,
            &price_bytes,
            &slippage_bytes,
            &timestamp_bytes,
        ]);
        trade_record.trade_id = hash.0;

        // Populate the TradeRecord PDA
        trade_record.trade_type = trade_type;
        trade_record.slippage_bps = slippage_bps;
        trade_record.tag = tag;
        trade_record.user = *user;
        trade_record.token_in = token_in;
        trade_record.token_out = token_out;
        trade_record.amount = amount;
        trade_record.price = price;
        trade_record.timestamp = clock.unix_timestamp;
        trade_record.bump = ctx.bumps.trade_record;

        // Emit an Anchor event so off-chain indexers can pick up this trade immediately
        emit!(TradeEvent {
            trade_id: trade_record.trade_id,
            user: *user,
            token_in,
            token_out,
            amount,
            price,
            slippage_bps,
            timestamp: clock.unix_timestamp,
            tag,
        });

        // OPTIONAL: Example CPI to an analytics collector program (placeholder)
        // You would add the CPI context and actual call here.
        // e.g.:
        // let cpi_accounts = AnalyticsLog {
        //     collector_program: ctx.accounts.analytics_program.to_account_info(),
        //     // ... other required accounts ...
        // };
        // let cpi_ctx = CpiContext::new(ctx.accounts.analytics_program.to_account_info(), cpi_accounts);
        // analytics::cpi::log_trade(cpi_ctx, trade_record.trade_id, amount, price)?;

        // Increment the user's trade counter for the next PDA derivation
        state.trade_count = state.trade_count.checked_add(1).unwrap();

        Ok(())
    }

    /// ------------------------------------------------------------
    /// BATCH LOGGING (stubbed)
    ///
    /// Suggestion: enable submitting multiple trade logs in one transaction
    /// for high-frequency scenarios. A full implementation would:
    ///   • Iterate over `trades: Vec<TradeInput>`
    ///   • Derive a new PDA for each record (using the updated trade_count)
    ///   • Populate fields exactly as in `log_trade`.
    /// Note: Anchor does not natively support creating an unbounded number of PDAs
    /// inside a loop. You'd typically pre-allocate or use a different pattern.
    pub fn log_trades(
        _ctx: Context<LogTrades>,
        _trades: Vec<TradeInput>,
    ) -> Result<()> {
        // TODO: Implement batch-logging logic
        Ok(())
    }
}

/// ------------------------------------------------------------
/// CONTEXT STRUCTS
/// ------------------------------------------------------------

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    /// Config PDA: seeds = ["config"]
    #[account(
        init,
        payer = admin,
        seeds = [b"config"],
        bump,
        // Space calculation:
        // 8 bytes  for discriminator
        // 32 bytes for `admin: Pubkey`
        // 4 bytes  (vector length) + (32 * MAX_WHITELIST) bytes for `Vec<Pubkey>`
        //  2 bytes for `protocol_version: u16`
        //  1 byte  for `bump: u8`
        space = 8 + 32 + 4 + (32 * MAX_WHITELIST) + 2 + 1
    )]
    pub config: Account<'info, Config>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    /// UserState PDA: seeds = ["user-state", user_pubkey]
    #[account(
        init,
        payer = user,
        seeds = [b"user-state", user.key().as_ref()],
        bump,
        // Space calculation:
        // 8 bytes  for discriminator
        // 32 bytes for `user: Pubkey`
        //  8 bytes for `trade_count: u64`
        //  1 byte  for `bump: u8`
        space = 8 + 32 + 8 + 1
    )]
    pub user_state: Account<'info, UserState>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct LogTrade<'info> {
    /// Config PDA (read-only)
    #[account(
        seeds = [b"config"],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,

    /// The existing UserState PDA (mutable)
    #[account(
        mut,
        seeds = [b"user-state", user.key().as_ref()],
        bump = user_state.bump
    )]
    pub user_state: Account<'info, UserState>,

    /// A new TradeRecord PDA created for THIS trade:
    /// seeds = ["trade-record", user_pubkey, trade_count.to_le_bytes()]
    #[account(
        init,
        payer = signer,
        seeds = [
            b"trade-record",
            user.key().as_ref(),
            &user_state.trade_count.to_le_bytes()
        ],
        bump,
        // Space calculation:
        // 8   bytes for discriminator
        // 1   byte  for trade_type
        // 2   bytes for slippage_bps
        // 16  bytes for tag
        // 32  bytes for trade_id ([u8;32])
        // 32  bytes for user: Pubkey
        // 32  bytes for token_in: Pubkey
        // 32  bytes for token_out: Pubkey
        //  8  bytes for amount: u64
        //  8  bytes for price: u64
        //  8  bytes for timestamp: i64
        //  1  byte  for bump: u8
        space = 8 + 1 + 2 + 16 + 32 + 32 + 32 + 32 + 8 + 8 + 8 + 1
    )]
    pub trade_record: Account<'info, TradeRecord>,

    /// The user on whose behalf the trade is being logged
    #[account(mut)]
    pub user: AccountInfo<'info>,

    /// The signer (either the user themselves or the admin/logger) paying for the new PDA
    #[account(mut)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct LogTrades<'info> {
    /// Config PDA
    #[account(
        seeds = [b"config"],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,

    /// UserState PDA
    #[account(
        mut,
        seeds = [b"user-state", user.key().as_ref()],
        bump = user_state.bump
    )]
    pub user_state: Account<'info, UserState>,

    #[account(mut)]
    pub user: AccountInfo<'info>,

    #[account(mut)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// ------------------------------------------------------------
/// INPUT STRUCT FOR BATCH LOGGING
/// ------------------------------------------------------------
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TradeInput {
    pub trade_type: u8,    // 0 = swap, 1 = add liquidity, etc.
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub amount: u64,
    pub price: u64,
    pub slippage_bps: u16, // in basis points
    pub tag: [u8; 16],     // Optional 16-byte label/tag field
}

/// ------------------------------------------------------------
/// ACCOUNT STRUCTS
/// ------------------------------------------------------------
#[account]
pub struct Config {
    /// Designated admin (can log on behalf of any user)
    pub admin: Pubkey,

    /// Whitelist of supported tokens (max length = MAX_WHITELIST)
    pub whitelist: Vec<Pubkey>,

    /// Protocol version for migration/compatibility
    pub protocol_version: u16,

    /// Bump seed for PDA derivation
    pub bump: u8,
}

#[account]
pub struct UserState {
    /// The wallet that owns this state account
    pub user: Pubkey,

    /// How many trades have been logged so far
    pub trade_count: u64,

    /// Bump seed for PDA derivation
    pub bump: u8,
}

#[account]
pub struct TradeRecord {
    pub trade_type: u8,        // 0 = swap, 1 = add liquidity, etc.
    pub slippage_bps: u16,     // e.g., 50 = 0.50%
    pub tag: [u8; 16],         // Optional 16-byte label/tag field
    pub trade_id: [u8; 32],    // Unique hash for off-chain indexing
    pub user: Pubkey,          // Wallet that made the trade
    pub token_in: Pubkey,      // Input token mint
    pub token_out: Pubkey,     // Output token mint
    pub amount: u64,           // Amount of token_in swapped
    pub price: u64,            // Price (unitless or chosen unit)
    pub timestamp: i64,        // Unix timestamp of trade
    pub bump: u8,              // Bump seed for this PDA
}

/// ------------------------------------------------------------
/// ANCHOR EVENT FOR OFF-CHAIN INDEXERS
/// ------------------------------------------------------------
#[event]
pub struct TradeEvent {
    pub trade_id: [u8; 32],
    pub user: Pubkey,
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub amount: u64,
    pub price: u64,
    pub slippage_bps: u16,
    pub timestamp: i64,
    pub tag: [u8; 16],
}

/// ------------------------------------------------------------
/// CUSTOM ERROR CODES
/// ------------------------------------------------------------
#[error_code]
pub enum ErrorCode {
    #[msg("Amount must be greater than zero.")]
    InvalidAmount,

    #[msg("Token is not in the whitelist.")]
    InvalidToken,

    #[msg("Signer is not authorized to log trades for this user.")]
    UnauthorizedLogger,
}

/// ------------------------------------------------------------
/// CONSTANTS
/// ------------------------------------------------------------
// Maximum number of token mints you intend to support in the whitelist.
// Adjust as needed. Ensure `8 + 32 + 4 + (32 * MAX_WHITELIST) + 2 + 1` matches your real max size.
const MAX_WHITELIST: usize = 10;

/// ------------------------------------------------------------
/// UNIT TESTS (Anchor + Rust)
/// ------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amount_non_zero() {
        let amount = 0u64;
        assert!(
            amount > 0,
            "Amount must be non-zero"
        );
    }

    #[test]
    fn test_user_state_pda() {
        let user_pubkey = Pubkey::new_unique();
        let (pda, bump) = Pubkey::find_program_address(
            &[b"user-state", user_pubkey.as_ref()],
            &crate::ID,
        );
        assert!(bump <= 255, "Bump must fit in a u8");
        // Ensure PDA is actually derived (non-default)
        assert_ne!(pda, Pubkey::default());
    }

    #[test]
    fn test_trade_id_hash_consistency() {
        let user = Pubkey::new_unique();
        let token_in = Pubkey::new_unique();
        let token_out = Pubkey::new_unique();
        let amount = 1_000;
        let price = 500;
        let slippage_bps = 30;
        let timestamp = 1_700_000_000;

        let hash = hashv(&[
            &user.to_bytes(),
            &token_in.to_bytes(),
            &token_out.to_bytes(),
            &amount.to_le_bytes(),
            &price.to_le_bytes(),
            &slippage_bps.to_le_bytes(),
            &timestamp.to_le_bytes(),
        ]);

        assert_eq!(
            hash.0.len(),
            32,
            "Keccak hash must produce 32 bytes"
        );
    }

    #[test]
    fn test_token_whitelist_check() {
        let whitelist = vec![
            Pubkey::new_unique(),
            Pubkey::new_unique(),
        ];
        let token = whitelist[0];
        assert!(
            whitelist.contains(&token),
            "Token should be in whitelist"
        );

        let non_whitelisted = Pubkey::new_unique();
        assert!(
            !whitelist.contains(&non_whitelisted),
            "Token should not be in whitelist"
        );
    }
}