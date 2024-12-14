use anchor_lang::prelude::*;
use pyth_sdk_solana::load_price_feed_from_account_info;

declare_id!("DR85urM1zGQhEA5b9MorTjC3FyTacXEgPY9jfmnMt9JX");

pub const PYTH_SOL_USD_DEVNET: &str = "J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix";

#[program]
pub mod weak_hands {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let lock_account = &mut ctx.accounts.lock_account;
        lock_account.owner = ctx.accounts.user.key();
        lock_account.amount = 0;
        lock_account.target_date = 0;
        lock_account.target_price = 0;
        lock_account.parameters_set = false;
        lock_account.withdrawn = false;
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        // Check withdrawal status before any modifications
        require!(!ctx.accounts.lock_account.withdrawn, ErrorCode::AlreadyWithdrawn);
    
        // Perform transfer
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.user.to_account_info(),
                    to: ctx.accounts.lock_account.to_account_info(),
                },
            ),
            amount,
        )?;
    
        // After transfer completes, then get mutable reference and update state
        let lock_account = &mut ctx.accounts.lock_account;
        lock_account.amount += amount;
        
        emit!(DepositEvent {
            user: ctx.accounts.user.key(),
            amount,
            new_total: lock_account.amount,
        });
        
        Ok(())
    }

    pub fn set_parameters(ctx: Context<SetParameters>, target_date: i64, target_price_usd: u64) -> Result<()> {
        let lock_account = &mut ctx.accounts.lock_account;
        
        require!(lock_account.amount > 0, ErrorCode::NoDeposit);
        require!(!lock_account.parameters_set, ErrorCode::ParametersAlreadySet);
        require!(target_date > Clock::get()?.unix_timestamp, ErrorCode::InvalidDate);
        require!(target_price_usd > 0, ErrorCode::InvalidPrice);

        lock_account.target_date = target_date;
        lock_account.target_price = target_price_usd * 100_000_000; // Convert to price feed format
        lock_account.parameters_set = true;

        emit!(ParametersSetEvent {
            user: ctx.accounts.user.key(),
            target_date,
            target_price: target_price_usd,
        });

        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        let lock_account = &mut ctx.accounts.lock_account;
        
        require!(
            ctx.accounts.price_feed.key().to_string() == PYTH_SOL_USD_DEVNET,
            ErrorCode::InvalidPriceFeed
        );
        
        let price_feed = load_price_feed_from_account_info(&ctx.accounts.price_feed.to_account_info())
            .map_err(|_| ErrorCode::PriceFeedError)?;
        
        let price = price_feed.get_price_unchecked();
        require!(price.price > 0, ErrorCode::InvalidPrice);
        
        require!(lock_account.amount > 0, ErrorCode::NoDeposit);
        require!(lock_account.parameters_set, ErrorCode::ParametersNotSet);
        require!(!lock_account.withdrawn, ErrorCode::AlreadyWithdrawn);
        
        let current_time = Clock::get()?.unix_timestamp;
        require!(
            current_time >= lock_account.target_date || 
            (price.price as u64) >= lock_account.target_price,
            ErrorCode::CannotWithdraw
        );

        let amount = lock_account.amount;
        lock_account.withdrawn = true;
        lock_account.amount = 0;

        **ctx.accounts.lock_account.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.user.to_account_info().try_borrow_mut_lamports()? += amount;

        emit!(WithdrawEvent {
            user: ctx.accounts.user.key(),
            amount,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + LockAccount::SIZE)]
    pub lock_account: Account<'info, LockAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub lock_account: Account<'info, LockAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetParameters<'info> {
    #[account(mut)]
    pub lock_account: Account<'info, LockAccount>,
    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub lock_account: Account<'info, LockAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: This is the Pyth price feed account for SOL/USD that we manually verify in the withdraw instruction
    pub price_feed: AccountInfo<'info>,
}

#[account]
pub struct LockAccount {
    pub owner: Pubkey,
    pub amount: u64,
    pub target_date: i64,
    pub target_price: u64,
    pub parameters_set: bool,
    pub withdrawn: bool,
}

impl LockAccount {
    pub const SIZE: usize = 32 + 8 + 8 + 8 + 1 + 1;
}

#[event]
pub struct DepositEvent {
    pub user: Pubkey,
    pub amount: u64,
    pub new_total: u64,
}

#[event]
pub struct ParametersSetEvent {
    pub user: Pubkey,
    pub target_date: i64,
    pub target_price: u64,
}

#[event]
pub struct WithdrawEvent {
    pub user: Pubkey,
    pub amount: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("No deposit found")]
    NoDeposit,
    #[msg("Parameters already set")]
    ParametersAlreadySet,
    #[msg("Parameters not set")]
    ParametersNotSet,
    #[msg("Invalid date")]
    InvalidDate,
    #[msg("Invalid price")]
    InvalidPrice,
    #[msg("Already withdrawn")]
    AlreadyWithdrawn,
    #[msg("Cannot withdraw yet")]
    CannotWithdraw,
    #[msg("Invalid price feed account")]
    InvalidPriceFeed,
    #[msg("Error getting price from feed")]
    PriceFeedError,
}