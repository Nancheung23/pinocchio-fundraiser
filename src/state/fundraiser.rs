use bytemuck::{Pod, Zeroable};
use pinocchio::{error::ProgramError, AccountView};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Fundraiser {
    pub maker: [u8; 32],
    pub mint_to_raise: [u8; 32],
    pub amount_to_raise: u64,
    pub current_amount: u64,
    pub time_started: i64,
    pub duration: u8,
    pub bump: u8,
    _padding: [u8; 6],
}

impl Fundraiser {
    pub const LEN: usize = 32 + 32 + 8 + 8 + 8 + 1 + 1 + 6;
    pub fn from_fundraiser_info(info: &AccountView) -> Result<&mut Self, ProgramError> {
        let mut data = info.try_borrow_mut()?;
        // len check
        if data.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        let raw_slice: &mut [u8] =
            unsafe { core::slice::from_raw_parts_mut(data.as_mut_ptr(), data.len()) };
        let result = bytemuck::try_from_bytes_mut::<Self>(raw_slice);
        match result {
            Ok(fundriaser_ref) => Ok(fundriaser_ref),
            Err(_) => Err(ProgramError::InvalidAccountData),
        }
    }
}
