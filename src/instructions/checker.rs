use pinocchio::cpi::{Seed, Signer};
use pinocchio::sysvars::clock::Clock;
use pinocchio::sysvars::Sysvar;
use pinocchio::{error::ProgramError, AccountView, Address};
use pinocchio_log::log;

use crate::constants::*;
use crate::error::FundraiserError;
use crate::state::Fundraiser;

pub fn process_checker_instruction(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // accounts
    log!("Accounts count: {}", accounts.len());
    let [maker, mint_to_raise, fundraiser_pda, vault, maker_ata, _token_program, _associated_token_program @ ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    // check signiture
    if !maker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    };
    // check owner is signer
    {
        let maker_ata_state = pinocchio_token::state::TokenAccount::from_account_view(maker_ata)?;
        let vault_state = pinocchio_token::state::TokenAccount::from_account_view(vault)?;
        if maker_ata_state.owner() != maker.address()
            || vault_state.owner() != fundraiser_pda.address()
        {
            return Err(ProgramError::IllegalOwner);
        };
        if maker_ata_state.mint() != mint_to_raise.address()
            || vault_state.mint() != mint_to_raise.address()
        {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    // no augment from instructions
    if instruction_data.len() != 0 {
        return Err(ProgramError::InvalidInstructionData);
    }
    // check seeds
    let pda_seeds: &[&[u8]] = &[b"fundraiser", maker.address().as_ref()];
    let (expected_pda, bump) = Address::find_program_address(pda_seeds, program_id);
    if fundraiser_pda.address() != &expected_pda {
        return Err(ProgramError::InvalidSeeds);
    };
    let bump = [bump.to_le()];
    let seed = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_array()),
        Seed::from(&bump),
    ];
    // let seeds = Signer::from(&seed);

    // check time
    {
        let current_time = Clock::get()?.unix_timestamp;
        let fundraiser_state = Fundraiser::from_fundraiser_info(fundraiser_pda)?;
        let end_time =
            fundraiser_state.time_started + (fundraiser_state.duration as i64 * SECONDS_TO_DAYS);
        // check
        if current_time < end_time {
            return Err(ProgramError::Custom(
                FundraiserError::FundraiserNotEnded as u32,
            ));
        }
        let is_raised = fundraiser_state.current_amount >= fundraiser_state.amount_to_raise;
        match is_raised {
            true => {
                // transfer all token to maker's ata
                pinocchio_token::instructions::Transfer {
                    from: vault,
                    to: maker_ata,
                    authority: fundraiser_pda,
                    amount: fundraiser_state.current_amount,
                }
                .invoke_signed(&[Signer::from(&seed)])?;

                // close vault account
                pinocchio_token::instructions::CloseAccount {
                    account: vault,
                    destination: maker,
                    authority: fundraiser_pda,
                }
                .invoke_signed(&[Signer::from(&seed)])?;

                // close pda
                let fundraiser_lamports = fundraiser_pda.lamports();
                let maker_lamports = maker.lamports();
                maker.set_lamports(maker_lamports + fundraiser_lamports);
                fundraiser_pda.set_lamports(0);
                if let Ok(mut data) = fundraiser_pda.try_borrow_mut() {
                    data.fill(0);
                }
            }
            _ => {
                // if raise failed
            }
        }
    }
    Ok(())
}
