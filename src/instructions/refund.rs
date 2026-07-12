use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    AccountView, Address,
};
use pinocchio_log::log;

use crate::state::{Contributor, Fundraiser};
use crate::{constants::*, error::FundraiserError};

pub fn process_refund_instruction(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // accounts
    log!("Accounts count: {}", accounts.len());
    let [contributor, maker, mint_to_raise, fundraiser_pda, contributor_pda, vault, contributor_ata, _token_program, _associated_token_program @ ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    // check signiture
    if !contributor.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    };
    // check owner is signer
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

    // instructions
    // 0 byte
    if instruction_data.len() != 0 {
        return Err(ProgramError::InvalidInstructionData);
    };
    // seeds
    let pda_seeds_contributor = &[
        b"contributor",
        fundraiser_pda.address().as_ref(),
        contributor.address().as_ref(),
    ];
    let pda_seeds_fundraiser = &[b"fundraiser", maker.address().as_ref()];

    let (expected_pda_contributor, _bump_contributor) =
        Address::find_program_address(pda_seeds_contributor, program_id);
    let (expected_pda_fundraiser, bump_fundraiser) =
        Address::find_program_address(pda_seeds_fundraiser, program_id);
    if contributor_pda.address() != &expected_pda_contributor
        || fundraiser_pda.address() != &expected_pda_fundraiser
    {
        return Err(ProgramError::InvalidSeeds);
    };
    let bump_fundraiser = [bump_fundraiser.to_le()];
    let seed_fundraiser = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_array()),
        Seed::from(&bump_fundraiser),
    ];

    // verify: raise failed
    {
        let fundraiser_state = Fundraiser::from_fundraiser_info(fundraiser_pda)?;
        let current_time = Clock::get()?.unix_timestamp;
        let end_time =
            fundraiser_state.time_started + (fundraiser_state.duration as i64 * SECONDS_TO_DAYS);
        let is_failed = current_time >= end_time
            && fundraiser_state.current_amount < fundraiser_state.amount_to_raise;
        // check if do refund
        match is_failed {
            true => {
                // transfer from vault to contributor_ata
                let contributor_pda_state = Contributor::from_contributor_info(contributor_pda)?;
                if contributor_pda_state.amount > 0 {
                    if fundraiser_state.current_amount >= contributor_pda_state.amount {
                        pinocchio_token::instructions::Transfer {
                            from: vault,
                            to: contributor_ata,
                            authority: fundraiser_pda,
                            amount: contributor_pda_state.amount,
                        }
                        .invoke_signed(&[Signer::from(&seed_fundraiser)])?;
                        // update amount
                        fundraiser_state.current_amount = fundraiser_state
                            .current_amount
                            .checked_sub(contributor_pda_state.amount)
                            .ok_or(ProgramError::ArithmeticOverflow)?;
                        contributor_pda_state.amount = 0;

                        // close contributor pda
                        let contributor_pda_lamports = contributor_pda.lamports();
                        let contributor_lamports = contributor.lamports();
                        contributor.set_lamports(contributor_lamports + contributor_pda_lamports);
                        contributor_pda.set_lamports(0);
                        if let Ok(mut data) = contributor_pda.try_borrow_mut() {
                            data.fill(0);
                        }
                    } else {
                        return Err(ProgramError::Custom(
                            FundraiserError::InsufficientFund as u32,
                        ));
                    }
                } else {
                    // verify: not refunded yet
                    return Err(ProgramError::Custom(
                        FundraiserError::RefundAlreadyOrNotContributed as u32,
                    ));
                }
            }
            _ => {
                if !is_failed {
                    return Err(ProgramError::Custom(
                        FundraiserError::FundraiserNotEnded as u32,
                    ));
                }
            }
        }
    }

    Ok(())
}
