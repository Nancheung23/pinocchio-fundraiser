#![allow(unexpected_cfgs)]
mod constants;
mod error;
mod instructions;
mod state;

#[cfg(test)]
mod tests;

use instructions::*;
use pinocchio::error::ProgramError;
use pinocchio::{address::declare_id, entrypoint, AccountView, Address, ProgramResult};

entrypoint!(process_instruction);
declare_id!("4ibrEMW5F6hKnkW4jVedswYv6H6VtwPN6ar6dvXDN1nT");
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum FundraiserInstructions {
    Initialize = 0,
    Contribute = 1,
    Checker = 2,
    Refund = 3,
}

impl TryFrom<&u8> for FundraiserInstructions {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Initialize),
            1 => Ok(Self::Contribute),
            2 => Ok(Self::Checker),
            3 => Ok(Self::Refund),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

pub fn process_instruction(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    let (discriminator, data) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    let instruction = FundraiserInstructions::try_from(discriminator)?;

    match instruction {
        FundraiserInstructions::Initialize => {
            process_initialize_instruction(program_id, accounts, data)
        }
        FundraiserInstructions::Contribute => {
            process_contribute_instruction(program_id, accounts, data)
        }
        FundraiserInstructions::Checker => process_checker_instruction(program_id, accounts, data),
        FundraiserInstructions::Refund => process_refund_instruction(program_id, accounts, data),
    }
}
