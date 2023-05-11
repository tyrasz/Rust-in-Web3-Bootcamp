use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    near_bindgen, require,
    serde::{Deserialize, Serialize},
    store::*,
    AccountId, BorshStorageKey, PanicOnDefault, Promise,
};
use near_sdk_contract_tools::{
    event,
    standard::{
        nep141::{Nep141Hook, Nep141Resolver, Nep141},
        nep297::Event,
    },
    FungibleToken,
};

const TOKEN_CONTRACT_WASM: &[u8] = include_bytes!("../../target/wasm32-unknown-unknown/release/token_contract.wasm");

#[event(
    standard = "x-predictions-market",
    version = "0.1.0",
    serde = "near_sdk::serde"
)]

enum ContractEvent {
    MarketCreated {
        market_id: u32,
        owner: AccountId,
    },
    OfferCreated {
        offer_id: u32,
        market_id: u32,
        is_long: bool,
        account_id: AccountId,
        amount: U128,
    },
    OfferAccepted {
        offer_id: u32,
        market_id: u32,
        account_id: AccountId,
    },
    MarketClosed {
        market_id: u32,
    },
    Credits {
        account_id: AccountId,
        amount: U128,
    },
    Withdrawals {
        account_id: AccountId,
        amount: U128,
    },
}


#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Market {
    id: u32,
    is_open: bool,
    description: String,
    owner: AccountId,
    shares: Vector<SharePair>,
    token_contract: AccountId,  // add this field
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

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ViewMarket<'a> {
    id: u32,
    is_open: bool,
    description: &'a str,
    owner: &'a AccountId,
    shares: u32,
}

impl<'a> From<&'a Market> for ViewMarket<'a> {
    fn from(v: &'a Market) -> Self {
        Self {
            id: v.id,
            is_open: v.is_open,
            description: &v.description,
            owner: &v.owner,
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

#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault, FungibleToken)]
#[fungible_token(name = "Options Market Token", symbol = "OMT", decimals = 24)]
#[near_bindgen]
pub struct Contract {
    next_offer_id: u32,
    markets: Vector<Market>,
    credit: LookupMap<AccountId, u128>,
    offers: UnorderedMap<u32, Offer>,
}

impl Nep141Hook for Contract {
    // fn nep141_on_transfer(
    //     &mut self,
    //     _sender_id: AccountId,
    //     _receiver_id: AccountId,
    //     _amount: U128,
    //     _msg: String,
    // ) {
    //     env::panic_str("This contract does not accept tokens.");
    // }

    fn before_transfer(
        &mut self,
        _transfer: &near_sdk_contract_tools::standard::nep141::Nep141Transfer,
    ) -> () {
        Default::default()
    }

    fn after_transfer(
        &mut self,
        _transfer: &near_sdk_contract_tools::standard::nep141::Nep141Transfer,
        _state: (),
    ) {
    }
}

#[derive(BorshSerialize, BorshStorageKey)]
pub enum StorageKey {
    Markets,
    Offers,
    Credit,
    MarketShares(u32),
    Withdrawals(u32),
}

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

    pub fn create_market(&mut self, description: String, outcomes: Vec<String>, token_contract: AccountId) -> ViewMarket {
        let id = self.markets.len();
        let owner = env::predecessor_account_id();

        let m = Market {
            id,
            description,
            owner: owner.clone(),
            is_open: true,
            shares: Vector::new(StorageKey::MarketShares(id)),
        };

        // Mint tokens
        let promise = Promise::new(token_contract.clone())
            .function_call(
                "mint_tokens".into(),
                format!("{{\"account_id\": \"{}\", \"amount\": \"{}\"}}", env::current_account_id(), total_supply).into_bytes(),
                0,
                GAS_FOR_MINT_AND_BURN,
            );

        self.markets.insert(&market_id, &Market {
            // ...
            token_contract,
        });  
        
        let account_id: AccountId = format!("{}.{}", 0, env::current_account_id()).parse().unwrap();
        Promise::new(account_id.clone()).deploy_contract(TOKEN_CONTRACT_WASM.to_vec()).then(Self::ext(env::current_account_id()).after_market_create(/* ... */));

        ContractEvent::MarketCreated {
            market_id: id,
            owner,
        }
        .emit();

        self.markets.get(id).unwrap().into()
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

        ContractEvent::MarketClosed { market_id }.emit();

        let promise = Promise::new(market.token_contract.clone())
            .function_call(
                "burn_tokens".into(),
                format!("{{\"account_id\": \"{}\", \"amount\": \"{}\"}}", env::current_account_id(), total_supply).into_bytes(),
                0,
                GAS_FOR_MINT_AND_BURN,
            );

        drop(market);

        for (creditor, amount) in credits {
            self.credit_account(creditor, amount.0 * 2);
        }
    }

    pub fn get_market(&self, market_id: u32) -> Option<ViewMarket> {
        self.markets.get(market_id).map(|m| m.into())
    }

    pub fn list_markets(&self) -> Vec<ViewMarket> {
        self.markets.iter().map(|m| m.into()).collect()
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
        let account_id = env::predecessor_account_id();
        let o = Offer {
            id,
            is_long,
            account_id: account_id.clone(),
            amount: amount.into(),
            market_id,
        };

        self.offers.insert(id, o.clone());

        ContractEvent::OfferCreated {
            offer_id: id,
            market_id,
            is_long,
            account_id,
            amount: amount.into(),
        }
        .emit();

        o
    }

    #[payable]
    pub fn accept_offer(&mut self, offer_id: u32) {
        let amount = env::attached_deposit();
        require!(
            amount > 0,
            "You must attach a nonzero amount to accept an offer."
        );
        let amount: U128 = amount.into();

        let o = self.offers.remove(&offer_id).unwrap_or_else(|| {
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

        ContractEvent::OfferAccepted {
            offer_id,
            market_id: o.market_id,
            account_id: predecessor.clone(),
        }
        .emit();

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
