pub mod checker;
pub mod contribute;
pub mod initialize;
pub mod refund;

pub use checker::*;
pub use contribute::*;
pub use initialize::*;
pub use refund::*;

use pinocchio::error::ProgramError;

pub enum FundraiserInstructions {
    Initialize = 0,
    Checker = 1,
    Contribute = 2,
    Refund = 3,
}

impl TryFrom<&u8> for FundraiserInstructions {
    type Error = ProgramError;
    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(FundraiserInstructions::Initialize),
            1 => Ok(FundraiserInstructions::Checker),
            2 => Ok(FundraiserInstructions::Contribute),
            3 => Ok(FundraiserInstructions::Refund),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
