use anchor_lang::{
    prelude::*,
    system_program::{create_account, CreateAccount},
};
use fixed::types::{I80F48, U64F64};

use anchor_spl::{
    token_2022::Token2022,
    token_interface::{Mint, TokenAccount}
};
use spl_tlv_account_resolution::{
    account::ExtraAccountMeta,
    state::ExtraAccountMetaList,
};
use spl_transfer_hook_interface::instruction::{ExecuteInstruction, TransferHookInstruction};

pub const RAYDIUM_POOL_LEN: usize = 1544;
pub const RAYDIUM_POOL_DISCRIMINATOR: [u8; 8] = [247, 237, 227, 245, 215, 195, 222, 70];
pub const SOL_LAMPORTS: u64 = 1_000_000_000; // 1 SOL in lamports

pub mod raydium_mainnet {
    use solana_program::declare_id;
    declare_id!("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK");
}

pub struct CLMMPoolState {
    pub sqrt_price: u128,
    pub token_mint_a: Pubkey,
    pub token_mint_b: Pubkey,
}

pub mod usdc_mint_mainnet {
    use solana_program::declare_id;
    declare_id!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
}

pub mod sol_mint_mainnet {
    use solana_program::declare_id;
    declare_id!("So11111111111111111111111111111111111111112");
}

impl CLMMPoolState {
    pub fn is_inverted(&self) -> bool {
        self.token_mint_a == usdc_mint_mainnet::ID
            || (self.token_mint_a == sol_mint_mainnet::ID
                && self.token_mint_b != usdc_mint_mainnet::ID)
    }

    pub fn get_clmm_price(&self) -> I80F48 {
        if self.is_inverted() {
            let sqrt_price = U64F64::from_bits(self.sqrt_price).to_num::<f64>();
            let inverted_price = sqrt_price * sqrt_price;
            I80F48::from_num(1.0f64 / inverted_price)
        } else {
            let sqrt_price = U64F64::from_bits(self.sqrt_price);
            I80F48::from_num(sqrt_price * sqrt_price)
        }
    }
}


pub fn load_raydium_pool_state<'info>(acc_info: &AccountInfo<'info>) -> Result<CLMMPoolState> {
    let data: &[u8] = &acc_info.data.try_borrow().unwrap();
    require!(
        data[0..8] == RAYDIUM_POOL_DISCRIMINATOR[..],
        TransferHookError::InvalidCLMMOracle
    );
    require!(
        data.len() == RAYDIUM_POOL_LEN,
        TransferHookError::InvalidCLMMOracle
    );
    require!(
        acc_info.owner == &raydium_mainnet::ID,
        TransferHookError::InvalidCLMMOracle
    );

    let price_bytes: &[u8; 16] = &data[253..269].try_into().unwrap();
    let sqrt_price = u128::from_le_bytes(*price_bytes);
    let a: &[u8; 32] = &(&data[73..105]).try_into().unwrap();
    let b: &[u8; 32] = &(&data[105..137]).try_into().unwrap();
    let mint_a = Pubkey::from(*a);
    let mint_b = Pubkey::from(*b);

    Ok(CLMMPoolState {
        sqrt_price,
        token_mint_a: mint_a,
        token_mint_b: mint_b,
    })
}
#[account]
pub struct Game {
    pub last_price: u64,
    pub other_mint: Pubkey,
    pub leverage: u64,
    pub creator: Pubkey,
    pub t22_reserve: Pubkey,
    pub lst_reserve: Pubkey,
    pub sol_reserve: Pubkey,
    pub t22_sol_pool: Pubkey,
    pub lst_sol_pool: Pubkey,
    pub t22_lst_pool: Pubkey,
}

impl Game {
    pub fn calculate_price_ratio(&self, current_price: u64, is_short: bool) -> f64 {
        let price_change = (current_price as f64 - self.last_price as f64) / self.last_price as f64;
        let leveraged_change = price_change * self.leverage as f64;
        
        if is_short {
            1.0 - leveraged_change
        } else {
            1.0 + leveraged_change
        }
    }

    pub fn calculate_payout(&self, amount: u64, current_price: u64, is_short: bool) -> u64 {
        let price_ratio = self.calculate_price_ratio(current_price, is_short);
        (amount as f64 * price_ratio).round() as u64
    }
}
declare_id!("AZR4kEoxHrD879oPU5vLbJnryCHEyrJfiFwmASUXdFqf");

