use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{clock::Clock, rent::Rent, Sysvar},
    AccountView, Address,
};
use pinocchio_log::log;
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::state::Mint;

use crate::constants::*;
use crate::state::Fundraiser;

pub fn process_initialize_instruction(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Accounts
    // maker, mint_to_raise, fundraiser PDA, vault ata, system_program, token_program, associated_token_program
    log!("Accounts count: {}", accounts.len());
    let [maker, mint_to_raise, fundraiser_pda, vault, system_program, token_program, _associated_token_program @ ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    // check signiture
    if !maker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Instructions
    // check instructions: 8 bytes amount + 1 byte duration
    if instruction_data.len() != 9 {
        return Err(ProgramError::InvalidInstructionData);
    };

    // get augments from instruction_data
    let amount_to_raise = u64::from_le_bytes(instruction_data[0..8].try_into().unwrap());
    // amount require (based on logic from anchor version)
    let decimals = Mint::from_account_view(mint_to_raise)?.decimals();
    if amount_to_raise <= MIN_AMOUNT_TO_RAISE.pow(decimals as u32) {
        return Err(ProgramError::InvalidArgument);
    };

    let duration = instruction_data[8];
    let pda_seeds: &[&[u8]] = &[b"fundraiser", maker.address().as_ref()];
    let (expected_pda, bump) = Address::find_program_address(pda_seeds, program_id);
    if fundraiser_pda.address() != &expected_pda {
        return Err(ProgramError::InvalidSeeds);
    };

    // create fundraiser pda
    let bump = [bump.to_le()];
    let seed = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_array()),
        Seed::from(&bump),
    ];
    let seeds = Signer::from(&seed);
    CreateAccount {
        from: maker,
        to: fundraiser_pda,
        lamports: Rent::get()?.try_minimum_balance(Fundraiser::LEN)?,
        space: Fundraiser::LEN as u64,
        owner: program_id,
    }
    .invoke_signed(&[seeds])?;

    // set inner
    {
        let mut data = fundraiser_pda.try_borrow_mut()?;
        let raw_slice: &mut [u8] =
            unsafe { core::slice::from_raw_parts_mut(data.as_mut_ptr(), data.len()) };
        if raw_slice.len() != Fundraiser::LEN {
            return Err(ProgramError::InvalidAccountData);
        };
        let fundraiser_state = bytemuck::try_from_bytes_mut::<Fundraiser>(raw_slice)
            .map_err(|_| ProgramError::InvalidAccountData)?;
        fundraiser_state.maker = *maker.address().as_array();
        fundraiser_state.mint_to_raise = *mint_to_raise.address().as_array();
        fundraiser_state.amount_to_raise = amount_to_raise;
        fundraiser_state.current_amount = 0 as u64;
        fundraiser_state.time_started = Clock::get()?.unix_timestamp as i64;
        fundraiser_state.duration = duration;
        fundraiser_state.bump = bump[0];
    }

    // create ata: vault
    pinocchio_associated_token_account::instructions::Create {
        funding_account: maker,
        account: vault,
        wallet: fundraiser_pda,
        mint: mint_to_raise,
        system_program: system_program,
        token_program: token_program,
    }
    .invoke()?;
    Ok(())
}
