use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock::Clock;

declare_id!("prge4tp2p2R6DyYXtkyRCHJbarSAaMzEgk9u7yRFHJh");

#[program]
pub mod paywall_program {
    use anchor_lang::system_program::{transfer, Transfer};

    use super::*;

    pub fn initialize_config(ctx: Context<InitializeConfig>, fees_address: Pubkey, min_lamports_fee: u64, percentage_fee_sol: u64, creation_lamports_cost: u64) -> Result<()> {
        msg!("Initializing program config: {:?}", ctx.program_id);

        if percentage_fee_sol > 100 {
            return Err(ErrorCode::InvalidPercentageFee.into());
        }

        let program_config = &mut ctx.accounts.program_config;
        program_config.authority_address = *ctx.accounts.authority_address.key;
        program_config.fees_address = fees_address;
        program_config.min_lamports_fee = min_lamports_fee;
        program_config.percentage_fee = percentage_fee_sol;
        program_config.bump = ctx.bumps.program_config;
        program_config.creation_lamports_cost = creation_lamports_cost;
        msg!("Program config initialized");

        Ok(())
    }

    pub fn update_fees(ctx: Context<UpdateFees>, fees_address: Pubkey, min_lamports_fee: u64, percentage_fee_sol: u64, creation_lamports_cost: u64) -> Result<()> {        
        if percentage_fee_sol > 100 {
            return Err(ErrorCode::InvalidPercentageFee.into());
        }

        let program_config = &mut ctx.accounts.program_config;
        program_config.min_lamports_fee = min_lamports_fee;
        program_config.percentage_fee = percentage_fee_sol;
        program_config.fees_address = fees_address;
        program_config.creation_lamports_cost = creation_lamports_cost;

        msg!("Updating fees: min_lamports_fee: {}, percentage_fee_sol: {}, fees_address: {}, creation_lamports_cost: {}", min_lamports_fee, percentage_fee_sol, fees_address.to_string(), creation_lamports_cost);

        Ok(())
    }

    pub fn update_authority(ctx: Context<UpdateAuthority>, authority_address: Pubkey) -> Result<()> {
        let program_config = &mut ctx.accounts.program_config;
        program_config.authority_address = authority_address;

        msg!("Updating authority: {:?}", authority_address.to_string());

        Ok(())
    }

    pub fn create_paywall(ctx: Context<CreatePaywall>, paywall_id: String, max_mint_quantity: u64, lamports_price: u64) -> Result<()> {
        if paywall_id.len() < 1 || paywall_id.len() > 50 {
            return Err(ErrorCode::InvalidPaywallId.into());
        }
        
        let paywall = &mut ctx.accounts.paywall;
        paywall.id = paywall_id;
        paywall.max_mint_quantity = max_mint_quantity;
        paywall.minted_quantity = 0;
        paywall.creator_address = *ctx.accounts.creator_address.key;
        paywall.lamports_price = lamports_price;
        paywall.bump = ctx.bumps.paywall;
        paywall.created_at = Clock::get()?.unix_timestamp;

        if ctx.accounts.program_config.creation_lamports_cost > 0 {
            let fee_transfer = Transfer {
                from: ctx.accounts.creator_address.to_account_info(),
                to: ctx.accounts.fees_address.to_account_info(),
            };
            let fee_transfer_ctx = CpiContext::new(ctx.accounts.system_program.to_account_info(), fee_transfer);
            transfer(fee_transfer_ctx, ctx.accounts.program_config.creation_lamports_cost)?;

            msg!("Fee lamports creation amount transferred: {}", ctx.accounts.program_config.creation_lamports_cost);

        }

        msg!("Paywall created with id: {}", paywall.id);
        msg!("Paywall address PDA: {}", paywall.key().to_string());
        msg!("Paywall creator address: {}", paywall.creator_address.to_string());
        msg!("Paywall lamports price: {}", paywall.lamports_price);

        emit!(PaywallCreated {
            paywall_id: paywall.id.clone(),
            creator_address: paywall.creator_address,
            bump: paywall.bump,
        });

        Ok(())
    }

