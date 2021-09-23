# motor

`motor` (German for 'engine') is a playground for blockchain experiments and exploration based on [Substrate](https://github.com/paritytech/substrate) and mostly related to block authoring and finalization.

Note that `motor` follows [Substrate](https://github.com/paritytech/substrate) `master`. We do check in `Cargo.lock` for  a somewhat
controlled update process of [Substrate](https://github.com/paritytech/substrate), however.

<br>

## Engine Parts

* [Arber](./arber) is a Merkle-Mountain-Range pallet utilizing the [arber](https://github.com/adoerr/arber) library.

* [Emptor](./emptor) is a mock service client for testing purposes.

* [Falso](./falso) is a mock network for testing purposes.

* [Simplex](./simplex) is a minimal block authoring and finalization engine.



