use crate::error::EngineError;
use crate::transaction::Transaction;
use anyhow::{Error, Result};
use csv::{Reader, ReaderBuilder, Trim};
use futures::StreamExt;
use std::{env, fs::File};
use tokio::sync::mpsc::Sender;

pub async fn start_processing_input_data(transaction_sink: Sender<Transaction>) -> Result<()> {
    let mut reader = initialize_reader()?;

    futures::stream::iter(reader.deserialize::<Transaction>().map(|result| async {
        let transaction = result.map_err(Error::from)?;
        transaction_sink
            .send(transaction)
            .await
            .map_err(Error::from)
    }))
    .buffered(16)
    .collect::<Vec<Result<()>>>()
    .await
    .into_iter()
    .collect::<Result<_>>()?;

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
