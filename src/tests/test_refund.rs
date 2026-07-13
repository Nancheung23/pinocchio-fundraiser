use super::*;
use solana_sdk::{
    clock::Clock,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::str::FromStr;

fn setup_contributed_fundraiser() -> (TestContext, Keypair, Pubkey, Pubkey) {
    let mut context = setup();

    let system_program = Pubkey::from_str("11111111111111111111111111111111").unwrap();
    let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    let ata_program = Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap();

    let amount_to_raise: u64 = 1_000_000_000;
    let duration: u8 = 7;
    let mut init_data = vec![0u8];
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
    context.svm.send_transaction(setup_tx).unwrap();

    let (contributor_pda, _) = Pubkey::find_program_address(
        &[
            b"contributor",
            context.fundraiser_pda.as_ref(),
            contributor.pubkey().as_ref(),
        ],
        &context.program_id,
    );

    let contribute_amount: u64 = 10_000_000;
    let mut contribute_data = vec![1u8]; // 1: Contribute
    contribute_data.extend_from_slice(&contribute_amount.to_le_bytes());

    let contribute_accounts = vec![
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

    let contribute_ix =
        Instruction::new_with_bytes(context.program_id, &contribute_data, contribute_accounts);
    let contribute_tx = Transaction::new_signed_with_payer(
        &[contribute_ix],
        Some(&contributor.pubkey()),
        &[&contributor],
        context.svm.latest_blockhash(),
    );
    context.svm.send_transaction(contribute_tx).unwrap();

    (context, contributor, contributor_ata, contributor_pda)
}

#[test]
fn test_refund_fails_before_end_time() {
    let (mut context, contributor, contributor_ata, contributor_pda) =
        setup_contributed_fundraiser();

    let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    let ata_program = Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap();

    let refund_data = vec![3u8]; // 3: Refund
    let refund_accounts = vec![
        AccountMeta::new(contributor.pubkey(), true),
        AccountMeta::new_readonly(context.maker.pubkey(), false),
        AccountMeta::new_readonly(context.mint, false),
        AccountMeta::new(context.fundraiser_pda, false),
        AccountMeta::new(contributor_pda, false),
        AccountMeta::new(context.vault, false),
        AccountMeta::new(contributor_ata, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(ata_program, false),
    ];

    let refund_ix = Instruction::new_with_bytes(context.program_id, &refund_data, refund_accounts);
    let refund_tx = Transaction::new_signed_with_payer(
        &[refund_ix],
        Some(&contributor.pubkey()),
        &[&contributor],
        context.svm.latest_blockhash(),
    );

    let result = context.svm.send_transaction(refund_tx);
    assert!(result.is_err(), "Refund should fail before end time!");
}

#[test]
fn test_refund_success() {
    let (mut context, contributor, contributor_ata, contributor_pda) =
        setup_contributed_fundraiser();

    let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    let ata_program = Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap();

    let mut clock = context.svm.get_sysvar::<Clock>();
    clock.unix_timestamp += 86400 * 10;
    context.svm.set_sysvar(&clock);

    let refund_data = vec![3u8]; // 3: Refund
    let refund_accounts = vec![
        AccountMeta::new(contributor.pubkey(), true), // contributor
        AccountMeta::new_readonly(context.maker.pubkey(), false), // maker
        AccountMeta::new_readonly(context.mint, false), // mint
        AccountMeta::new(context.fundraiser_pda, false), // fundraiser_pda
        AccountMeta::new(contributor_pda, false),     // contributor_pda
        AccountMeta::new(context.vault, false),       // vault
        AccountMeta::new(contributor_ata, false),     // contributor_ata
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(ata_program, false),
    ];

    let refund_ix = Instruction::new_with_bytes(context.program_id, &refund_data, refund_accounts);
    let refund_tx = Transaction::new_signed_with_payer(
        &[refund_ix],
        Some(&contributor.pubkey()),
        &[&contributor],
        context.svm.latest_blockhash(),
    );

    let result = context.svm.send_transaction(refund_tx);
    assert!(result.is_ok(), "Refund tx failed: {:?}", result.err());

    let pda_account = context.svm.get_account(&contributor_pda);
    assert!(
        pda_account.is_none(),
        "Contributor PDA should be closed and reaped!"
    );

    let ata_account = context.svm.get_account(&contributor_ata).unwrap();
    let final_balance = u64::from_le_bytes(ata_account.data[64..72].try_into().unwrap());
    assert_eq!(final_balance, 100_000_000);
}
