#[repr(u32)]
pub enum FundraiserError {
    InvalidAmount = 0,
    ContributionTooBig = 1,
    FundraiserEnded = 2,
    MaximumContributionsReached = 3,
}
