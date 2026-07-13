use super::*;
use crate::state::Fundraiser;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::Signer,
    transaction::Transaction,
};
use std::str::FromStr;

#[test]
fn test_initialize_success() {
    let mut context = setup();

    let amount_to_raise: u64 = 10_000_000;
    let duration: u8 = 7;

    let mut instruction_data = vec![0u8];
    instruction_data.extend_from_slice(&amount_to_raise.to_le_bytes());
    instruction_data.push(duration);

    let system_program = Pubkey::from_str("11111111111111111111111111111111").unwrap();
    let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    let associated_token_program =
        Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap();

    let accounts = vec![
        AccountMeta::new(context.maker.pubkey(), true),
        AccountMeta::new_readonly(context.mint, false),
        AccountMeta::new(context.fundraiser_pda, false),
        AccountMeta::new(context.vault, false),
        AccountMeta::new_readonly(system_program, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(associated_token_program, false),
    ];

    let ix = Instruction::new_with_bytes(context.program_id, &instruction_data, accounts);

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&context.maker.pubkey()),
        &[&context.maker],
        context.svm.latest_blockhash(),
    );

    let result = context.svm.send_transaction(tx);
    assert!(result.is_ok(), "Initialize tx failed: {:?}", result.err());

    let account = context.svm.get_account(&context.fundraiser_pda).unwrap();
    let fundraiser_state = bytemuck::from_bytes::<Fundraiser>(&account.data);

    assert_eq!(fundraiser_state.maker, context.maker.pubkey().to_bytes());
    assert_eq!(fundraiser_state.mint_to_raise, context.mint.to_bytes());
    assert_eq!(fundraiser_state.amount_to_raise, amount_to_raise);
    assert_eq!(fundraiser_state.current_amount, 0);
    assert_eq!(fundraiser_state.duration, duration);
    assert!(fundraiser_state.time_started == 0);

    let vault_account = context.svm.get_account(&context.vault).unwrap();
    assert_eq!(vault_account.owner, token_program);
}
