use super::*;
use crate::state::{Contributor, Fundraiser};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::str::FromStr;

fn setup_initialized_fundraiser_and_contributor() -> (TestContext, Keypair, Pubkey, Pubkey) {
    let mut context = setup();

    let system_program = Pubkey::from_str("11111111111111111111111111111111").unwrap();
    let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    let ata_program = Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap();

    let amount_to_raise: u64 = 1_000_000_000;
    let duration: u8 = 7;
    let mut init_data = vec![0u8]; // 0: Initialize
    init_data.extend_from_slice(&amount_to_raise.to_le_bytes());
    init_data.push(duration);

    let init_accounts = vec![
        AccountMeta::new(context.maker.pubkey(), true),
        AccountMeta::new_readonly(context.mint, false),
        AccountMeta::new(context.fundraiser_pda, false),
        AccountMeta::new(context.vault, false),
        AccountMeta::new_readonly(system_program, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(ata_program, false),
    ];

    let init_ix = Instruction::new_with_bytes(context.program_id, &init_data, init_accounts);
    let init_tx = Transaction::new_signed_with_payer(
        &[init_ix],
        Some(&context.maker.pubkey()),
        &[&context.maker],
        context.svm.latest_blockhash(),
    );
    context.svm.send_transaction(init_tx).unwrap();

    let contributor = Keypair::new();
    context
        .svm
        .airdrop(&contributor.pubkey(), 10_000_000_000)
        .unwrap();

    let (contributor_ata, _) = Pubkey::find_program_address(
        &[
            contributor.pubkey().as_ref(),
            token_program.as_ref(),
            context.mint.as_ref(),
        ],
        &ata_program,
    );

    let create_ata_ix = Instruction::new_with_bytes(
        ata_program,
        &[],
        vec![
            AccountMeta::new(context.maker.pubkey(), true),
            AccountMeta::new(contributor_ata, false),
            AccountMeta::new_readonly(contributor.pubkey(), false),
            AccountMeta::new_readonly(context.mint, false),
            AccountMeta::new_readonly(system_program, false),
            AccountMeta::new_readonly(token_program, false),
        ],
    );

    let initial_balance: u64 = 100_000_000;
    let mut mint_to_data = vec![7u8];
    mint_to_data.extend_from_slice(&initial_balance.to_le_bytes());

    let mint_to_ix = Instruction::new_with_bytes(
        token_program,
        &mint_to_data,
        vec![
            AccountMeta::new(context.mint, false),
            AccountMeta::new(contributor_ata, false),
            AccountMeta::new_readonly(context.maker.pubkey(), true),
        ],
    );

    let setup_tx = Transaction::new_signed_with_payer(
        &[create_ata_ix, mint_to_ix],
        Some(&context.maker.pubkey()),
        &[&context.maker],
        context.svm.latest_blockhash(),
    );
    context
        .svm
        .send_transaction(setup_tx)
        .expect("Failed to initialize and mint to contributor_ata");

    let (contributor_pda, _) = Pubkey::find_program_address(
        &[
            b"contributor",
            context.fundraiser_pda.as_ref(),
            contributor.pubkey().as_ref(),
        ],
        &context.program_id,
    );

    (context, contributor, contributor_ata, contributor_pda)
}

#[test]
fn test_contribute_success() {
    let (mut context, contributor, contributor_ata, contributor_pda) =
        setup_initialized_fundraiser_and_contributor();

    let system_program = Pubkey::from_str("11111111111111111111111111111111").unwrap();
    let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    let ata_program = Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap();

    let contribute_amount: u64 = 10_000_000;

    let mut instruction_data = vec![1u8];
    instruction_data.extend_from_slice(&contribute_amount.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(contributor.pubkey(), true),
        AccountMeta::new_readonly(context.mint, false),
        AccountMeta::new(context.fundraiser_pda, false),
        AccountMeta::new(contributor_pda, false),
        AccountMeta::new(contributor_ata, false),
        AccountMeta::new(context.vault, false),
        AccountMeta::new_readonly(system_program, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(ata_program, false),
    ];

    let ix = Instruction::new_with_bytes(context.program_id, &instruction_data, accounts);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&contributor.pubkey()),
        &[&contributor],
        context.svm.latest_blockhash(),
    );

    let result = context.svm.send_transaction(tx);
    assert!(result.is_ok(), "Contribute tx failed: {:?}", result.err());

    let contributor_account = context.svm.get_account(&contributor_pda).unwrap();
    let contributor_state = bytemuck::from_bytes::<Contributor>(&contributor_account.data);
    assert_eq!(contributor_state.amount, contribute_amount);

    let fundraiser_account = context.svm.get_account(&context.fundraiser_pda).unwrap();
    let fundraiser_state = bytemuck::from_bytes::<Fundraiser>(&fundraiser_account.data);
    assert_eq!(fundraiser_state.current_amount, contribute_amount);

    let vault_account = context.svm.get_account(&context.vault).unwrap();
    let vault_balance = u64::from_le_bytes(vault_account.data[64..72].try_into().unwrap());
    assert_eq!(vault_balance, contribute_amount);
}

#[test]
fn test_contribute_multiple_times() {
    let (mut context, contributor, contributor_ata, contributor_pda) =
        setup_initialized_fundraiser_and_contributor();

    let system_program = Pubkey::from_str("11111111111111111111111111111111").unwrap();
    let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    let ata_program = Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap();

    let accounts = vec![
        AccountMeta::new(contributor.pubkey(), true),
        AccountMeta::new_readonly(context.mint, false),
        AccountMeta::new(context.fundraiser_pda, false),
        AccountMeta::new(contributor_pda, false),
        AccountMeta::new(contributor_ata, false),
        AccountMeta::new(context.vault, false),
        AccountMeta::new_readonly(system_program, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(ata_program, false),
    ];

    let amount_1: u64 = 10_000_000;
    let mut data_1 = vec![1u8];
    data_1.extend_from_slice(&amount_1.to_le_bytes());
    let ix_1 = Instruction::new_with_bytes(context.program_id, &data_1, accounts.clone());

    let tx_1 = Transaction::new_signed_with_payer(
        &[ix_1],
        Some(&contributor.pubkey()),
        &[&contributor],
        context.svm.latest_blockhash(),
    );
    context.svm.send_transaction(tx_1).unwrap();

    let amount_2: u64 = 25_000_000;
    let mut data_2 = vec![1u8];
    data_2.extend_from_slice(&amount_2.to_le_bytes());
    let ix_2 = Instruction::new_with_bytes(context.program_id, &data_2, accounts);

    let tx_2 = Transaction::new_signed_with_payer(
        &[ix_2],
        Some(&contributor.pubkey()),
        &[&contributor],
        context.svm.latest_blockhash(),
    );
    context.svm.send_transaction(tx_2).unwrap();

    let total_expected = amount_1 + amount_2;

    let contributor_account = context.svm.get_account(&contributor_pda).unwrap();
    let contributor_state = bytemuck::from_bytes::<Contributor>(&contributor_account.data);
    assert_eq!(contributor_state.amount, total_expected);

    let fundraiser_account = context.svm.get_account(&context.fundraiser_pda).unwrap();
    let fundraiser_state = bytemuck::from_bytes::<Fundraiser>(&fundraiser_account.data);
    assert_eq!(fundraiser_state.current_amount, total_expected);

    let vault_account = context.svm.get_account(&context.vault).unwrap();
    let vault_balance = u64::from_le_bytes(vault_account.data[64..72].try_into().unwrap());
    assert_eq!(vault_balance, total_expected);
}
