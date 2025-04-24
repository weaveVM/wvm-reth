use crate::inner::REQ_TIMEOUT;
use alloy_primitives::Bytes;
use reth::revm::precompile::{PrecompileError, PrecompileFn, PrecompileOutput, PrecompileResult};

pub const KYVE_PC_BASE: u64 = 10_000;
pub const KYVE_API_URL: &str = "https://data.services.kyve.network";

pub const KYVE_READ_PC: PrecompileFn = kyve_read as PrecompileFn;

fn kyve_read(input: &Bytes, gas_limit: u64) -> PrecompileResult {
    let data_size = input.len();
    let gas_used: u64 = (KYVE_PC_BASE as usize + data_size * 3) as u64;

    if input.is_empty() {
        return Err(PrecompileError::Other("A block number and field must be provided".to_string()));
    }

    if gas_used > gas_limit {
        return Err(PrecompileError::OutOfGas);
    }

    let input_str = match String::from_utf8(input.0.to_vec()) {
        Ok(s) => s,
        Err(_) => {
            return Err(PrecompileError::Other("Invalid input".to_string()));
        }
    };

    let (block_number, field) = {
        let mut parts = input_str.split(";");
        let block_number = parts.next();
        let field = parts.next();

        (block_number, field)
    };

    if block_number.is_none() {
        return Err(PrecompileError::Other("A block number must be provided".to_string()));
    } else if field.is_none() {
        return Err(PrecompileError::Other("Field must be provided".to_string()));
    }

    let blk_number = block_number.unwrap();
    let usize_blk_number = blk_number
        .to_string()
        .parse::<usize>()
        .map_err(|_| PrecompileError::Other("Invalid Block Number".to_string()))?;

    if !(usize_blk_number >= 19426589) {
        return Err(PrecompileError::Other("Can only read from block 19426589".to_string()));
    }

    let field = field.unwrap();

    let req = ureq::get(
        format!("{}/ethereum/beacon/blob_sidecars?block_height={}", KYVE_API_URL, blk_number)
            .as_str(),
    )
    .timeout((&*REQ_TIMEOUT).clone())
    .call();

    match req {
        Ok(resp) => {
            let json_val = resp
                .into_json::<serde_json::Value>()
                .map_err(|_| PrecompileError::Other("Invalid Response from server".to_string()))?;

            let main_val = json_val
                .get("value")
                .ok_or_else(|| PrecompileError::Other("Missing 'value' field".to_string()))?;

            let slot = main_val
                .get("slot")
                .and_then(|s| s.as_u64())
                .ok_or_else(|| {
                    PrecompileError::Other("Missing or invalid 'slot' field".to_string())
                })?
                .to_string();

            let blobs = main_val.get("blobs").and_then(|b| b.as_array()).ok_or_else(|| {
                PrecompileError::Other("Missing or invalid 'blobs' field".to_string())
            })?;

            let (blob_indx, field) = field
                .split_once('.')
                .ok_or_else(|| PrecompileError::Other("Invalid field format".to_string()))?;

            if field == "slot" {
                return Ok(PrecompileOutput::new(gas_used, slot.into_bytes().into()));
            }

            let blob_index = blob_indx
                .parse::<usize>()
                .map_err(|_| PrecompileError::Other("Invalid blob index".to_string()))?;

            let get_field = blobs
                .get(blob_index)
                .ok_or_else(|| PrecompileError::Other("Blob index does not exist".to_string()))?;

            let field_val = get_field
                .get(field)
                .and_then(|val| val.as_str())
                .ok_or_else(|| PrecompileError::Other("Field does not exist".to_string()))?;

            Ok(PrecompileOutput::new(gas_used, field_val.to_string().into_bytes().into()))
        }
        Err(e) => {
            println!("{:?}", e);
            Err(PrecompileError::Other("Could not connect with KYVE".to_string()))
        }
    }
}

#[cfg(test)]
mod kyve_tests {
    use crate::inner::kyve_precompile::kyve_read;
    use alloy_primitives::Bytes;

    #[test]
    pub fn test_kyve_precompile() {
        let input = Bytes::from("20033062;0.kzg_commitment".as_bytes());
        let read = kyve_read(&input, 100_000).unwrap();
        let res = read.bytes.0.to_vec();
        assert_eq!(String::from_utf8(res).unwrap(), "0x81eb4254a890fd840a6bc60de54fb6fcd3b91242153386b9e83337f00f641a12bf6ebd876134e8703edce6725e29046c");
    }

    #[test]
    pub fn test_kyve_precompile_before_blk() {
        let input = Bytes::from("19426588;0.kzg_commitment".as_bytes());
        let read = kyve_read(&input, 100_000);
        assert!(read.is_err());
        assert_eq!("Can only read from block 19426589", read.err().unwrap().to_string());
    }

    #[test]
    pub fn test_kyve_precompile_slot() {
        let input = Bytes::from("20033062;0.slot".as_bytes());
        let read = kyve_read(&input, 100_000).unwrap();
        let res = read.bytes.0.to_vec();
        assert_eq!(String::from_utf8(res).unwrap(), "9238016");
    }
}