#[program]
pub mod transfer_hook {
    use anchor_spl::token_2022::{mint_to, MintTo};
    use solana_program::{program::{invoke, invoke_signed}, system_instruction};
    use super::*;
    pub fn initialize_game(
        ctx: Context<InitializeGame>,
        leverage: u64,
        amount: u64,
        t22_reserve: Pubkey,
        lst_reserve: Pubkey,
        sol_reserve: Pubkey,
        t22_sol_pool: Pubkey,
        lst_sol_pool: Pubkey,
        t22_lst_pool: Pubkey,
    ) -> Result<()> {
        let game = &mut ctx.accounts.game;
        game.creator = ctx.accounts.payer.key();
        game.last_price = 0; // Initialize with a default value
        game.other_mint = ctx.accounts.other_mint.key();
        game.leverage = leverage;
        game.t22_reserve = t22_reserve;
        game.lst_reserve = lst_reserve;
        game.sol_reserve = sol_reserve;
        game.t22_sol_pool = t22_sol_pool;
        game.lst_sol_pool = lst_sol_pool;
        game.t22_lst_pool = t22_lst_pool;
    
        Ok(())
    }
    pub fn initialize_extra_account_meta_list(
        ctx: Context<InitializeExtraAccountMetaList>,
        leverage: u64,
        amount: u64
    ) -> Result<()> {
        let account_metas = vec![
            ExtraAccountMeta::new_with_pubkey(&ctx.accounts.other_mint.key(), false, false)?,
            ExtraAccountMeta::new_with_pubkey(&ctx.accounts.game.key(), false, true)?,
            ExtraAccountMeta::new_with_pubkey(&ctx.accounts.raydium_clmm.key(), false, false)?,
        ];

        let ix = spl_token_2022::instruction::set_authority(
            &spl_token_2022::ID,
            &ctx.accounts.other_mint.key(),
            Some(&ctx.accounts.game.key()),
            spl_token_2022::instruction::AuthorityType::MintTokens,
            &ctx.accounts.payer.key(),
            &[]
        )?;

        let accounts = vec![
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.other_mint.to_account_info(),
            ctx.accounts.game.to_account_info(),
            ctx.accounts.raydium_clmm.to_account_info(),
        ];
        invoke(&ix, &accounts)?;

        let account_size = ExtraAccountMetaList::size_of(account_metas.len())? as u64;
        let lamports = Rent::get()?.minimum_balance(account_size as usize);

        let mint = ctx.accounts.mint.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"extra-account-metas",
            &mint.as_ref(),
            &[ctx.bumps.extra_account_meta_list],
        ]];

        create_account(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                CreateAccount {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.extra_account_meta_list.to_account_info(),
                },
            )
            .with_signer(signer_seeds),
            lamports,
            account_size,
            ctx.program_id,
        )?;

        let clmm = &ctx.accounts.raydium_clmm;
        let clmm_state = load_raydium_pool_state(clmm)?;
        let unix_timestamp = Clock::get()?.unix_timestamp;

        let game = &mut ctx.accounts.game;
        game.creator = ctx.accounts.payer.key();
        game.last_price = clmm_state.get_clmm_price().to_num::<u64>();
        game.other_mint = ctx.accounts.other_mint.key();
        game.leverage = leverage;
        let initial_liquidity = amount as f64 / 1.0;


        // Transfer initial liquidity to the pool
        let transfer_a = system_instruction::transfer(
            &ctx.accounts.payer.key(),
            &ctx.accounts.game.key(),
            initial_liquidity_a,
        );
        invoke(
            &transfer_a,
            &[
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.game.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        let transfer_b = system_instruction::transfer(
            &ctx.accounts.payer.key(),
            &ctx.accounts.game.key(),
            initial_liquidity_b,
        );
        invoke(
            &transfer_b,
            &[
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.game.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        // Initialize the pool state
        let pool_state = CLMMPoolState {
            sqrt_price: clmm_state.sqrt_price,
            token_mint_a: clmm_state.token_mint_a,
            token_mint_b: clmm_state.token_mint_b,
        };
        let pool = Pool {
            state: pool_state,
            last_price: clmm_state.get_clmm_price().to_num::<u64>(),
            fee_percentage,
        };
        game.pools.push(pool);

        ExtraAccountMetaList::init::<ExecuteInstruction>(
            &mut ctx.accounts.extra_account_meta_list.try_borrow_mut_data()?,
            &account_metas,
        )?;

        Ok(())
    }

    pub fn mint_sol_tokens(ctx: Context<MintSolTokens>, amount: u64, is_short: bool) -> Result<()> {
        let game = &ctx.accounts.game;
        let current_price = load_raydium_pool_state(&ctx.accounts.raydium_clmm)?.get_clmm_price().to_num::<u64>();
        
        let tokens_to_mint = game.calculate_payout(amount, current_price, is_short);
        let signer_seeds: &[&[&[u8]]] = &[&[b"game", ctx.accounts.mint.to_account_info().key.as_ref(), &[ctx.bumps.game]]];

        // Mint tokens 

        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    authority: ctx.accounts.game.to_account_info(),
                    to: ctx.accounts.destination.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                },
                signer_seeds,
            ),
            tokens_to_mint,
        )?;

        let lamports_to_transfer = amount;
        let transfer_instruction = system_instruction::transfer(
            &ctx.accounts.payer.key(),
            &ctx.accounts.game.key(),
            lamports_to_transfer
        );
        invoke(
            &transfer_instruction,
            &[
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.game.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        Ok(())
    }

    pub fn burn_baby_burn(ctx: Context<BurnBabyBurn>, amount: u64) -> Result<()> {
        let signer_seeds: &[&[&[u8]]] = &[&[b"game", ctx.accounts.mint.to_account_info().key.as_ref(), &[ctx.bumps.game]]];
        let ix = spl_token_2022::instruction::burn_checked(
            &spl_token_2022::ID,
            &ctx.accounts.mint.key(),
            &ctx.accounts.mint_ata.key(),
            &ctx.accounts.payer.key(),
            &[],
            amount,
            ctx.accounts.mint.decimals,
        )?;
        let accounts = vec![
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.mint_ata.to_account_info(),
        ];
        invoke_signed(&ix, &accounts, signer_seeds)?;

        let game = &mut ctx.accounts.game;
        let current_price = load_raydium_pool_state(&ctx.accounts.raydium_clmm)?.get_clmm_price().to_num::<u64>();
    
        let is_short = ctx.accounts.mint.key() < ctx.accounts.other_mint.key();
        let payout = game.calculate_payout(amount, current_price, is_short);
    
        let ix = spl_token_2022::instruction::withdraw_excess_lamports(
            &spl_token_2022::ID,
            &ctx.accounts.mint.key(),
            &ctx.accounts.payer.key(),
            &ctx.accounts.game.key(),
            &[]
        )?;
        invoke_signed(&ix, &accounts, signer_seeds)?;
        let game_account = ctx.accounts.game.to_account_info();
        let payer_account = ctx.accounts.payer.to_account_info();

        **game_account.try_borrow_mut_lamports()? -= payout;
        **payer_account.try_borrow_mut_lamports()? += payout;

        Ok(())
    }

    pub fn transfer_hook(ctx: Context<TransferHook>, _amount: u64) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let clmm = &ctx.accounts.raydium_clmm;
        let clmm_state = load_raydium_pool_state(clmm)?;
        let price = clmm_state.get_clmm_price().to_num::<u64>();

        game.last_price = price;

        Ok(())
    }

    pub fn fallback<'info>(
        program_id: &Pubkey,
        accounts: &'info [AccountInfo<'info>],
        data: &[u8],
    ) -> Result<()> {
        let instruction = TransferHookInstruction::unpack(data)?;

        match instruction {
            TransferHookInstruction::Execute { amount } => {
                let amount_bytes = amount.to_le_bytes();
                __private::__global::transfer_hook(program_id, accounts, &amount_bytes)
            }
            _ => return Err(ProgramError::InvalidInstructionData.into()),
        }
    }
}


