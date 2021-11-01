use crate::error::EngineError;
use crate::transaction::Transaction;
use anyhow::Result;
use csv::{Reader, ReaderBuilder, Trim};
use std::{env, fs::File};
use tokio::sync::mpsc::Sender;

pub async fn start_processing_input_data(transaction_sink: Sender<Transaction>) -> Result<()> {
    let mut reader = initialize_reader()?;

    let mut transaction_stream = reader.deserialize::<Transaction>();
    for result in transaction_stream.by_ref() {
        let transaction = result?;
        transaction_sink.send(transaction).await?;
    }

    Ok(())
}

fn initialize_reader() -> Result<Reader<File>> {
    let path = env::args().nth(1).ok_or(EngineError::NoInputArgument)?;
    let file = File::open(path)?;

    let reader = ReaderBuilder::new()
        .trim(Trim::All)
        .flexible(true)
        .from_reader(file);
    Ok(reader)
}
