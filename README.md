# xmip-core-contract-asyncapi

AsyncAPI content contract: a sound description always, 2.x or 3.x in JSON, every reference landing; defining a bound channel when a Location names one. A technology of [xmip-core-contract](https://github.com/IlleNilsson/xmip-core-contract).

Each departure names where it is as a JSON Pointer into the description
(`/channels/orders~1placed`), and a `$ref` is percent-decoded before it is one,
both by `xmip-core-contract`.

## The declaration

What a Location naming this contract is configured with is two `const`s in
its `declaration` module, `src/declaration.rs`, beside the code, which its
`ContractFactory` reads (ADR-0064, amendment 2026-11-26):
`declaration::SETTINGS`, `reference`, the channel a description must define,
and `NAME`, `asyncapi`, the name a Validate step's `format` names it by.
Built with its `declaration` feature alone (`--no-default-features
--features declaration`), the crate is those and nothing more: its code and
every crate the code is built on are behind the default feature. So the
library the surfaces load can carry it without the code, for the language
server and the desktop to suggest and check a Location's contract settings
though no node in their process loaded the technology, once the gathering
crate of open problem 48 links it. `test/Declaration.Test.ps1` in the estate
holds that the declaration builds nothing the contract capability does not.

## Toolchain

`rust-toolchain.toml` pins the toolchain for the whole estate. Do not change it
here.

## Verification

The included workflow is manual-only and calls the versioned shared workflow at
`IlleNilsson/.github@v1`.
