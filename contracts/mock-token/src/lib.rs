#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env, Symbol};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MockTokenError {
    /// Caller is not the admin.
    Unauthorized = 1,
    /// The account does not hold enough tokens.
    InsufficientBalance = 2,
    /// Amount must be greater than zero.
    InvalidAmount = 3,
    /// The token has already been initialised.
    AlreadyInitialized = 4,
    /// The token has not been initialised yet.
    NotInitialized = 5,
    /// Total supply would overflow `i128`.
    SupplyOverflow = 6,
    /// The spender's allowance is too small.
    InsufficientAllowance = 7,
    /// The allowance expiration ledger is invalid.
    InvalidExpiration = 8,
}

/// Storage keys for the mock token.
///
/// The mock is a self-contained test fixture: it keeps its own balance map
/// and total supply instead of depending on the shared token interface
/// (which has not merged yet — see issue #116).
#[contracttype]
pub enum DataKey {
    Admin,
    TotalSupply,
    Balance(Address),
    Allowance(Address, Address),
}

/// An allowance granted via [`MockToken::approve`], mirroring the allowance
/// representation used by the built-in Stellar asset contract.
#[contracttype]
pub struct AllowanceValue {
    pub amount: i128,
    pub expiration_ledger: u32,
}

#[contract]
pub struct MockToken;

#[contractimpl]
impl MockToken {
    /// Initialise the token with an admin who controls minting and burning.
    pub fn initialize(env: Env, admin: Address) -> Result<(), MockTokenError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(MockTokenError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        Ok(())
    }

