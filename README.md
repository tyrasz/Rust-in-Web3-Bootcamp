# NEAR Dapp Template

## Prerequisites

You need to have the [Node.js](https://nodejs.org/) and [Rust](https://www.rust-lang.org/) programming languages installed.

Install the WASM target for Rust:

```bash
rustup target add wasm32-unknown-unknown
```

You will also probably want:

- [Git](https://git-scm.com/)
- [NEAR CLI](https://docs.near.org/tools/near-cli)
  ```bash
  npm install -g near-cli
  ```
- [`cargo make`](https://github.com/sagiegurari/cargo-make)
  ```bash
  cargo install --force cargo-make
  ```

## Setup

1. Build the smart contract

   ```bash
   cargo build --target wasm32-unknown-unknown --release
   ```

   or (if you installed `cargo-make`)

   ```bash
   cargo make build
   ```

2. Deploy the smart contract

   ```bash
   near dev-deploy --wasmFile target/wasm32-unknown-unknown/release/contract.wasm
   ```

   or (if you installed `cargo-make`)

   ```bash
   cargo make dev-deploy
   ```

   Be sure to remember the contract account ID!

3. Start the frontend

   ```bash
   cd frontend
   npm install
   CONTRACT_ID=<contract-id> npm run dev
   ```

   Where `<contract-id>` is the contract account ID from step 2.
