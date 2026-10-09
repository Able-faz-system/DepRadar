use super::{DepRaderRouter, DepRaderRouterClient, Error};
use soroban_sdk::{
    testutils::Address as _,
    token::{StellarAssetClient, TokenClient},
    Address, Env, String,
};

fn create_token_contract<'a>(
    env: &Env,
    admin: &Address,
) -> (TokenClient<'a>, StellarAssetClient<'a>) {
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    (
        TokenClient::new(env, &sac.address()),
        StellarAssetClient::new(env, &sac.address()),
    )
}

#[test]
fn test_create_bounty_locks_funds() {
    let env = Env::default();
    env.mock_all_auths();

    let creator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    token_admin_client.mint(&creator, &1_000);

    let contract_id = env.register(DepRaderRouter, ());
    let client = DepRaderRouterClient::new(&env, &contract_id);

    let bounty_id = 1u64;
    let issue_ref = String::from_str(&env, "org/repo#123");
    let amount: i128 = 400;

    client.create_bounty(
        &creator,
        &bounty_id,
        &issue_ref,
        &amount,
        &token_client.address,
    );

    // Funds moved from the creator into the contract.
    assert_eq!(token_client.balance(&creator), 600);
    assert_eq!(token_client.balance(&contract_id), 400);

    // Bounty state is readable via the getter.
    let bounty = client.get_bounty(&bounty_id).expect("bounty should exist");
    assert_eq!(bounty.creator, creator);
    assert_eq!(bounty.issue_ref, issue_ref);
    assert_eq!(bounty.amount, amount);
    assert_eq!(bounty.token, token_client.address);
}

#[test]
fn test_submit_claim_happy_path() {
    let env = Env::default();
    env.mock_all_auths();

    let creator = Address::generate(&env);
    let claimant = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    token_admin_client.mint(&creator, &1_000);

    let contract_id = env.register(DepRaderRouter, ());
    let client = DepRaderRouterClient::new(&env, &contract_id);

    let bounty_id = 1u64;
    let issue_ref = String::from_str(&env, "org/repo#123");
    client.create_bounty(
        &creator,
        &bounty_id,
        &issue_ref,
        &400,
        &token_client.address,
    );

    let proof_ref = String::from_str(&env, "org/repo#123-pr-456");
    client.submit_claim(&bounty_id, &claimant, &proof_ref);

    // No funds move on a claim.
    assert_eq!(token_client.balance(&contract_id), 400);

    let claim = client.get_claim(&bounty_id).expect("claim should exist");
    assert_eq!(claim.claimant, claimant);
    assert_eq!(claim.proof_ref, proof_ref);
}

#[test]
fn test_submit_claim_nonexistent_bounty() {
    let env = Env::default();
    env.mock_all_auths();

    let claimant = Address::generate(&env);
    let contract_id = env.register(DepRaderRouter, ());
    let client = DepRaderRouterClient::new(&env, &contract_id);

    let proof_ref = String::from_str(&env, "org/repo#999-pr-1");
    let result = client.try_submit_claim(&999u64, &claimant, &proof_ref);

    assert_eq!(result, Err(Ok(Error::BountyNotFound)));
    assert!(client.get_claim(&999u64).is_none());
}

/// Registers the contract, initializes it with `admin`, creates a bounty funded by
/// `creator`, and (unless `with_claim` is false) has `claimant` submit a claim for it.
fn setup<'a>(
    env: &Env,
    admin: &Address,
    creator: &Address,
    claimant: &Address,
    with_claim: bool,
) -> (DepRaderRouterClient<'a>, TokenClient<'a>, u64, i128) {
    let token_admin = Address::generate(env);
    let (token_client, token_admin_client) = create_token_contract(env, &token_admin);
    token_admin_client.mint(creator, &1_000);

    let contract_id = env.register(DepRaderRouter, ());
    let client = DepRaderRouterClient::new(env, &contract_id);
    client.initialize(admin);

    let bounty_id = 1u64;
    let issue_ref = String::from_str(env, "org/repo#123");
    let amount: i128 = 400;
    client.create_bounty(
        creator,
        &bounty_id,
        &issue_ref,
        &amount,
        &token_client.address,
    );

    if with_claim {
        let proof_ref = String::from_str(env, "org/repo#123-pr-456");
        client.submit_claim(&bounty_id, claimant, &proof_ref);
    }

    (client, token_client, bounty_id, amount)
}

#[test]
fn test_release_happy_path() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let claimant = Address::generate(&env);
    let (client, token_client, bounty_id, amount) = setup(&env, &admin, &creator, &claimant, true);

    client.release(&admin, &bounty_id);

    // Funds moved from the contract to the claimant.
    assert_eq!(token_client.balance(&client.address), 0);
    assert_eq!(token_client.balance(&claimant), amount);

    let bounty = client.get_bounty(&bounty_id).expect("bounty should exist");
    assert!(bounty.released);
}

#[test]
fn test_release_no_claim() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let claimant = Address::generate(&env);
    let (client, _token_client, bounty_id, _amount) =
        setup(&env, &admin, &creator, &claimant, false);

    let result = client.try_release(&admin, &bounty_id);

    assert_eq!(result, Err(Ok(Error::NoClaimSubmitted)));
}

#[test]
fn test_release_twice() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let claimant = Address::generate(&env);
    let (client, _token_client, bounty_id, _amount) =
        setup(&env, &admin, &creator, &claimant, true);

    client.release(&admin, &bounty_id);
    let result = client.try_release(&admin, &bounty_id);

    assert_eq!(result, Err(Ok(Error::AlreadyReleased)));
}

#[test]
fn test_release_non_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let claimant = Address::generate(&env);
    let (client, token_client, bounty_id, _amount) = setup(&env, &admin, &creator, &claimant, true);

    let not_admin = Address::generate(&env);
    let result = client.try_release(&not_admin, &bounty_id);

    assert_eq!(result, Err(Ok(Error::NotAuthorized)));
    // Funds stay locked in the contract.
    assert_eq!(token_client.balance(&claimant), 0);
    let bounty = client.get_bounty(&bounty_id).expect("bounty should exist");
    assert!(!bounty.released);
}
