use pinocchio::cpi::{Seed, Signer};
use pinocchio::sysvars::clock::Clock;
use pinocchio::sysvars::rent::Rent;
use pinocchio::sysvars::Sysvar;
use pinocchio::{error::ProgramError, AccountView, Address};
use pinocchio_log::log;
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::state::Mint;

use crate::constants::*;
use crate::error::FundraiserError;
use crate::state::{Contributor, Fundraiser};

pub fn process_contribute_instruction(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // accounts
    log!("Accounts count: {}", accounts.len());
    // contributor, mint_to_raise, fundraiser_pda, contributor_pda, contributor_ata, vault, token_program, system_program
    let [contributor, mint_to_raise, fundraiser_pda, contributor_pda, contributor_ata, vault, _system_program, _token_program, _associated_token_program @ ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    // check signiture
    if !contributor.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    };
    // check ata owner is signer (ata should be initialized already)
    {
        let contributor_ata_state =
            pinocchio_token::state::TokenAccount::from_account_view(contributor_ata)?;
        let vault_state = pinocchio_token::state::TokenAccount::from_account_view(vault)?;
        if contributor_ata_state.owner() != contributor.address()
            || vault_state.owner() != fundraiser_pda.address()
        {
            return Err(ProgramError::IllegalOwner);
        };
        if contributor_ata_state.mint() != mint_to_raise.address()
            || vault_state.mint() != mint_to_raise.address()
        {
            return Err(ProgramError::InvalidAccountData);
        };
    }

    // instruction
    // amount: 8 bytes
    if instruction_data.len() != 8 {
        log!("Instruction length: {}", instruction_data.len());
        return Err(ProgramError::InvalidInstructionData);
    };
    // get amount
    let amount = u64::from_le_bytes(instruction_data[0..8].try_into().unwrap());
    // verify: minimum amount
    let decimals = Mint::from_account_view(mint_to_raise)?.decimals();
    if amount < 10_u64.pow(decimals as u32) {
        return Err(ProgramError::Custom(FundraiserError::InvalidAmount as u32));
    };
    {
        let fundraiser_state = Fundraiser::from_fundraiser_info(fundraiser_pda)?;
        // verify: less than the maximum allowed contribution
        if amount
            > (fundraiser_state.amount_to_raise * MAX_CONTRIBUTION_PERCENTAGE) / PERCENTAGE_SCALER
        {
            return Err(ProgramError::Custom(
                FundraiserError::ContributionTooBig as u32,
            ));
        };

        // verify: check duration has been reached
        let current_time = Clock::get()?.unix_timestamp;
        let end_time =
            fundraiser_state.time_started + (fundraiser_state.duration as i64 * SECONDS_TO_DAYS);
        if current_time >= end_time {
            return Err(ProgramError::Custom(
                FundraiserError::FundraiserEnded as u32,
            ));
        }
    }
    // seeds
    let pda_seeds: &[&[u8]] = &[
        b"contributor",
        fundraiser_pda.address().as_ref(),
        contributor.address().as_ref(),
    ];
    let (expected_pda, bump) = Address::find_program_address(pda_seeds, program_id);
    if contributor_pda.address() != &expected_pda {
        return Err(ProgramError::InvalidSeeds);
    };
    let bump = [bump.to_le()];
    let seed = [
        Seed::from(b"contributor"),
        Seed::from(fundraiser_pda.address().as_array()),
        Seed::from(contributor.address().as_array()),
        Seed::from(&bump),
    ];
    let seeds = Signer::from(&seed);

    // init if needed
    let is_uninitialized = contributor_pda.try_borrow()?.len() == 0;
    if is_uninitialized {
        // create pda account
        CreateAccount {
            from: contributor,
            to: contributor_pda,
            lamports: Rent::get()?.try_minimum_balance(Contributor::LEN)?,
            space: Contributor::LEN as u64,
            owner: program_id,
        }
        .invoke_signed(&[seeds])?;
    }
    // set or update inner
    {
        let contributor_state = Contributor::from_contributor_info(contributor_pda)?;
        let fundraiser_state = Fundraiser::from_fundraiser_info(fundraiser_pda)?;
        if is_uninitialized {
            contributor_state.amount = amount;
            contributor_state.bump = bump[0];
        } else {
            // verify: maximum contributions per contributor have been reached
            if contributor_state.amount
                > ((fundraiser_state.amount_to_raise * MAX_CONTRIBUTION_PERCENTAGE)
                    / PERCENTAGE_SCALER)
                || (contributor_state.amount + amount
                    > (fundraiser_state.amount_to_raise * MAX_CONTRIBUTION_PERCENTAGE)
                        / PERCENTAGE_SCALER)
            {
                return Err(ProgramError::Custom(
                    FundraiserError::MaximumContributionsReached as u32,
                ));
            }
            contributor_state.amount = contributor_state
                .amount
                .checked_add(amount)
                .ok_or(ProgramError::ArithmeticOverflow)?;
        }
        // update current amount in fundraiser
        fundraiser_state.current_amount = fundraiser_state
            .current_amount
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }

    // if pass everything, start transfer token from contributor ata to vault
    pinocchio_token::instructions::Transfer {
        from: contributor_ata,
        to: vault,
        authority: contributor,
        amount: amount,
    }
    .invoke()?;
    Ok(())
}
