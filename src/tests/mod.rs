pub mod test_initialize;

use litesvm::LiteSVM;
use solana_sdk::{
    account::Account,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::str::FromStr;

pub struct TestContext {
    pub svm: LiteSVM,
    pub program_id: Pubkey,
    pub maker: Keypair,
    pub mint: Pubkey,
    pub fundraiser_pda: Pubkey,
    pub vault: Pubkey,
}

pub fn setup() -> TestContext {
    let mut svm = LiteSVM::new();
    let program_id = Pubkey::new_unique();

    svm.add_program_from_file(program_id, "target/deploy/fundraiser.so")
        .unwrap_or_else(|_| panic!("Failed to load .so file. Please run 'cargo build-sbf' first!"));

    let maker = Keypair::new();
    svm.airdrop(&maker.pubkey(), 100_000_000_000).unwrap();

    let mint_keypair = Keypair::new();
    let mint = mint_keypair.pubkey();
    let token_program_id = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    let mint_rent = svm.minimum_balance_for_rent_exemption(82);

    let mut mint_data = vec![0u8; 82];
    mint_data[0..4].copy_from_slice(&1u32.to_le_bytes());
    mint_data[4..36].copy_from_slice(&maker.pubkey().to_bytes());
    mint_data[36..44].copy_from_slice(&0u64.to_le_bytes());
    mint_data[44] = 6;
    mint_data[45] = 1;
    mint_data[46..50].copy_from_slice(&0u32.to_le_bytes());

    svm.set_account(
        mint,
        Account {
            lamports: mint_rent,
            data: mint_data,
            owner: token_program_id,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    let (fundraiser_pda, _) =
        Pubkey::find_program_address(&[b"fundraiser", maker.pubkey().as_ref()], &program_id);

    let ata_program_id = Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap();
    let (vault, _) = Pubkey::find_program_address(
        &[
            fundraiser_pda.as_ref(),
            token_program_id.as_ref(),
            mint.as_ref(),
        ],
        &ata_program_id,
    );

    TestContext {
        svm,
        program_id,
        maker,
        mint,
        fundraiser_pda,
        vault,
    }
}
