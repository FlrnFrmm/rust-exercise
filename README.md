# Rust Exercise

An application that that processes transaction data.

First, a `collector_thread` is spawned that reads the transaction data line by line from a CSV file. These transactions are sent via `channel` to the `PaymentsEngine`.

The `PaymentsEngine` evaluates each incoming transaction and creates/maintains the state of the different accounts.

When the `collector_thread` reaches the end of the file, the final state of each account is written as CSV to stdout by the `PaymentsEngine`.

## Assumptions

### Frozen accounts

As soon as an account is 'locked' it ignores all further transactions.

### Multiple disputes are not possible

If a transaction is already in dispute, further disputes on that transaction have no effect.

## Tests

### With test data

In `./csv` are some CSV files, that were used to verify that the application behaves according to the specification.

### Automated Tests

There are also some tests included in `crate::account::Account` that check against all basic rules of the specification.

## Run

`cargo run -- ./path/to/input.csv > output.csv`
