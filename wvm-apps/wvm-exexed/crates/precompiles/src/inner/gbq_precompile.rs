use crate::inner::arweave_read_precompile::ARWEAVE_PC_READ_BASE;
use crate::inner::string_block::{from_sealed_block_senders, Block};
use crate::inner::wevm_block_precompile::{
    process_block_to_field, process_pc_response_from_str_bytes,
};
use alloy_primitives::Bytes;
use reth::primitives::SealedBlockWithSenders;
use revm_primitives::{Precompile, PrecompileError, PrecompileErrors, PrecompileResult};
use wevm_static::WVM_BIGQUERY;

pub const GBQ_READ_PC: Precompile = Precompile::Standard(gbq_read);

fn gbq_read(input: &Bytes, gas_limit: u64) -> PrecompileResult {
    let data_size = input.len();
    let gas_used: u64 = (ARWEAVE_PC_READ_BASE as usize + data_size * 3) as u64;

    if gas_used > gas_limit {
        return Err(PrecompileErrors::Error(PrecompileError::OutOfGas));
    }

    if input.is_empty() {
        return Err(PrecompileErrors::Error(PrecompileError::Other(
            "Arweave Transaction Id cannot be empty".to_string(),
        )));
    }

    let id_str = unsafe { String::from_utf8(input.0.to_vec()) };

    let res = match id_str {
        Ok(val) => {
            let (block_id, field) = {
                let mut split = val.split(';');
                (
                    split.next().map(|e| e.to_string()).unwrap_or(String::default()),
                    split.next().map(|e| e.to_string()).unwrap_or(String::default()),
                )
            };

            // It needs to be obtained OUTSIDE the thread
            let wvm_bgc = (&*WVM_BIGQUERY).clone();

            let res_from_bgc = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async { wvm_bgc.bq_query_state(block_id.to_string()).await });

            Some((field, res_from_bgc))
        }
        Err(_) => None,
    }
    .ok_or(PrecompileErrors::Error(PrecompileError::Other("Invalid Input".to_string())))?;

    let block_str = res.1.ok_or_else(|| {
        PrecompileErrors::Error(PrecompileError::Other("Unknown block".to_string()))
    })?;

    let block = serde_json::from_str::<SealedBlockWithSenders>(&block_str).map_err(|_| {
        PrecompileErrors::Error(PrecompileError::Other(
            "Cannot deserialize block with senders".to_string(),
        ))
    })?;
    let block = from_sealed_block_senders(block);

    let process_field = process_block_to_field(res.0.to_string(), block);

    process_pc_response_from_str_bytes(gas_used, process_field)
}

#[cfg(test)]
mod tests {
    use crate::inner::gbq_precompile::gbq_read;
    use alloy_primitives::Bytes;

    #[test]
    pub fn test_gbq_pc() {
        std::env::set_var(
            "CONFIG",
            std::env::current_dir().unwrap().join("./../../../bq-config.json").to_str().unwrap(),
        );
        let input = Bytes::from("253;hash".as_bytes());
        let result = gbq_read(&input, 100_000);
    }
}
