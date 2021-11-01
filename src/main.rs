mod account;
mod collector;
mod error;
mod payment_engine;
mod transaction;

use anyhow::Result;
use payment_engine::PaymentsEngine;

#[tokio::main]
async fn main() -> Result<()> {
    let (mut payments_engine, sender) = PaymentsEngine::new();

    let collector_thread = tokio::spawn(collector::start_processing_input_data(sender));

    payments_engine.process_transactions().await?;
    collector_thread.await??;

    payments_engine.print_accounts()
}
