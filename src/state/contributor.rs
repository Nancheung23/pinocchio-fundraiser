use bytemuck::{Pod, Zeroable};
use pinocchio::{error::ProgramError, AccountView};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Contributor {
    // 8 bytes
    pub amount: u64,
    pub bump: u8,
    _padding: [u8; 7],
}

impl Contributor {
    pub const LEN: usize = 8;
    pub fn from_contributor_info(info: &AccountView) -> Result<&mut Self, ProgramError> {
        let mut data = info.try_borrow_mut()?;
        if data.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        };
        let raw_slice: &mut [u8] =
            unsafe { core::slice::from_raw_parts_mut(data.as_mut_ptr(), data.len()) };
        let result = bytemuck::try_from_bytes_mut::<Self>(raw_slice);
        match result {
            Ok(contributor_ref) => Ok(contributor_ref),
            Err(_) => Err(ProgramError::InvalidAccountData),
        }
    }
}
