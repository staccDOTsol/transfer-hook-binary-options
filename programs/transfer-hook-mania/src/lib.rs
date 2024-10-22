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



#[account]
pub struct VolatilityIndex {
    pub price_history: [u64; 24],  // Store last 24 hourly prices
    pub current_index: usize,      // Current index in the price history array
    pub last_update_time: i64,     // Last update timestamp
    pub volatility: u64,           // Current volatility (as a percentage, multiplied by 100)
    pub up_threshold: u64,         // Threshold for up volatility (percentage * 100)
    pub down_threshold: u64,       // Threshold for down volatility (percentage * 100)
    pub sol_mint: Pubkey,          // SOL mint address
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
    pub this_mint_won: bool, // 1
    pub this_mint_ate_the_other_already: bool, // +1=2
    pub total_pending_payout: u64, // +8=10
    pub next_epoch: u64, // +8=18
    pub last_epoch: u64, // +8=26
    pub last_price: u64, // +8=34
    pub other_mint: Pubkey, // +32=42
}
declare_id!("AZR4kEoxHrD879oPU5vLbJnryCHEyrJfiFwmASUXdFqf");

#[program]
pub mod transfer_hook {

    use solana_program::{program::{invoke, invoke_signed}, system_instruction::transfer};

    use super::*;

    pub fn initialize_extra_account_meta_list(
        ctx: Context<InitializeExtraAccountMetaList>,
        up_threshold: u64,
        down_threshold: u64,
    ) -> Result<()> {
        // index 0-3 are the accounts required for token transfer (source, mint, destination, owner)
        // index 4 is address of ExtraAccountMetaList account
        let account_metas = vec![
            // index 5, wrapped SOL mint
            ExtraAccountMeta::new_with_pubkey(&ctx.accounts.other_mint.key(), false, false)?,
            ExtraAccountMeta::new_with_pubkey(&ctx.accounts.game.key(), false, true)?,
            ExtraAccountMeta::new_with_pubkey(&ctx.accounts.volatility_index.key(), false, true)?,
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
    
        // calculate account size
        let account_size = ExtraAccountMetaList::size_of(account_metas.len())? as u64;
        // calculate minimum required lamports
        let lamports = Rent::get()?.minimum_balance(account_size as usize);
    
        let mint = ctx.accounts.mint.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"extra-account-metas",
            &mint.as_ref(),
            &[ctx.bumps.extra_account_meta_list],
        ]];
    
        // create ExtraAccountMetaList account
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
    
        // Initialize game account
        let game = &mut ctx.accounts.game;
        game.this_mint_won = false;
        game.this_mint_ate_the_other_already = false;
        game.total_pending_payout = 0;
        game.next_epoch = (unix_timestamp + 24 * 3600) as u64;
        game.last_epoch = unix_timestamp as u64;
        game.last_price = clmm_state.get_clmm_price().to_num::<u64>();
        game.other_mint = ctx.accounts.other_mint.key();
    
        // Initialize volatility index account
        let volatility_index = &mut ctx.accounts.volatility_index;
        volatility_index.price_history = [0; 24];
        volatility_index.current_index = 0;
        volatility_index.last_update_time = unix_timestamp;
        volatility_index.volatility = 0;
        volatility_index.up_threshold = up_threshold;
        volatility_index.down_threshold = down_threshold;
        volatility_index.sol_mint = ctx.accounts.mint.key();
    
        // initialize ExtraAccountMetaList account with extra accounts
        ExtraAccountMetaList::init::<ExecuteInstruction>(
            &mut ctx.accounts.extra_account_meta_list.try_borrow_mut_data()?,
            &account_metas,
        )?;
    