    pub fn update_paywall(ctx: Context<UpdatePaywall>, paywall_id: String, max_mint_quantity: u64, lamports_price: u64) -> Result<()> {
        let paywall = &mut ctx.accounts.paywall;
        paywall.max_mint_quantity = max_mint_quantity;
        paywall.lamports_price = lamports_price;

        msg!("Paywall updated with id: {}", paywall_id);
        msg!("Paywall max mint quantity: {}", paywall.max_mint_quantity);
        msg!("Paywall lamports price: {}", paywall.lamports_price);

        emit!(PaywallUpdated {
            paywall_id,
            creator_address: paywall.creator_address,
            max_mint_quantity: paywall.max_mint_quantity,
            lamports_price: paywall.lamports_price,
        });

        Ok(())
    }

    pub fn mint_paywall(ctx: Context<MintPaywall>, paywall_id: String) -> Result<()> {
        let paywall = &mut ctx.accounts.paywall;
        paywall.minted_quantity += 1;

        if paywall.max_mint_quantity != 0 && paywall.minted_quantity > paywall.max_mint_quantity {
            return Err(ErrorCode::MaxMintQuantityReached.into());
        }

        // That means it's not initialized yet
        // if paywall.id != paywall_id {
        //     paywall.id = paywall_id;
        //     paywall.creator_address = *ctx.accounts.creator_address.key;

        //     msg!("Paywall initialized during mint with id: {}", paywall_id);
        // }

        let payment = &mut ctx.accounts.payment;
        payment.paywall_id = paywall_id;
        payment.payer_address = *ctx.accounts.user.key;
        payment.lamports_paid = paywall.lamports_price;
        payment.bump = ctx.bumps.payment;
        payment.created_at = Clock::get()?.unix_timestamp;

        let program_config = &mut ctx.accounts.program_config;
        // let authority_address = program_config.authority_address;
        // let fees_address = program_config.fees_address;
        let min_lamports_fee = program_config.min_lamports_fee;
        let percentage_fee_sol = program_config.percentage_fee;

        let mut fee_lamports_amount = payment.lamports_paid.checked_mul(percentage_fee_sol)
            .ok_or(ErrorCode::NumericalOverflow)?
            .checked_div(100)
            .ok_or(ErrorCode::NumericalOverflow)?;

        if fee_lamports_amount < min_lamports_fee {
            fee_lamports_amount = min_lamports_fee;
        }

        msg!("Fee lamports amount: {}", fee_lamports_amount);
        let fee_transfer = Transfer {
            from: ctx.accounts.user.to_account_info(),
            to: ctx.accounts.fees_address.to_account_info(),
        };
        let fee_transfer_ctx = CpiContext::new(ctx.accounts.system_program.to_account_info(), fee_transfer);
        transfer(fee_transfer_ctx, fee_lamports_amount)?;

        msg!("Creator transfer lamports amount: {}", payment.lamports_paid.checked_sub(fee_lamports_amount).unwrap());
        let creator_transfer = Transfer {
            from: ctx.accounts.user.to_account_info(),
            to: ctx.accounts.creator_address.to_account_info()
        };
        let creator_transfer_ctx = CpiContext::new(ctx.accounts.system_program.to_account_info(), creator_transfer);
        transfer(creator_transfer_ctx, payment.lamports_paid.checked_sub(fee_lamports_amount)
            .ok_or(ErrorCode::NumericalOverflow)?)?;

        msg!("Mint for paywall {} for total lamports paid: {}", paywall.id, payment.lamports_paid);

        emit!(PaywallMinted {
            paywall_id: paywall.id.clone(),
            creator_address: paywall.creator_address,
            payer_address: payment.payer_address,
            lamports_paid: payment.lamports_paid,
            bump: payment.bump,
        });

        Ok(())
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid paywall id")]
    InvalidPaywallId,
    #[msg("Max mint quantity reached")]
    MaxMintQuantityReached,
    #[msg("Numerical overflow")]
    NumericalOverflow,
    #[msg("Invalid percentage fee")]
    InvalidPercentageFee,
}

#[event]
pub struct PaywallMinted {
    pub paywall_id: String,
    pub creator_address: Pubkey,
    pub payer_address: Pubkey,
    pub lamports_paid: u64,
    bump: u8,
}

#[event]
pub struct PaywallCreated {
    pub paywall_id: String,
    pub creator_address: Pubkey,
    bump: u8,
}

#[event]
pub struct PaywallUpdated {
    pub paywall_id: String,
    pub creator_address: Pubkey,
    pub max_mint_quantity: u64,
    pub lamports_price: u64,
}

#[account]
#[derive(InitSpace)]
pub struct Paywall {
    #[max_len(24)]
    pub id: String,
    pub max_mint_quantity: u64,
    pub minted_quantity: u64,
    pub creator_address: Pubkey,
    pub lamports_price: u64,
    pub created_at: i64,
    bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Payment {
    #[max_len(24)]
    pub paywall_id: String,
    pub payer_address: Pubkey,
    pub lamports_paid: u64,
    pub created_at: i64,
    bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct ProgramConfig {
    pub authority_address: Pubkey,
    pub fees_address: Pubkey,
    pub min_lamports_fee: u64,
    pub percentage_fee: u64,
    pub creation_lamports_cost: u64,
    bump: u8,
}

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(
        init,
        payer = authority_address,
        space = 8 + ProgramConfig::INIT_SPACE,
        seeds = [b"program_config"],
        bump
    )]
    pub program_config: Account<'info, ProgramConfig>,
    pub system_program: Program<'info, System>,
    #[account(mut)]
    pub authority_address: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateFees<'info> {
    #[account(mut,
        has_one = authority_address,
        seeds = [b"program_config"],
        bump = program_config.bump,
    )]
    pub program_config: Account<'info, ProgramConfig>,
    pub system_program: Program<'info, System>,
    pub authority_address: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateAuthority<'info> {
    #[account(mut,
        has_one = authority_address,
        seeds = [b"program_config"],
        bump = program_config.bump,
    )]
    pub program_config: Account<'info, ProgramConfig>,
    pub system_program: Program<'info, System>,
    pub authority_address: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(paywall_id: String)]
