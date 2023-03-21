use std::borrow::Borrow;

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    near_bindgen, require,
    serde::{Deserialize, Serialize},
    store::*,
    AccountId, BorshStorageKey, Promise,
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct Market {
    id: u32,
    is_open: bool,
    description: String,
    owner: AccountId,
    shares: Vector<SharePair>,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Offer {
    id: u32,
    market_id: u32,
    is_long: bool,
    account_id: AccountId,
    amount: U128,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ViewMarket {
    id: u32,
    is_open: bool,
    description: String,
    owner: AccountId,
    shares: u32,
}

impl<T: Borrow<Market>> From<T> for ViewMarket {
    fn from(value: T) -> Self {
        let v = value.borrow();
        Self {
            id: v.id,
            is_open: v.is_open,
            description: v.description.clone(),
            owner: v.owner.clone(),
            shares: v.shares.len(),
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
struct SharePair {
    long: AccountId,
    short: AccountId,
    amount: U128,
}

#[near_bindgen]
pub struct Contract {
    next_offer_id: u32,
    markets: Vector<Market>,
    credit: LookupMap<AccountId, u128>,
    offers: UnorderedMap<u32, Offer>,
}

#[derive(BorshSerialize, BorshStorageKey)]
pub enum StorageKey {
    Markets,
    Offers,
    Credit,
    MarketShares(u32),
}

// This contract doesn't charge for storage costs!
// Eventually, if it consumes enough storage, it could become soft-locked
// until its balance increases or storage usage decreases.
//
// Two solutions:
//  - Implement Storage Management (https://nomicon.io/Standards/StorageManagement)
//  - Charge for storage as it is used (by taking fees from the attached deposit)
//      Example: https://docs.rs/near-sdk-contract-tools/latest/near_sdk_contract_tools/utils/fn.apply_storage_fee_and_refund.html
#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {
            next_offer_id: 0,
            offers: UnorderedMap::new(StorageKey::Offers),
            credit: LookupMap::new(StorageKey::Credit),
            markets: Vector::new(StorageKey::Markets),
        }
    }

    pub fn create_market(&mut self, description: String) -> ViewMarket {
        let id = self.markets.len();
        let m = Market {
            id,
            description,
            owner: env::predecessor_account_id(),
            is_open: true,
            shares: Vector::new(StorageKey::MarketShares(id)),
        };

        let view_market = (&m).into();

        self.markets.push(m);

        view_market
    }

    fn credit_account(&mut self, account_id: AccountId, amount: u128) {
        *self.credit.entry(account_id).or_insert(0) += amount;
    }

    pub fn withdraw(&mut self) -> Promise {
        let predecessor = env::predecessor_account_id();
        let amount = self
            .credit
            .remove(&predecessor)
            .unwrap_or_else(|| env::panic_str("You have no rewards to withdraw."));

        Promise::new(predecessor).transfer(amount)
    }

    pub fn close_market(&mut self, market_id: u32, is_long: bool) {
        let market = self
            .markets
            .get_mut(market_id)
            .unwrap_or_else(|| env::panic_str("Market does not exist!"));
        require!(market.is_open, "Market is already closed.");
        let predecessor = env::predecessor_account_id();
        require!(
            market.owner == predecessor,
            "You are not allowed to close a market you did not create."
        );
        market.is_open = false;

        let credits = market
            .shares
            .iter()
            .map(|s| {
                (
                    if is_long {
                        s.long.clone()
                    } else {
                        s.short.clone()
                    },
                    s.amount,
                )
            })
            .collect::<Vec<_>>();

        drop(market);

        for (creditor, amount) in credits {
            self.credit_account(creditor, amount.0 * 2);
        }
    }

    pub fn get_market(&self, market_id: u32) -> Option<ViewMarket> {
        self.markets.get(market_id).map(|m| m.into())
    }

    pub fn get_offers(&self, market_id: u32) -> Vec<Offer> {
        self.offers
            .iter()
            .filter_map(|(_, b)| {
                if b.market_id == market_id {
                    Some(b.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    #[payable]
    pub fn create_offer(&mut self, market_id: u32, is_long: bool) -> Offer {
        let amount = env::attached_deposit();
        require!(
            amount > 0,
            "You must attach a nonzero amount to make an offer."
        );

        let id = self.next_offer_id;
        self.next_offer_id += 1;
        let o = Offer {
            id,
            is_long,
            account_id: env::predecessor_account_id(),
            amount: amount.into(),
            market_id,
        };

        self.offers.insert(id, o.clone());

        o
    }

    #[payable]
    pub fn accept_offer(&mut self, bid_id: u32) {
        let amount = env::attached_deposit();
        require!(
            amount > 0,
            "You must attach a nonzero amount to accept an offer."
        );
        let amount: U128 = amount.into();

        let o = self.offers.remove(&bid_id).unwrap_or_else(|| {
            env::panic_str("Offer does not exist. Maybe someone already accepted it?")
        });

        require!(
            o.amount == amount,
            "You must attach exactly the same amount as the offer you are accepting."
        );
        let predecessor = env::predecessor_account_id();
        require!(
            predecessor != o.account_id,
            "You cannot accept your own offer."
        );

        let market = self
            .markets
            .get_mut(o.market_id)
            .unwrap_or_else(|| env::panic_str("Market no longer exists!"));

        let (long, short) = if o.is_long {
            (o.account_id, predecessor)
        } else {
            (predecessor, o.account_id)
        };

        market.shares.push(SharePair {
            long,
            short,
            amount: o.amount,
        });
    }
}
