#[repr(u32)]
pub enum FundraiserError {
    InvalidAmount = 0,
    ContributionTooBig = 1,
    FundraiserEnded = 2,
    FundraiserNotEnded = 3,
    MaximumContributionsReached = 4,
    RefundAlreadyOrNotContributed = 5,
    InsufficientFund = 6,
}
