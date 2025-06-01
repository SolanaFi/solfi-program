use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use solana_program::{
    instruction::Instruction,
    program::invoke_signed,
};
use solana_program::bpf_loader_upgradeable;
use borsh::{BorshDeserialize};

declare_id!("Ho5GQXQ7gUpb7d6uCoobVX11oiv8PefHR2yeu3iMtM9d");

pub const SEED_TOKEN_ACCOUNT_PDA: &[u8] = b"token_account_pda";

#[error_code]
pub enum ErrorCode {
    InsufficientPoolBalance,
    UnauthorizedAccess,
    InvalidProgramData,
}

#[program]
pub mod solana_swap_pool {
    use super::*;
  
    pub fn initialize_token_account_pda(
        ctx: Context<InitializeTokenAccountPda>,
    ) -> Result<()> {
        verify_upgrade_authority(&ctx.accounts.authority, &ctx.accounts.program_data)?;

        ctx.accounts.token_account_pda.token_count = 0;
        msg!("Event: TokenAccountPda initialized successfully [pda={}]", 
            ctx.accounts.token_account_pda.key());
        Ok(())
    }

    pub fn deposit_to_pool(
        ctx: Context<DepositToPool>,
        amount: u64
    ) -> Result<()> {
        msg!("The deposit operation starts");
        
        let transfer_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.from_token_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                transfer_accounts,
            ),
            amount,
        )?;

        msg!("Event: Deposit completed [user={}, amount={}]", 
            ctx.accounts.user.key(),
            amount);
        Ok(())
    }

    pub fn swap_from_pool_dev(
        ctx: Context<SwapFromPoolDev>,
        amount: u64,
    ) -> Result<()> {
        msg!("The swap operation from pool starts");

        verify_upgrade_authority(&ctx.accounts.authority, &ctx.accounts.program_data)?;

        let pool_balance = ctx.accounts.pool_token_account.amount;
        require!(
            pool_balance >= amount,
            ErrorCode::InsufficientPoolBalance
        );
        msg!("pool_balance: {}", pool_balance);
        let bump = ctx.bumps.token_account_pda;
        let seeds = &[
            SEED_TOKEN_ACCOUNT_PDA,
            &[bump],
        ];
        let signer_seeds = &[&seeds[..]];
        msg!("signer_seeds: {:?}", signer_seeds);

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.pool_token_account.to_account_info(),     
                    to: ctx.accounts.recipient_token_account.to_account_info(),       
                    authority: ctx.accounts.token_account_pda.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;

        msg!("Event: Swap from pool completed [authority={}, amount={}, recipient={}]",
            ctx.accounts.authority.key(),
            amount,
            ctx.accounts.recipient_token_account.key());

        Ok(())
    }

}


#[derive(Accounts)]
pub struct InitializeTokenAccountPda<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init_if_needed,
        payer = authority,
        seeds = [SEED_TOKEN_ACCOUNT_PDA],
        bump,
        space = 8 + TokenAccountPda::INIT_SPACE
    )]
    pub token_account_pda: Account<'info, TokenAccountPda>,

    #[account(
        constraint = program_data.owner == &bpf_loader_upgradeable::id(),
        constraint = program_data.data_len() >= 13 + 32
    )]
    pub program_data: AccountInfo<'info>,

    pub program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositToPool<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = user_token_account.owner == user.key()
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub from_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct SwapFromPoolDev<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        seeds = [SEED_TOKEN_ACCOUNT_PDA],
        bump
    )]
    pub token_account_pda: Account<'info, TokenAccountPda>,
    
    #[account(
        mut,
        constraint = pool_token_account.owner == token_account_pda.key()
    )]
    pub pool_token_account: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub recipient_token_account: Account<'info, TokenAccount>,

    #[account(
        constraint = program_data.owner == &bpf_loader_upgradeable::id(),
        constraint = program_data.data_len() >= 13 + 32
    )]
    pub program_data: AccountInfo<'info>,

    pub program: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(Default, InitSpace)]
pub struct TokenAccountPda {
    pub token_count: u64,
}

pub fn verify_upgrade_authority(
    authority: &Signer,
    program_data: &AccountInfo,
) -> Result<()> {
    let program_data_data = program_data.try_borrow_data()?;
    let upgrade_authority_offset = 13;

    require!(
        program_data_data.len() >= upgrade_authority_offset + 32,
        ErrorCode::InvalidProgramData
    );

    let upgrade_authority = Pubkey::try_from_slice(
        &program_data_data[upgrade_authority_offset..upgrade_authority_offset + 32]
    )?;

    require!(
        authority.key() == upgrade_authority,
        ErrorCode::UnauthorizedAccess
    );

    Ok(())
}
