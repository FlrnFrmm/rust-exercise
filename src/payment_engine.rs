use crate::{account::Account, transaction::Transaction};
use anyhow::{Error, Result};
use std::collections::HashMap;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub struct PaymentsEngine {
    accounts: HashMap<u16, Account>,
    transactions: Receiver<Transaction>,
}

impl PaymentsEngine {
    pub fn new() -> (Self, Sender<Transaction>) {
        let (transaction_sink, transactions) = channel::<Transaction>(16);
        let accounts = HashMap::new();

        (
            Self {
                accounts,
                transactions,
            },
            transaction_sink,
        )
    }

    pub async fn process_transactions(&mut self) -> Result<()> {
        while let Some(transaction) = self.transactions.recv().await {
            let account = self
                .accounts
                .entry(transaction.client)
                .or_insert_with(|| Account::new(transaction.client));
            account.apply_transaction(transaction)?;
        }

        Ok(())
    }

    pub fn print_accounts(&self) -> Result<()> {
        let mut writer = csv::Writer::from_writer(std::io::stdout());
        self.accounts
            .values()
            .try_for_each(|transaction| writer.serialize(transaction))
            .map_err(Error::from)
    }
}
