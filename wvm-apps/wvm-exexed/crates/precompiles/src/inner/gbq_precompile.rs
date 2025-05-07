// TODO: retest and clean in dev
use crate::inner::{
    arweave_read_precompile::ARWEAVE_PC_READ_BASE,
    string_block::from_sealed_block_senders_value,
    wvm_block_precompile::{process_block_to_field, process_pc_response_from_str_bytes},
};
use alloy_primitives::Bytes;
use load_db::LoadDbConnection;
use revm_primitives::{Precompile, PrecompileError, PrecompileErrors, PrecompileResult};
use serde_json::Value;
use wvm_static::{internal_block, PRECOMPILE_LOADDB_CLIENT};

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

    let id_str = String::from_utf8(input.0.to_vec());

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
            let wvm_bgc = (&*PRECOMPILE_LOADDB_CLIENT).clone();

            let res_from_bgc =
                internal_block(async { wvm_bgc.query_state(block_id.to_string()).await }).map_err(
                    |_| {
                        PrecompileError::Other(
                            "Tokio runtime could not block_on for operation".to_string(),
                        )
                    },
                )?;

            Some((field, res_from_bgc))
        }
        Err(_) => None,
    }
    .ok_or(PrecompileErrors::Error(PrecompileError::Other("Invalid Input".to_string())))?;

    let block_str = res.1.ok_or_else(|| {
        PrecompileErrors::Error(PrecompileError::Other("Unknown block".to_string()))
    })?;

    let block = serde_json::from_str::<Value>(&block_str).map_err(|_| {
        PrecompileErrors::Error(PrecompileError::Other(
            "Cannot deserialize block with senders".to_string(),
        ))
    })?;
    let block = from_sealed_block_senders_value(block);

    let process_field = process_block_to_field(res.0.to_string(), block);

    process_pc_response_from_str_bytes(gas_used, process_field)
}

#[cfg(test)]
mod tests {
    use crate::inner::{
        gbq_precompile::gbq_read, string_block::from_sealed_block_senders_value,
        wvm_block_precompile::process_block_to_field,
    };
    use alloy_primitives::Bytes;
    use reth::primitives::SealedBlockWithSenders;
    use serde_json::Value;

    #[test]
    pub fn test_gbq_pc() {
        unsafe {
            std::env::set_var(
                "CONFIG",
                std::env::current_dir()
                    .unwrap()
                    .join("./../../../bq-config.json")
                    .to_str()
                    .unwrap(),
            );
        }
        let input = Bytes::from("253;hash".as_bytes());
        let result = gbq_read(&input, 100_000);
        assert_eq!(
            result.unwrap().bytes.to_vec(),
            "0x79daa2736e6272f8ad4a3453cecc00db8f0cbcc03604cbd42cc13fec8b8214fb"
                .as_bytes()
                .to_vec()
        );
    }

    #[test]
    pub fn test_des() {
        let a = r#"{"block":{"header":{"hash":"0x79daa2736e6272f8ad4a3453cecc00db8f0cbcc03604cbd42cc13fec8b8214fb","header":{"parent_hash":"0xf5e00ab64482565b2f05d1010ebe1b662c7d86ebefa80fff7e89bf80f0f96e26","ommers_hash":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347","beneficiary":"0x123463a4b065722e99115d6c222f267d9cabb524","state_root":"0xeeec063dfa322eaf90ef24b34cd901bb471ef2082ed714df8f6db3a3cdcd19a2","transactions_root":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","receipts_root":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","withdrawals_root":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","logs_bloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","difficulty":"0x0","number":253,"gas_limit":30000000,"gas_used":0,"timestamp":1721649945,"mix_hash":"0x4b20893dcd8d56e89d9a2324109f826953d701310d57d9ccdd27298bdb024b1e","nonce":0,"base_fee_per_gas":7,"blob_gas_used":null,"excess_blob_gas":null,"parent_beacon_block_root":null,"requests_root":null,"extra_data":"0x726574682f76312e302e312f6c696e7578"}},"body":[],"ommers":[],"withdrawals":[],"requests":null},"senders":[]}"#;
        let deserialize = serde_json::from_str::<Value>(a);
        let v = deserialize.unwrap();
        let block = from_sealed_block_senders_value(v);

        let process_field = process_block_to_field("hash".to_string(), block).unwrap();
        assert_eq!(
            process_field,
            "0x79daa2736e6272f8ad4a3453cecc00db8f0cbcc03604cbd42cc13fec8b8214fb"
                .as_bytes()
                .to_vec()
        );
    }
}
