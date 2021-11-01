use crate::{error::EngineError, transaction::Transaction};
use std::collections::{HashMap, HashSet};

#[derive(serde::Serialize, PartialEq, Debug)]
pub struct Account {
    pub client: u16,
    #[serde(serialize_with = "round_serialize")]
    pub available: f32,
    #[serde(serialize_with = "round_serialize")]
    pub held: f32,
    #[serde(serialize_with = "round_serialize")]
    pub total: f32,
    pub locked: bool,
    #[serde(skip_serializing)]
    transaction_history: HashMap<u32, f32>,
    #[serde(skip_serializing)]
    transactions_in_dispute: HashSet<u32>,
}

impl Account {
    pub fn new(client: u16) -> Self {
        Account {
            client,
            available: 0.0,
            held: 0.0,
            total: 0.0,
            locked: false,
            transaction_history: HashMap::with_capacity(1),
            transactions_in_dispute: HashSet::new(),
        }
    }

    pub fn apply_transaction(
        &mut self,
        Transaction {
            r#type, tx, amount, ..
        }: Transaction,
    ) -> Result<(), EngineError> {
        if self.locked {
            return Ok(());
        }

        match r#type.as_ref() {
            "withdrawal" => amount
                .map(|amount| {
                    self.withdrawal(amount);
                    self.transaction_history.insert(tx, amount);
                })
                .ok_or(EngineError::NoAmountInWitdrawal),
            "deposit" => amount
                .map(|amount| {
                    self.deposit(amount);
                    self.transaction_history.insert(tx, amount);
                })
                .ok_or(EngineError::NoAmountInDeposit),
            "dispute" => {
                self.dispute(tx);
                Ok(())
            }
            "resolve" => {
                self.resolve(tx);
                Ok(())
            }
            "chargeback" => {
                self.chargeback(tx);
                Ok(())
            }
            unknown => Err(EngineError::InvalidRawTransactionType(unknown.into())),
        }
    }

    fn deposit(&mut self, amount: f32) {
        self.available += amount;
        self.update_total()
    }

    fn withdrawal(&mut self, amount: f32) {
        if self.available - amount >= 0.0 {
            self.available -= amount;
            self.update_total();
        }
    }

    fn dispute(&mut self, transaction_id: u32) {
        if let Some(amount) = self.lookup_transaction_history(transaction_id) {
            if self.transactions_in_dispute.get(&transaction_id).is_none() {
                self.apply_dispute(amount, transaction_id)
            }
        }
    }

    fn apply_dispute(&mut self, amount: f32, transaction_id: u32) {
        self.available -= amount;
        self.held += amount;
        self.transactions_in_dispute.insert(transaction_id);
    }

    fn resolve(&mut self, transaction_id: u32) {
        if self.transactions_in_dispute.get(&transaction_id).is_some() {
            if let Some(amount) = self.lookup_transaction_history(transaction_id) {
                self.apply_resolve(amount);
                self.transactions_in_dispute.remove(&transaction_id);
            }
        }
    }

    fn apply_resolve(&mut self, amount: f32) {
        self.available += amount;
        self.held -= amount;
    }

    fn chargeback(&mut self, transaction_id: u32) {
        if self.transactions_in_dispute.get(&transaction_id).is_some() {
            if let Some(amount) = self.lookup_transaction_history(transaction_id) {
                self.apply_chargeback(amount);
                self.transactions_in_dispute.remove(&transaction_id);
            }
        }
    }

    fn apply_chargeback(&mut self, amount: f32) {
        self.held -= amount;
        self.update_total();
        self.locked = true;
    }

    fn lookup_transaction_history(&self, transaction_id: u32) -> Option<f32> {
        self.transaction_history.get(&transaction_id).copied()
    }

    fn update_total(&mut self) {
        self.total = self.available + self.held;
    }
}

// Precision n -> precision_factor = 10^n
const PRECISION_FACTOR: f32 = 10000.0; // n = 4

fn round_to_precision_4(value: f32) -> f32 {
    (value * PRECISION_FACTOR).round() / PRECISION_FACTOR
}

