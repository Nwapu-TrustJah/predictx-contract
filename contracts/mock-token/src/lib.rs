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
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

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
}
