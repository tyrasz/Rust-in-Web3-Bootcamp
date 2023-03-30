# Sample smart contract

This is a sample smart contract that you can use as a starting point or reference to creating your own.

However, it's far from perfect! If you choose to use this contract for your project, you should know that there are a number of issues with it, and it's missing a lot of features.

## Issues

### Incomplete events

The contract emits events for most interesting actions, but not for withdrawals or credits.

### Share enumeration

There's no way for a user to (easily) check what shares he owns!

### Storage staking

This contract doesn't charge for storage costs! Eventually, if it consumes enough storage, it could become soft-locked until its balance increases or storage usage decreases.

Two solutions:

- Implement Storage Management (https://nomicon.io/Standards/StorageManagement)
- Charge for storage as it is used (by taking fees from the attached deposit)
  Example: https://docs.rs/near-sdk-contract-tools/latest/near_sdk_contract_tools/utils/fn.apply_storage_fee_and_refund.html

### Share transfer

Users cannot transfer shares to other users, so it's really less of a market and more of a casino. Users should be able to transfer and sell their shares to other users to make the market more dynamic.

Ideas:

- Should this contract implement a token standard?
  - Should that be the FT or NFT standard?
- Should the internal data structure be changed?

### Oracle integration

Currently, this contract requires that oracles send transactions to the contract to resolve markets. This might work, if oracles decide to add functionality that interfaces directly with this contract (or someone else writes a catalyst contract), but it might be better if this contract were able to reach out to oracles to resolve markets directly.
