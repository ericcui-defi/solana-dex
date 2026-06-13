use {
    anchor_lang::{
        solana_program::{
            instruction::Instruction,
            program_pack::Pack,
            pubkey::Pubkey,
            system_instruction,
            system_program,
        },
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    anchor_spl::token::spl_token::{self, instruction as spl_ix, state::Mint as SplMint, state::Account},
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

// Function to create a mint and return it's public address
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

// Function to create a token account and return its public address for a user
fn create_token_account(svm: &mut LiteSVM, payer: &Keypair, mint: &Pubkey, owner: &Pubkey) -> Pubkey {
    let user_account = Keypair::new();
    let rent = svm.minimum_balance_for_rent_exemption(Account::LEN);
    let ixs = [
        system_instruction::create_account(
            &payer.pubkey(),
            &user_account.pubkey(),
            rent,
            Account::LEN as u64,
            &spl_token::ID,
        ),
        spl_ix::initialize_account(
            &spl_token::ID,
            &user_account.pubkey(),
            mint,
            owner,
        ).unwrap(),
    ];
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&ixs, Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[payer, &user_account]).unwrap();
    svm.send_transaction(tx).unwrap();
    user_account.pubkey()
}

fn mint_tokens(svm: &mut LiteSVM, payer: &Keypair, mint: &Pubkey, token_account: &Pubkey, amount: u64) {
    let ixs = [
        spl_ix::mint_to(
            &spl_token::ID,
            mint,
            token_account,
            &payer.pubkey(),
            &[],
            amount,
        ).unwrap(),
    ];
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&ixs, Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[payer]).unwrap();
    svm.send_transaction(tx).unwrap();
}


#[test]
fn test_swap() {

    // Chain intialization
    let program_id = dex::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/dex.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 5_000_000_000).unwrap();

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
        &program_id
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
            token_program: spl_token::ID
        }.to_account_metas(None),
    );

    // Initializing program
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer]).unwrap();

    let res = svm.send_transaction(tx);
    assert!(res.is_ok());

    // Create token accounts
    let user_a = create_token_account(&mut svm, &payer, &mint_a, &payer.pubkey());
    let user_b = create_token_account(& mut svm, &payer, &mint_b, &payer.pubkey());

    let balance_a = 10000;
    let balance_b = 10000;
    let add_a = 5000;
    let add_b = 5000;

    // Funding token accounts
    // u64 implemented Copy so we can just directly pass in the variable names (they are not consumed by the function)
    mint_tokens(&mut svm, &payer, &mint_a, &user_a, balance_a);
    mint_tokens(&mut svm, &payer, &mint_b, &user_b, balance_b);

    // Creating user lp account
    let user_lp = create_token_account(&mut svm, &payer, &lp_mint, &payer.pubkey());

    let add_liquidity_instruction = Instruction::new_with_bytes(
        program_id,
        &dex::instruction::AddLiquidity { a_amount: add_a, b_amount: add_b, min_lp_out: 1}.data(),
        dex::accounts::AddLiquidity {
            user: payer.pubkey(),
            pool: pool,
            token_vault_a: token_vault_a,
            token_vault_b: token_vault_b,
            user_a: user_a,
            user_b: user_b,
            user_lp: user_lp,
            lp_mint: lp_mint,
            token_mint_a: mint_a,
            token_mint_b: mint_b,
            token_program: spl_token::ID
        }.to_account_metas(None),
    );

    // Running liquidity add
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[add_liquidity_instruction], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer]).unwrap();
    let res = svm.send_transaction(tx);

    // Assertions
    if let Err(e) = &res {
        panic!("add_liquidity failed: {:?}\nlogs:\n{}", e.err, e.meta.logs.join("\n"));
    }

    // Swap instruction
    let swap_instruction = Instruction::new_with_bytes(
        program_id,
        &dex::instruction::Swap { a_to_b: true, in_amount: 100, min_out: 10 }.data(),
        dex::accounts::Swap {
            user: payer.pubkey(),
            pool: pool,
            token_vault_a: token_vault_a,
            token_vault_b: token_vault_b,
            user_a: user_a,
            user_b: user_b,
            lp_mint: lp_mint,
            token_mint_a: mint_a,
            token_mint_b: mint_b,
            token_program: spl_token::ID
        }.to_account_metas(None),
    ); 

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[swap_instruction], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer]).unwrap();
    let res = svm.send_transaction(tx);

    // Assertions
    if let Err(e) = &res {
        panic!("swap failed: {:?}\nlogs:\n{}", e.err, e.meta.logs.join("\n"));
    }

    // Assert users A token balance is 100 less
    let data = svm.get_account(&user_a).unwrap().data;
    let user_a_state = Account::unpack(&data).unwrap();
    assert_eq!(user_a_state.amount, 4900);

    // Asserting user got paid the appropriate number of B tokens
    let data = svm.get_account(&user_b).unwrap().data;
    let user_b_state = Account::unpack(&data).unwrap();
    assert_eq!(user_b_state.amount, 5097);

    let data = svm.get_account(&pool).unwrap().data;
    let pool_state = dex::Pool::try_deserialize(&mut data.as_slice()).unwrap();
    assert_eq!(pool_state.reserve_a, 5100);
    assert_eq!(pool_state.reserve_b, 4903);
}