pub struct CreatePaywall<'info> {
    #[account(
        init,
        payer = creator_address,
        space = 8 + Paywall::INIT_SPACE,
        seeds = [b"paywall", creator_address.key().as_ref(), paywall_id.as_bytes()],
        bump
    )]
    pub paywall: Account<'info, Paywall>,
    #[account(
        seeds = [b"program_config"],
        bump = program_config.bump,
        has_one = fees_address
    )]
    pub program_config: Account<'info, ProgramConfig>,
    pub system_program: Program<'info, System>,
    #[account(mut)]
    pub creator_address: Signer<'info>,
    /// CHECK: We will check the fees address in the instruction (? or the constraint)
    #[account(mut)]
    pub fees_address: UncheckedAccount<'info>,
}

#[derive(Accounts)]
#[instruction(paywall_id: String)]
pub struct UpdatePaywall<'info> {
    #[account(
        mut,
        seeds = [b"paywall", creator_address.key().as_ref(), paywall_id.as_bytes()],
        bump,
        has_one = creator_address
    )]
    pub paywall: Account<'info, Paywall>,
    pub system_program: Program<'info, System>,
    pub creator_address: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(paywall_id: String)]
pub struct MintPaywall<'info> {
    #[account(mut,
        seeds = [b"paywall", creator_address.key().as_ref(), paywall_id.as_bytes()],
        bump = paywall.bump,
        has_one = creator_address
    )]
    pub paywall: Account<'info, Paywall>,
    #[account(
        init,
        payer = user,
        space = 8 + Payment::INIT_SPACE,
        seeds = [b"payment", creator_address.key().as_ref(), paywall_id.as_bytes(), user.key().as_ref()],
        bump
    )]
    pub payment: Account<'info, Payment>,
    #[account(
        seeds = [b"program_config"],
        bump = program_config.bump,
        has_one = fees_address
    )]
    pub program_config: Account<'info, ProgramConfig>,
    pub system_program: Program<'info, System>,
    /// CHECK: We will check the creator address in the instruction (? or the constraint)
    #[account(mut)]
    pub creator_address: UncheckedAccount<'info>,
    ///  CHECK: We will check the fees address in the instruction (? or the constraint)
    #[account(mut)]
    pub fees_address: UncheckedAccount<'info>,
    #[account(mut)]
    pub user: Signer<'info>,
}