    fn get_admin(env: &Env) -> Result<Address, MockTokenError> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(MockTokenError::NotInitialized)
    }

    /// Verify `caller` is the admin and require their authorization.
    fn require_admin(env: &Env, caller: &Address) -> Result<(), MockTokenError> {
        let admin = Self::get_admin(env)?;
        if caller != &admin {
            return Err(MockTokenError::Unauthorized);
        }
        caller.require_auth();
        Ok(())
    }

    fn balance_of(env: &Env, account: &Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(account.clone()))
            .unwrap_or(0)
    }

    fn supply(env: &Env) -> i128 {
        env.storage().instance().get(&DataKey::TotalSupply).unwrap_or(0)
    }

    /// Returns the admin address.
    pub fn admin(env: Env) -> Result<Address, MockTokenError> {
        Self::get_admin(&env)
    }

    /// Returns the balance of `account`.
    pub fn balance(env: Env, account: Address) -> i128 {
        Self::balance_of(&env, &account)
    }

    /// Returns the total supply.
    pub fn total_supply(env: Env) -> i128 {
        Self::supply(&env)
    }

    /// Mint `amount` tokens to `to`, increasing both their balance and the
    /// total supply. Admin-only.
    pub fn mint(
        env: Env,
        caller: Address,
        to: Address,
        amount: i128,
    ) -> Result<(), MockTokenError> {
        Self::require_admin(&env, &caller)?;
        if amount <= 0 {
            return Err(MockTokenError::InvalidAmount);
        }
        let supply = Self::supply(&env);
        let new_supply = supply
            .checked_add(amount)
            .ok_or(MockTokenError::SupplyOverflow)?;
        let new_balance = Self::balance_of(&env, &to)
            .checked_add(amount)
            .ok_or(MockTokenError::SupplyOverflow)?;

        env.storage()
            .persistent()
            .set(&DataKey::Balance(to.clone()), &new_balance);
        env.storage().instance().set(&DataKey::TotalSupply, &new_supply);

        env.events().publish((Symbol::new(&env, "mint"),), (to, amount));
        Ok(())
    }

    /// Burn `amount` tokens from `from`, decreasing both their balance and
    /// the total supply. Rejects burning more than `from` holds. Admin-only.
    pub fn burn(
        env: Env,
        caller: Address,
        from: Address,
        amount: i128,
    ) -> Result<(), MockTokenError> {
        Self::require_admin(&env, &caller)?;
        if amount <= 0 {
            return Err(MockTokenError::InvalidAmount);
        }
        let balance = Self::balance_of(&env, &from);
        if balance < amount {
            return Err(MockTokenError::InsufficientBalance);
        }
        // Total supply is always >= any individual balance, so it cannot
        // underflow here.
        let new_supply = Self::supply(&env) - amount;

        env.storage()
            .persistent()
            .set(&DataKey::Balance(from.clone()), &(balance - amount));
        env.storage().instance().set(&DataKey::TotalSupply, &new_supply);

        env.events().publish((Symbol::new(&env, "burn"),), (from, amount));
        Ok(())
    }

    /// Read the live allowance from → spender, treating expired or absent
    /// entries as zero (SEP-41 semantics, matching built-in assets).
    fn live_allowance(env: &Env, from: &Address, spender: &Address) -> i128 {
        let current = env.ledger().sequence();
        let stored: Option<AllowanceValue> = env
            .storage()
            .temporary()
            .get(&DataKey::Allowance(from.clone(), spender.clone()));
        match stored {
            Some(val) if val.expiration_ledger >= current => val.amount,
            _ => 0,
        }
    }

    /// Authorize `spender` to pull up to `amount` tokens from `from` via
    /// [`MockToken::transfer_from`], until `expiration_ledger` (inclusive).
    ///
    /// Matches the SEP-41 `approve` signature, including
    /// `expiration_ledger`. A zero `amount` clears the allowance.
    pub fn approve(
        env: Env,
        from: Address,
        spender: Address,
        amount: i128,
        expiration_ledger: u32,
    ) -> Result<(), MockTokenError> {
        from.require_auth();
        if amount < 0 {
            return Err(MockTokenError::InvalidAmount);
        }
        let current = env.ledger().sequence();
        if amount > 0 && expiration_ledger < current {
            return Err(MockTokenError::InvalidExpiration);
        }

        let key = DataKey::Allowance(from.clone(), spender.clone());
        env.storage().temporary().set(
            &key,
            &AllowanceValue { amount, expiration_ledger },
        );
        // Keep the entry alive until the expiration ledger, like built-in
        // assets bump their allowance entries.
        if expiration_ledger > current {
            let ttl = expiration_ledger - current;
            env.storage().temporary().extend_ttl(&key, ttl, ttl);
        }

        env.events()
            .publish((Symbol::new(&env, "approve"),), (from, spender, amount, expiration_ledger));
        Ok(())
    }

    /// Returns the amount `spender` may still pull from `from`.
    /// Expired allowances count as zero.
    pub fn allowance(env: Env, from: Address, spender: Address) -> i128 {
        Self::live_allowance(&env, &from, &spender)
    }

    /// Pull `amount` tokens from `from` to `to` on behalf of `spender`,
    /// within the allowance previously granted via [`MockToken::approve`].
    ///
    /// Decrements the allowance and the `from` balance; rejects spending
    /// beyond the live allowance or beyond the `from` balance.
    pub fn transfer_from(
        env: Env,
        spender: Address,
        from: Address,
        to: Address,
        amount: i128,
    ) -> Result<(), MockTokenError> {
        spender.require_auth();
        if amount <= 0 {
            return Err(MockTokenError::InvalidAmount);
        }

        let key = DataKey::Allowance(from.clone(), spender.clone());
        let current = env.ledger().sequence();
        let stored: Option<AllowanceValue> = env.storage().temporary().get(&key);
        let live = match &stored {
            Some(val) if val.expiration_ledger >= current => val.amount,
            _ => 0,
        };
        if amount > live {
            return Err(MockTokenError::InsufficientAllowance);
        }

        let from_balance = Self::balance_of(&env, &from);
        if from_balance < amount {
            return Err(MockTokenError::InsufficientBalance);
        }
        let to_balance = Self::balance_of(&env, &to)
            .checked_add(amount)
            .ok_or(MockTokenError::SupplyOverflow)?;

        // Decrement the allowance; drop the entry once it is fully spent.
        let remaining = live - amount;
        if remaining == 0 {
            env.storage().temporary().remove(&key);
        } else if let Some(val) = &stored {
            env.storage().temporary().set(
                &key,
                &AllowanceValue { amount: remaining, expiration_ledger: val.expiration_ledger },
            );
            if val.expiration_ledger > current {
                let ttl = val.expiration_ledger - current;
                env.storage().temporary().extend_ttl(&key, ttl, ttl);
            }
        }

        env.storage()
            .persistent()
            .set(&DataKey::Balance(from.clone()), &(from_balance - amount));
        env.storage()
            .persistent()
            .set(&DataKey::Balance(to.clone()), &to_balance);

        env.events()
            .publish((Symbol::new(&env, "transfer"),), (from, to, amount));
        Ok(())
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger as _};

    fn setup() -> (Env, Address, MockTokenClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(MockToken, ());
        let client = MockTokenClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        (env, admin, client)
    }

    #[test]
    fn initialize_sets_admin_and_rejects_reinitialization() {
        let (env, admin, client) = setup();
        assert_eq!(client.admin(), admin);

        let err = client
            .try_initialize(&Address::generate(&env))
            .expect_err("re-init should fail");
        assert_eq!(err, Ok(MockTokenError::AlreadyInitialized));
    }

    #[test]
    fn mint_increases_recipient_balance_and_total_supply() {
        let (env, admin, client) = setup();
        let user = Address::generate(&env);

        client.mint(&admin, &user, &100);
        assert_eq!(client.balance(&user), 100);
        assert_eq!(client.total_supply(), 100);

        client.mint(&admin, &user, &50);
        assert_eq!(client.balance(&user), 150);
        assert_eq!(client.total_supply(), 150);
    }

    #[test]
    fn burn_decreases_balance_and_total_supply() {
        let (env, admin, client) = setup();
        let user = Address::generate(&env);

        client.mint(&admin, &user, &100);
        client.burn(&admin, &user, &40);
        assert_eq!(client.balance(&user), 60);
        assert_eq!(client.total_supply(), 60);
    }

    #[test]
    fn burn_rejects_amount_greater_than_held() {
        let (env, admin, client) = setup();
        let user = Address::generate(&env);

        client.mint(&admin, &user, &50);
        let err = client
            .try_burn(&admin, &user, &60)
            .expect_err("burning more than held should fail");
        assert_eq!(err, Ok(MockTokenError::InsufficientBalance));

        // State is unchanged after the failed burn.
        assert_eq!(client.balance(&user), 50);
        assert_eq!(client.total_supply(), 50);
    }

    #[test]
    fn non_admin_cannot_mint_or_burn() {
        let (env, _admin, client) = setup();
        let stranger = Address::generate(&env);
        let user = Address::generate(&env);

        let err = client
            .try_mint(&stranger, &user, &100)
            .expect_err("non-admin mint should fail");
        assert_eq!(err, Ok(MockTokenError::Unauthorized));

        let err = client
            .try_burn(&stranger, &user, &100)
            .expect_err("non-admin burn should fail");
        assert_eq!(err, Ok(MockTokenError::Unauthorized));

        assert_eq!(client.total_supply(), 0);
    }

    #[test]
    fn mint_and_burn_reject_non_positive_amounts() {
        let (env, admin, client) = setup();
        let user = Address::generate(&env);

        let err = client
            .try_mint(&admin, &user, &0)
            .expect_err("zero mint should fail");
        assert_eq!(err, Ok(MockTokenError::InvalidAmount));

        let err = client
            .try_mint(&admin, &user, &-5)
            .expect_err("negative mint should fail");
        assert_eq!(err, Ok(MockTokenError::InvalidAmount));

        let err = client
            .try_burn(&admin, &user, &0)
            .expect_err("zero burn should fail");
        assert_eq!(err, Ok(MockTokenError::InvalidAmount));
    }

    #[test]
    fn burn_from_account_with_zero_balance_is_rejected() {
        let (env, admin, client) = setup();
        let user = Address::generate(&env);

        let err = client
            .try_burn(&admin, &user, &1)
            .expect_err("burning from an empty account should fail");
        assert_eq!(err, Ok(MockTokenError::InsufficientBalance));
    }

    // ── Allowance (issue #117) ─────────────────────────────────────────────

    #[test]
    fn approve_grants_and_allowance_reports_it() {
        let (env, _admin, client) = setup();
        let owner = Address::generate(&env);
        let spender = Address::generate(&env);

        client.mint(&_admin, &owner, &1_000);
        assert_eq!(client.allowance(&owner, &spender), 0);

        client.approve(&owner, &spender, &300, &(env.ledger().sequence() + 100));
        assert_eq!(client.allowance(&owner, &spender), 300);

        // Re-approving replaces the previous allowance; zero clears it.
        client.approve(&owner, &spender, &50, &(env.ledger().sequence() + 100));
        assert_eq!(client.allowance(&owner, &spender), 50);
        client.approve(&owner, &spender, &0, &(env.ledger().sequence() + 100));
        assert_eq!(client.allowance(&owner, &spender), 0);
    }

    #[test]
    fn transfer_from_within_allowance_succeeds_and_decrements() {
        let (env, admin, client) = setup();
        let owner = Address::generate(&env);
        let spender = Address::generate(&env);
        let to = Address::generate(&env);

        client.mint(&admin, &owner, &500);
        client.approve(&owner, &spender, &300, &(env.ledger().sequence() + 100));

        client.transfer_from(&spender, &owner, &to, &250);
        assert_eq!(client.balance(&owner), 250);
        assert_eq!(client.balance(&to), 250);
        assert_eq!(client.allowance(&owner, &spender), 50);

        // Supply is unchanged by a transfer.
        assert_eq!(client.total_supply(), 500);
    }

    #[test]
    fn transfer_from_beyond_allowance_is_rejected() {
        let (env, admin, client) = setup();
        let owner = Address::generate(&env);
        let spender = Address::generate(&env);
        let to = Address::generate(&env);

        client.mint(&admin, &owner, &500);
        client.approve(&owner, &spender, &100, &(env.ledger().sequence() + 100));

        let err = client
            .try_transfer_from(&spender, &owner, &to, &101)
            .expect_err("spending beyond the allowance should fail");
        assert_eq!(err, Ok(MockTokenError::InsufficientAllowance));

        // A second spend that would exceed the *remaining* allowance fails too.
        client.transfer_from(&spender, &owner, &to, &100);
        let err = client
            .try_transfer_from(&spender, &owner, &to, &1)
            .expect_err("exhausted allowance should fail");
        assert_eq!(err, Ok(MockTokenError::InsufficientAllowance));

        // Nothing moved beyond the first transfer.
        assert_eq!(client.balance(&owner), 400);
        assert_eq!(client.balance(&to), 100);
    }

    #[test]
    fn transfer_from_beyond_balance_is_rejected() {
        let (env, admin, client) = setup();
        let owner = Address::generate(&env);
        let spender = Address::generate(&env);
        let to = Address::generate(&env);

        client.mint(&admin, &owner, &50);
        client.approve(&owner, &spender, &100, &(env.ledger().sequence() + 100));

        let err = client
            .try_transfer_from(&spender, &owner, &to, &60)
            .expect_err("spending beyond the balance should fail");
        assert_eq!(err, Ok(MockTokenError::InsufficientBalance));
    }

    #[test]
    fn expired_allowance_is_treated_as_zero() {
        let (env, admin, client) = setup();
        let owner = Address::generate(&env);
        let spender = Address::generate(&env);
        let to = Address::generate(&env);

        client.mint(&admin, &owner, &500);
        client.approve(&owner, &spender, &300, &(env.ledger().sequence() + 5));
        assert_eq!(client.allowance(&owner, &spender), 300);

        env.ledger().with_mut(|li| li.sequence_number += 6);

        assert_eq!(client.allowance(&owner, &spender), 0);
        let err = client
            .try_transfer_from(&spender, &owner, &to, &1)
            .expect_err("an expired allowance should not be spendable");
        assert_eq!(err, Ok(MockTokenError::InsufficientAllowance));
    }

    #[test]
    fn transfer_from_requires_spender_auth() {
        let (env, admin, client) = setup();
        let owner = Address::generate(&env);
        let spender = Address::generate(&env);
        let to = Address::generate(&env);

        client.mint(&admin, &owner, &500);
        client.approve(&owner, &spender, &300, &(env.ledger().sequence() + 100));

        env.set_auths(&[]);
        let err = client
            .try_transfer_from(&spender, &owner, &to, &10)
            .expect_err("transfer_from must require the spender's authorization");
        assert_eq!(err, Err(soroban_sdk::InvokeError::Abort));
    }

    #[test]
    fn approve_requires_owner_auth_and_validates_arguments() {
        let (env, _admin, client) = setup();
        let owner = Address::generate(&env);
        let spender = Address::generate(&env);

        // Owner must authorize the approval.
        env.set_auths(&[]);
        let err = client
            .try_approve(&owner, &spender, &100, &(env.ledger().sequence() + 10))
            .expect_err("approve must require the owner's authorization");
        assert_eq!(err, Err(soroban_sdk::InvokeError::Abort));

        env.mock_all_auths();

        // Negative amounts and past expirations (for non-zero amounts) are rejected.
        let err = client
            .try_approve(&owner, &spender, &-1, &(env.ledger().sequence() + 10))
            .expect_err("negative approval should fail");
        assert_eq!(err, Ok(MockTokenError::InvalidAmount));

        env.ledger().with_mut(|li| li.sequence_number = 10);
        let err = client
            .try_approve(&owner, &spender, &100, &9_u32)
            .expect_err("past expiration should fail for a non-zero amount");
        assert_eq!(err, Ok(MockTokenError::InvalidExpiration));
    }
}