fn round_serialize<S>(x: &f32, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_f32(round_to_precision_4(*x))
}

#[cfg(test)]
mod tests {
    use super::Account;
    use crate::{account::round_to_precision_4, transaction::Transaction};

    #[test]
    fn invalid_transaction() {
        let mut account = Account::new(0);

        let invalid_transaction = make_transaction("invalid", 0, 0, Some(1.0));
        assert!(account.apply_transaction(invalid_transaction).is_err());
    }

    #[test]
    fn basic_deposit_and_withdrawal() {
        let mut account = Account::new(0);

        let first_deposit = make_transaction("deposit", 0, 0, Some(1.0));
        account.apply_transaction(first_deposit).unwrap();
        assert_eq!(account.available, 1.0);
        assert_eq!(account.held, 0.0);
        assert_eq!(account.total, 1.0);
        assert_eq!(account.transaction_history.len(), 1);

        let second_deposit = make_transaction("deposit", 0, 1, Some(0.5555));
        account.apply_transaction(second_deposit).unwrap();
        assert_eq!(account.available, 1.5555);
        assert_eq!(account.held, 0.0);
        assert_eq!(account.total, 1.5555);
        assert_eq!(account.transaction_history.len(), 2);
        assert!(!account.locked);

        let first_withdrawal = make_transaction("withdrawal", 0, 2, Some(1.0));
        account.apply_transaction(first_withdrawal).unwrap();
        assert_eq!(round_to_precision_4(account.available), 0.5555);
        assert_eq!(account.held, 0.0);
        assert_eq!(round_to_precision_4(account.total), 0.5555);
        assert_eq!(account.transaction_history.len(), 3);
        assert!(!account.locked);

        let second_withdrawal = make_transaction("withdrawal", 0, 3, Some(2.0));
        account.apply_transaction(second_withdrawal).unwrap();
        assert_eq!(round_to_precision_4(account.available), 0.5555);
        assert_eq!(account.held, 0.0);
        assert_eq!(round_to_precision_4(account.total), 0.5555);
        assert_eq!(account.transaction_history.len(), 4);
        assert!(!account.locked);
    }

    #[test]
    fn invalid_deposit_without_amount() {
        let mut account = Account::new(0);

        let invalid_deposit = make_transaction("deposit", 0, 0, None);
        assert!(account.apply_transaction(invalid_deposit).is_err());
    }

    #[test]
    fn invalid_withdrawal_without_amount() {
        let mut account = Account::new(0);

        let invalid_withdrawal = make_transaction("withdrawal", 0, 0, None);
        assert!(account.apply_transaction(invalid_withdrawal).is_err());
    }

    #[test]
    fn valid_disput() {
        let mut account = Account::new(0);

        let deposit = make_transaction("deposit", 0, 0, Some(1.0));
        account.apply_transaction(deposit).unwrap();

        let dispute = make_transaction("dispute", 0, 0, None);
        account.apply_transaction(dispute).unwrap();

        let double_dispute = make_transaction("dispute", 0, 0, None);
        account.apply_transaction(double_dispute).unwrap();

        assert_eq!(account.available, 0.0);
        assert_eq!(account.held, 1.0);
        assert_eq!(account.total, 1.0);
        assert_eq!(account.transaction_history.len(), 1);
        assert_eq!(account.transactions_in_dispute.len(), 1);
        assert!(!account.locked);
    }

    #[test]
    fn invalid_dispute() {
        let mut account = Account::new(0);

        let deposit = make_transaction("deposit", 0, 0, Some(1.0));
        account.apply_transaction(deposit).unwrap();

        let dispute = make_transaction("dispute", 0, 1, None);
        account.apply_transaction(dispute).unwrap();

        assert_eq!(account.available, 1.0);
        assert_eq!(account.held, 0.0);
        assert_eq!(account.total, 1.0);
        assert_eq!(account.transaction_history.len(), 1);
        assert_eq!(account.transactions_in_dispute.len(), 0);
        assert!(!account.locked);
    }