        Ok(())
    }
    
    
    pub fn om_nom_nom(ctx: Context<OmNomNom>) -> Result<()> {
        require!(ctx.accounts.game.this_mint_won, TransferHookError::InvalidCLMMOracle);
        let signer_seeds: &[&[&[u8]]] = &[&[b"game", ctx.accounts.mint.to_account_info().key.as_ref(), &[ctx.bumps.game]]];

        let ix = spl_token_2022::instruction::withdraw_excess_lamports(
            &spl_token_2022::ID,
            &ctx.accounts.other_mint.key(),
            &ctx.accounts.mint.key(),
            &ctx.accounts.game.key(),
            &[]
        )?;
        let accounts = vec![
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.other_mint.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.game.to_account_info(),
        ];
        invoke_signed(&ix, &accounts, signer_seeds)?;

        let game = &mut ctx.accounts.game;
        let volatility_index = &ctx.accounts.volatility_index;

        // Calculate the reward multiplier based on volatility
        let reward_multiplier = calculate_reward_multiplier(volatility_index.volatility, volatility_index.up_threshold, volatility_index.down_threshold);

        // Apply the reward multiplier to the payout
        let lamports = ctx.accounts.mint.get_lamports();
        let payout = (lamports as f64 * reward_multiplier).round() as u64;

        game.this_mint_won = false;
        game.this_mint_ate_the_other_already = true;
        
        // Transfer the payout
        let transfer_ix = transfer(
            &ctx.accounts.payer.key(),
            &ctx.accounts.mint.key(),
            payout,
        );
        invoke(&transfer_ix, &accounts)?;
        
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
        let lamports = ctx.accounts.mint.get_lamports();
        let supply = ctx.accounts.mint.supply;
        let amount_of_supply: f64 = amount as f64 / supply as f64;
        let payout = (amount_of_supply * lamports as f64).round() as u64;

        let volatility_index = &ctx.accounts.volatility_index;

        // Calculate the reward multiplier based on volatility
        let reward_multiplier = calculate_reward_multiplier(volatility_index.volatility, volatility_index.up_threshold, volatility_index.down_threshold);

        // Apply the reward multiplier to the payout
        let adjusted_payout = (payout as f64 * reward_multiplier).round() as u64;

        game.total_pending_payout -= adjusted_payout;

        let ix = spl_token_2022::instruction::withdraw_excess_lamports(
            &spl_token_2022::ID,
            &ctx.accounts.mint.key(),
            &ctx.accounts.payer.key(),
            &ctx.accounts.game.key(),
            &[]
        )?;
        invoke_signed(&ix, &accounts, signer_seeds)?;

        let transfer_back_ix = transfer(
            &ctx.accounts.payer.key(),
            &ctx.accounts.mint.key(),
            lamports as u64 - adjusted_payout,
        );
        invoke(&transfer_back_ix, &accounts)?;

        Ok(())
    }

    pub fn transfer_hook(ctx: Context<TransferHook>, _amount: u64) -> Result<()> {
        let unix_timestamp = Clock::get()?.unix_timestamp;
        let game = &mut ctx.accounts.game;
        game.last_epoch = unix_timestamp as u64;
        let clmm = &ctx.accounts.raydium_clmm;
        let clmm_state = load_raydium_pool_state(clmm)?;
        let price = clmm_state.get_clmm_price().to_num::<u64>();
        let up_option = ctx.accounts.mint.key() > ctx.accounts.other_mint.key();
        let option_type = if up_option { "up" } else { "down" };
        msg!("Option type triggered: {}", option_type);
    
        if game.last_price < price && up_option {
            game.this_mint_won = true;
            game.this_mint_ate_the_other_already = false;
        } else if game.last_price > price && !up_option {
            game.this_mint_won = true;
            game.this_mint_ate_the_other_already = false;
        } else {
            game.this_mint_won = false;
            game.this_mint_ate_the_other_already = false;
        }
    
        let volatility_index = &mut ctx.accounts.volatility_index;
        let current_time = unix_timestamp;
        let index = volatility_index.current_index;
        // Update only if an hour has passed
        if current_time - volatility_index.last_update_time >= 3600 {
            // Update price history
            volatility_index.price_history[index] = price;
            volatility_index.current_index = (index + 1) % 24;
    
            // Calculate volatility
            let (min_price, max_price) = volatility_index.price_history.iter()
                .filter(|&&p| p != 0)
                .fold((u64::MAX, 0), |(min, max), &p| (min.min(p), max.max(p)));
    
            if min_price != u64::MAX && max_price != 0 {
                volatility_index.volatility = ((max_price - min_price) * 10000) / min_price;
            }
    
            volatility_index.last_update_time = current_time;
        }
    
        // Check if current volatility is above thresholds
        if volatility_index.volatility > volatility_index.up_threshold {
            msg!("Volatility is above the up threshold: {}%", volatility_index.volatility as f64 / 100.0);
        } else if volatility_index.volatility < volatility_index.down_threshold {
            msg!("Volatility is below the down threshold: {}%", volatility_index.volatility as f64 / 100.0);
        }
    
        Ok(())
    }

    
 
    // fallback instruction handler as workaround to anchor instruction discriminator check
    pub fn fallback<'info>(
        program_id: &Pubkey,
        accounts: &'info [AccountInfo<'info>],
        data: &[u8],
    ) -> Result<()> {
        let instruction = TransferHookInstruction::unpack(data)?;

        // match instruction discriminator to transfer hook interface execute instruction  
        // token2022 program CPIs this instruction on token transfer
        match instruction {
            TransferHookInstruction::Execute { amount } => {
                let amount_bytes = amount.to_le_bytes();

                // invoke custom transfer hook instruction on our program
                __private::__global::transfer_hook(program_id, accounts, &amount_bytes)
            }
            _ => return Err(ProgramError::InvalidInstructionData.into()),
        }
    }
}

    // Helper function to calculate reward multiplier based on volatility
    fn calculate_reward_multiplier(current_volatility: u64, up_threshold: u64, down_threshold: u64) -> f64 {
        if current_volatility > up_threshold {
            1.0 + ((current_volatility - up_threshold) as f64 / 10000.0) // Increase reward
        } else if current_volatility < down_threshold {
            1.0 - ((down_threshold - current_volatility) as f64 / 10000.0) // Decrease reward
        } else {
            1.0 // No change in reward
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
    #[account(
        mut,
        seeds = [b"volatility-index", mint.key().as_ref()],
        bump
    )]
    pub volatility_index: Account<'info, VolatilityIndex>,
}
#[derive(Accounts)]
pub struct BurnBabyBurn<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut, token::mint = mint)]
    pub mint_ata: InterfaceAccount<'info, TokenAccount>,
    #[account(mut, seeds = [b"game", mint.key().as_ref()], bump)]
    pub game: Account<'info, Game>,
    pub token_program: Program<'info, Token2022>,
    #[account(
        mut,
        seeds = [b"volatility-index", mint.key().as_ref()],
        bump
    )]
    pub volatility_index: Account<'info, VolatilityIndex>,
}