#[error_code]
pub enum TransferHookError {
    #[msg("Invalid CLMM Oracle")]
    InvalidCLMMOracle,
}

#[derive(Accounts)]
pub struct OmNomNom<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, seeds = [b"game", mint.key().as_ref()], bump)]
    pub game: Account<'info, Game>,
    #[account(mut)]
    pub other_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
}

#[derive(Accounts)]
pub struct BurnBabyBurn<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub other_mint: InterfaceAccount<'info, Mint>,
    #[account(mut, token::mint = mint)]
    pub mint_ata: InterfaceAccount<'info, TokenAccount>,
    #[account(mut, seeds = [b"game", mint.key().as_ref()], bump)]
    pub game: Account<'info, Game>,
    pub token_program: Program<'info, Token2022>,
    #[account(mut)]

        /// CHECK:

    pub raydium_clmm: AccountInfo<'info>,
    
}

#[derive(Accounts)]
pub struct InitializeExtraAccountMetaList<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"extra-account-metas", mint.key().as_ref()], 
        bump
    )]
    /// CHECK:
    pub extra_account_meta_list: AccountInfo<'info>,
    
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,
    
    pub other_mint: InterfaceAccount<'info, Mint>,
    
    #[account(init, payer=payer, space=80, seeds = [b"game", mint.key().as_ref()], bump)]
    pub game: Account<'info, Game>,
    
    
    pub system_program: Program<'info, System>,
    
    #[account(mut)]
        /// CHECK:

    pub raydium_clmm: AccountInfo<'info>,
    
    pub token_program: Program<'info, Token2022>,
}

#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(
        token::mint = mint, 
        token::authority = owner,
    )]
    pub source_token: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        token::mint = mint,
    )]
    pub destination_token: InterfaceAccount<'info, TokenAccount>,
    /// CHECK:
    pub owner: UncheckedAccount<'info>,
    #[account(
        seeds = [b"extra-account-metas", mint.key().as_ref()], 
        bump
    )]
    /// CHECK:
    pub extra_account_meta_list: UncheckedAccount<'info>,
    pub other_mint: InterfaceAccount<'info, Mint>,
    #[account(mut, seeds = [b"game", mint.key().as_ref()], bump)]
    pub game: Account<'info, Game>,
    #[account(mut)]
        /// CHECK:

    pub raydium_clmm: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct MintSolTokens<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, seeds = [b"mint"], bump)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = payer,
    )]
    pub destination: InterfaceAccount<'info, TokenAccount>,
    #[account(mut, seeds = [b"game", mint.key().as_ref()], bump)]
    pub game: Account<'info, Game>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
    
    /// CHECK:
    pub associated_token_program: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    #[account(mut)]
    /// CHECK:
    pub raydium_clmm: AccountInfo<'info>,
}