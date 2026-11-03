use crate::components::merchant;
use crate::shade::ShadeClient;
use soroban_sdk::Address;

/// Links an arbitrary address as a merchant's account, skipping the factory.
/// Production only links accounts it deployed; tests that exercise payment routing
/// rather than the account itself can use a plain address and read its balance.
pub trait LinkMerchantAccount {
    fn link_merchant_account(&self, merchant: &Address, account: &Address);
}

impl LinkMerchantAccount for ShadeClient<'_> {
    fn link_merchant_account(&self, merchant_address: &Address, account: &Address) {
        self.env.as_contract(&self.address, || {
            let merchant_id = merchant::get_merchant_id(&self.env, merchant_address);
            merchant::link_account(&self.env, merchant_id, account);
        });
    }
}
