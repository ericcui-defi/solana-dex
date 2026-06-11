use {
    anchor_lang::{
        solana_program::{
            instruction::Instruction,
            program_pack::Pack,
            pubkey::Pubkey,
            system_instruction,
            system_program,
        },
        InstructionData, ToAccountMetas,
    },
    anchor_spl::token::spl_token::{self, instruction as spl_ix, state::Mint as SplMint},
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

fn create_mint(svm: &mut LiteSVM, payer: &Keypair, decimals: u8) -> Pubkey {
    let mint = Keypair::new();
    let rent = svm.minimum_balance_for_rent_exemption(SplMint::LEN);
    let ixs = [
        system_instruction::create_account(
            &payer.pubkey(),
            &mint.pubkey(),
            rent,
            SplMint::LEN as u64,
            &spl_token::ID,
        ),
        spl_ix::initialize_mint(
            &spl_token::ID,
            &mint.pubkey(),
            &payer.pubkey(),
            None,
            decimals,
        ).unwrap(),
    ];
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&ixs, Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[payer, &mint]).unwrap();
    svm.send_transaction(tx).unwrap();
    mint.pubkey()
}

#[test]
fn test_initialize() {
    let program_id = dex::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/dex.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();

    // Creating mock mints for the two tokens in the pool
    let mint_a = create_mint(&mut svm, &payer, 6);
    let mint_b = create_mint(&mut svm, &payer, 6);

    // Sorting?
    let (mint_a, mint_b) = if mint_a < mint_b { 
        (mint_a, mint_b) 
    } else { 
        (mint_b, mint_a) 
    };

    // Deriving PDAs
    let (pool, _) = Pubkey::find_program_address(
        &[b"pool", mint_a.as_ref(), mint_b.as_ref()], 
        &program_id
    );
    let (token_vault_a, _) = Pubkey::find_program_address(
        &[b"vault_a", pool.as_ref()], 
        &program_id
    );
    let (token_vault_b, _) = Pubkey::find_program_address(
        &[b"vault_b", pool.as_ref()],
        &program_id
    );
    let (lp_mint, _) = Pubkey::find_program_address(
        &[b"lp", pool.as_ref()],
        &program_id,
    );

    let instruction = Instruction::new_with_bytes(
        program_id,
        &dex::instruction::Initialize { fee_bps: 30 }.data(),
        dex::accounts::Initialize {
            payer: payer.pubkey(),
            token_mint_a: mint_a,
            token_mint_b: mint_b,
            pool: pool,
            token_vault_a: token_vault_a,
            token_vault_b: token_vault_b,
            lp_mint: lp_mint,
            system_program: system_program::ID,
            token_program: spl_token::ID,
        }.to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[payer]).unwrap();

    let res = svm.send_transaction(tx);
    assert!(res.is_ok());
}
