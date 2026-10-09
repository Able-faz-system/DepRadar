#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, token, Address, Env, String,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bounty {
    pub creator: Address,
    pub issue_ref: String,
    pub amount: i128,
    pub token: Address,
    pub released: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Claim {
    pub claimant: Address,
    pub proof_ref: String,
}

#[contracttype]
enum DataKey {
    Admin,
    Bounty(u64),
    Claim(u64),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Error {
    InvalidAmount = 1,
    BountyAlreadyExists = 2,
    BountyNotFound = 3,
    AlreadyClaimed = 4,
    AlreadyInitialized = 5,
    NotInitialized = 6,
    NotAuthorized = 7,
    NoClaimSubmitted = 8,
    AlreadyReleased = 9,
}

#[contract]
pub struct DepRaderRouter;

#[contractimpl]
impl DepRaderRouter {
    /// Sets the contract admin. Must be called once before `release`.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }

        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);

        Ok(())
    }

    /// Locks `amount` of `token` from `creator` into the contract against `bounty_id`.
    pub fn create_bounty(
        env: Env,
        creator: Address,
        bounty_id: u64,
        issue_ref: String,
        amount: i128,
        token: Address,
    ) -> Result<(), Error> {
        creator.require_auth();

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let key = DataKey::Bounty(bounty_id);
        if env.storage().persistent().has(&key) {
            return Err(Error::BountyAlreadyExists);
        }

        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&creator, &env.current_contract_address(), &amount);

        let bounty = Bounty {
            creator,
            issue_ref,
            amount,
            token,
            released: false,
        };
        env.storage().persistent().set(&key, &bounty);

        Ok(())
    }

    /// Reads back the stored state for a bounty, if one exists.
    pub fn get_bounty(env: Env, bounty_id: u64) -> Option<Bounty> {
        env.storage().persistent().get(&DataKey::Bounty(bounty_id))
    }

    /// Records a claim against an existing, not-yet-claimed bounty. Does not move funds.
    pub fn submit_claim(
        env: Env,
        bounty_id: u64,
        claimant: Address,
        proof_ref: String,
    ) -> Result<(), Error> {
        claimant.require_auth();

        if !env.storage().persistent().has(&DataKey::Bounty(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let claim_key = DataKey::Claim(bounty_id);
        if env.storage().persistent().has(&claim_key) {
            return Err(Error::AlreadyClaimed);
        }

        let claim = Claim {
            claimant,
            proof_ref,
        };
        env.storage().persistent().set(&claim_key, &claim);

        Ok(())
    }

    /// Reads back the stored claim for a bounty, if one exists.
    pub fn get_claim(env: Env, bounty_id: u64) -> Option<Claim> {
        env.storage().persistent().get(&DataKey::Claim(bounty_id))
    }

    /// Pays the locked bounty funds out to the claimant and marks the bounty released.
    /// Callable only by the configured admin, and only once per bounty.
    pub fn release(env: Env, admin: Address, bounty_id: u64) -> Result<(), Error> {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        if stored_admin != admin {
            return Err(Error::NotAuthorized);
        }

        let bounty_key = DataKey::Bounty(bounty_id);
        let mut bounty: Bounty = env
            .storage()
            .persistent()
            .get(&bounty_key)
            .ok_or(Error::BountyNotFound)?;

        if bounty.released {
            return Err(Error::AlreadyReleased);
        }

        let claim: Claim = env
            .storage()
            .persistent()
            .get(&DataKey::Claim(bounty_id))
            .ok_or(Error::NoClaimSubmitted)?;

        let token_client = token::Client::new(&env, &bounty.token);
        token_client.transfer(
            &env.current_contract_address(),
            &claim.claimant,
            &bounty.amount,
        );

        bounty.released = true;
        env.storage().persistent().set(&bounty_key, &bounty);

        Ok(())
    }
}

#[cfg(test)]
mod test;