    #[test]
    fn valid_resolve() {
        let mut account = Account::new(0);

        let deposit = make_transaction("deposit", 0, 0, Some(1.0));
        account.apply_transaction(deposit).unwrap();

        let dispute = make_transaction("dispute", 0, 0, None);
        account.apply_transaction(dispute).unwrap();

        let resolve = make_transaction("resolve", 0, 0, None);
        account.apply_transaction(resolve).unwrap();

        let double_resolve = make_transaction("resolve", 0, 0, None);
        account.apply_transaction(double_resolve).unwrap();

        assert_eq!(account.available, 1.0);
        assert_eq!(account.held, 0.0);
        assert_eq!(account.total, 1.0);
        assert_eq!(account.transaction_history.len(), 1);
        assert_eq!(account.transactions_in_dispute.len(), 0);
        assert!(!account.locked);
    }

    #[test]
    fn invalid_resolve() {
        let mut account = Account::new(0);

        let deposit = make_transaction("deposit", 0, 0, Some(1.0));
        account.apply_transaction(deposit).unwrap();

        let first_resolve = make_transaction("resolve", 0, 0, None);
        account.apply_transaction(first_resolve).unwrap();

        let second_resolve = make_transaction("resolve", 0, 42, None);
        account.apply_transaction(second_resolve).unwrap();

        assert_eq!(account.available, 1.0);
        assert_eq!(account.held, 0.0);
        assert_eq!(account.total, 1.0);
        assert_eq!(account.transaction_history.len(), 1);
        assert_eq!(account.transactions_in_dispute.len(), 0);
    }

    #[test]
    fn valid_chargeback() {
        let mut account = Account::new(0);

        let deposit = make_transaction("deposit", 0, 0, Some(1.0));
        account.apply_transaction(deposit).unwrap();

        let dispute = make_transaction("dispute", 0, 0, None);
        account.apply_transaction(dispute).unwrap();

        let chargeback = make_transaction("chargeback", 0, 0, None);
        account.apply_transaction(chargeback).unwrap();

        let double_chargeback = make_transaction("chargeback", 0, 0, None);
        account.apply_transaction(double_chargeback).unwrap();

        assert_eq!(account.available, 0.0);
        assert_eq!(account.held, 0.0);
        assert_eq!(account.total, 0.0);
        assert_eq!(account.transaction_history.len(), 1);
        assert_eq!(account.transactions_in_dispute.len(), 0);
        assert!(account.locked);

        let deposit_after_lock = make_transaction("deposit", 0, 1, Some(1.0));
        // Should have no effect
        account.apply_transaction(deposit_after_lock).unwrap();

        assert_eq!(account.available, 0.0);
        assert_eq!(account.held, 0.0);
        assert_eq!(account.total, 0.0);
        assert_eq!(account.transaction_history.len(), 1);
        assert_eq!(account.transactions_in_dispute.len(), 0);
        assert!(account.locked);
    }

    #[test]
    fn invalid_chargeback() {
        let mut account = Account::new(0);

        let deposit = make_transaction("deposit", 0, 0, Some(1.0));
        account.apply_transaction(deposit).unwrap();

        let first_chargeback = make_transaction("chargeback", 0, 0, None);
        account.apply_transaction(first_chargeback).unwrap();

        let second_chargeback = make_transaction("chargeback", 0, 42, None);
        account.apply_transaction(second_chargeback).unwrap();

        assert_eq!(account.available, 1.0);
        assert_eq!(account.held, 0.0);
        assert_eq!(account.total, 1.0);
        assert_eq!(account.transaction_history.len(), 1);
        assert_eq!(account.transactions_in_dispute.len(), 0);
        assert!(!account.locked);
    }

    fn make_transaction<T: Into<String>>(
        r#type: T,
        client: u16,
        tx: u32,
        amount: Option<f32>,
    ) -> Transaction {
        Transaction {
            r#type: r#type.into(),
            client,
            tx,
            amount,
        }
    }
}
