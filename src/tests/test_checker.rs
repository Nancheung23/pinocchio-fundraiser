use super::*;
use crate::state::Fundraiser;
use solana_sdk::{
    clock::Clock,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Signer,
    transaction::Transaction,
};
use std::str::FromStr;

fn setup_funded_fundraiser() -> (TestContext, Pubkey) {
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

    let (maker_ata, _) = Pubkey::find_program_address(
        &[
            context.maker.pubkey().as_ref(),
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
            AccountMeta::new(maker_ata, false),
            AccountMeta::new_readonly(context.maker.pubkey(), false),
            AccountMeta::new_readonly(context.mint, false),
            AccountMeta::new_readonly(system_program, false),
            AccountMeta::new_readonly(token_program, false),
        ],
    );
    let ata_tx = Transaction::new_signed_with_payer(
        &[create_ata_ix],
        Some(&context.maker.pubkey()),
        &[&context.maker],
        context.svm.latest_blockhash(),
    );
    context.svm.send_transaction(ata_tx).unwrap();

    let mut pda_account = context.svm.get_account(&context.fundraiser_pda).unwrap();
    let state = bytemuck::from_bytes_mut::<Fundraiser>(&mut pda_account.data);
    state.current_amount = amount_to_raise;
    context
        .svm
        .set_account(context.fundraiser_pda, pda_account)
        .unwrap();

    let mut vault_account = context.svm.get_account(&context.vault).unwrap();
    vault_account.data[64..72].copy_from_slice(&amount_to_raise.to_le_bytes());
    context
        .svm
        .set_account(context.vault, vault_account)
        .unwrap();

    (context, maker_ata)
}

#[test]
fn test_checker_fails_before_end_time() {
    let (mut context, maker_ata) = setup_funded_fundraiser();

    let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    let ata_program = Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap();

    let instruction_data = vec![2u8];

    let accounts = vec![
        AccountMeta::new(context.maker.pubkey(), true),
        AccountMeta::new_readonly(context.mint, false),
        AccountMeta::new(context.fundraiser_pda, false),
        AccountMeta::new(context.vault, false),
        AccountMeta::new(maker_ata, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(ata_program, false),
    ];

    let ix = Instruction::new_with_bytes(context.program_id, &instruction_data, accounts);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&context.maker.pubkey()),
        &[&context.maker],
        context.svm.latest_blockhash(),
    );

    let result = context.svm.send_transaction(tx);
    assert!(result.is_err(), "Checker should fail before end time!");
}

#[test]
fn test_checker_success() {
    let (mut context, maker_ata) = setup_funded_fundraiser();

    let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    let ata_program = Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap();

    let mut clock = context.svm.get_sysvar::<Clock>();
    clock.unix_timestamp += 86400 * 10;
    context.svm.set_sysvar(&clock);

    let instruction_data = vec![2u8];

    let accounts = vec![
        AccountMeta::new(context.maker.pubkey(), true),
        AccountMeta::new_readonly(context.mint, false),
        AccountMeta::new(context.fundraiser_pda, false),
        AccountMeta::new(context.vault, false),
        AccountMeta::new(maker_ata, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(ata_program, false),
    ];

    let ix = Instruction::new_with_bytes(context.program_id, &instruction_data, accounts);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&context.maker.pubkey()),
        &[&context.maker],
        context.svm.latest_blockhash(),
    );

    let result = context.svm.send_transaction(tx);
    assert!(result.is_ok(), "Checker tx failed: {:?}", result.err());

    let amount_to_raise: u64 = 1_000_000_000;
    let maker_ata_account = context.svm.get_account(&maker_ata).unwrap();
    let maker_balance = u64::from_le_bytes(maker_ata_account.data[64..72].try_into().unwrap());
    assert_eq!(maker_balance, amount_to_raise);

    let vault_account = context.svm.get_account(&context.vault);
    assert!(
        vault_account.is_none(),
        "Vault account should be closed and reaped!"
    );

    let pda_account = context.svm.get_account(&context.fundraiser_pda);
    assert!(
        pda_account.is_none(),
        "Fundraiser PDA should be closed and reaped!"
    );
}
