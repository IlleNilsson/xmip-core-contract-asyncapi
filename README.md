# xmip-core-contract-asyncapi

AsyncAPI content contract: a sound description always, 2.x or 3.x in JSON, every reference landing; defining a bound channel when a Location names one. A technology of [xmip-core-contract](https://github.com/IlleNilsson/xmip-core-contract).

Each departure names where it is as a JSON Pointer into the description
(`/channels/orders~1placed`), and a `$ref` is percent-decoded before it is one,
both by `xmip-core-contract`.

## Toolchain

`rust-toolchain.toml` pins the toolchain for the whole estate. Do not change it
here.

## Verification

The included workflow is manual-only and calls the versioned shared workflow at
`IlleNilsson/.github@v1`.