#[derive(Accounts)]
pub struct InitializeExtraAccountMetaList<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: ExtraAccountMetaList Account, must use these seeds
    #[account(
        mut,
        seeds = [b"extra-account-metas", mint.key().as_ref()], 
        bump
    )]
    pub extra_account_meta_list: AccountInfo<'info>,
    
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,
    
    pub other_mint: InterfaceAccount<'info, Mint>,
    
    #[account(init, payer=payer, space=80, seeds = [b"game", mint.key().as_ref()], bump)]
    pub game: Account<'info, Game>,
    
    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<VolatilityIndex>(),
        seeds = [b"volatility-index", mint.key().as_ref()],
        bump
    )]
    pub volatility_index: Account<'info, VolatilityIndex>,
    
    pub system_program: Program<'info, System>,
    
    /// CHECK: Raydium CLMM pool account
    pub raydium_clmm: AccountInfo<'info>,
    
    pub token_program: Program<'info, Token2022>,
}

// Order of accounts matters for this struct.
// The first 4 accounts are the accounts required for token transfer (source, mint, destination, owner)
// Remaining accounts are the extra accounts required from the ExtraAccountMetaList account
// These accounts are provided via CPI to this program from the token2022 program


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
    /// CHECK: source token account owner, can be SystemAccount or PDA owned by another program
    pub owner: UncheckedAccount<'info>,
    /// CHECK: ExtraAccountMetaList Account,
    #[account(
        seeds = [b"extra-account-metas", mint.key().as_ref()], 
        bump
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,
    pub other_mint: InterfaceAccount<'info, Mint>,
    #[account(mut, seeds = [b"game", mint.key().as_ref()], bump)]
    pub game: Account<'info, Game>,
    #[account(
        mut,
        seeds = [b"volatility-index", mint.key().as_ref()],
        bump
    )]
    pub volatility_index: Account<'info, VolatilityIndex>,
    /// CHECK: Raydium CLMM pool account
    pub raydium_clmm: AccountInfo<'info>,
}