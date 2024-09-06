use reth::primitives::Bytes;
use revm_primitives::{
    Precompile, PrecompileError, PrecompileErrors, PrecompileOutput, PrecompileResult,
};

pub const KYVE_PC_BASE: u64 = 10_000;
pub const KYVE_API_URL: &str = "https://data.services.kyve.network";

pub const KYVE_READ_PC: Precompile = Precompile::Standard(kyve_read);

fn kyve_read(input: &Bytes, gas_limit: u64) -> PrecompileResult {
    let data_size = input.len();
    let gas_used: u64 = (KYVE_PC_BASE as usize + data_size * 3) as u64;

    if input.is_empty() {
        return Err(PrecompileErrors::Error(PrecompileError::Other(
            "A block number and field must be provided".to_string(),
        )));
    }

    if gas_used > gas_limit {
        return Err(PrecompileErrors::Error(PrecompileError::OutOfGas));
    }

    let input_str = unsafe { String::from_utf8(input.0.to_vec()) }.unwrap();
    let (block_number, field) = {
        let mut parts = input_str.split(";");
        let block_number = parts.next();
        let field = parts.next();

        (block_number, field)
    };

    if block_number.is_none() {
        return Err(PrecompileErrors::Error(PrecompileError::Other(
            "A block number must be provided".to_string(),
        )));
    } else if field.is_none() {
        return Err(PrecompileErrors::Error(PrecompileError::Other(
            "Field must be provided".to_string(),
        )));
    }

    let blk_number = block_number.unwrap();

    if !(blk_number.to_string().parse::<usize>().unwrap() >= 19426589) {
        return Err(PrecompileErrors::Error(PrecompileError::Other(
            "Can only read from block 19426589".to_string(),
        )));
    }

    let field = field.unwrap();

    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        println!(
            "{}",
            format!("{}/ethereum/beacon/blob_sidecars?block_height={}", KYVE_API_URL, blk_number)
        );
        let req = reqwest::get(
            format!("{}/ethereum/beacon/blob_sidecars?block_height={}", KYVE_API_URL, blk_number)
                .as_str(),
        )
        .await;

        match req {
            Ok(resp) => match resp.json::<serde_json::Value>().await {
                Ok(json_val) => {
                    let main_val = json_val.get("value").and_then(|v| v.get("value")).unwrap();

                    let slot = main_val.get("slot").and_then(|s| s.as_u64()).unwrap().to_string();

                    let blobs = main_val.get("blobs").and_then(|b| b.as_array()).unwrap();

                    let (blob_indx, field) = field.split_once('.').unwrap();

                    if field.eq("slot") {
                        return Ok(PrecompileOutput::new(gas_used, slot.into_bytes().into()));
                    }

                    match blobs.get(blob_indx.parse::<usize>().unwrap()) {
                        Some(get_field) => {
                            if let Some(field_val) = get_field.get(field) {
                                Ok(PrecompileOutput::new(
                                    gas_used,
                                    field_val.as_str().unwrap().to_string().into_bytes().into(),
                                ))
                            } else {
                                Err(PrecompileErrors::Error(PrecompileError::Other(
                                    "Field does not exist".to_string(),
                                )))
                            }
                        }
                        None => Err(PrecompileErrors::Error(PrecompileError::Other(
                            "Blob index does not exist".to_string(),
                        ))),
                    }
                }
                Err(_) => Err(PrecompileErrors::Error(PrecompileError::Other(
                    "Invalid Response from server".to_string(),
                ))),
            },
            Err(e) => {
                println!("{:?}", e);
                println!("{}", e.url().unwrap());
                println!("{}", e.status().unwrap());
                println!("{}", e.to_string());
                Err(PrecompileErrors::Error(PrecompileError::Other(
                    "Could not connect with KYVE".to_string(),
                )))
            }
        }
    })
}

#[cfg(test)]
mod kyve_tests {
    use crate::inner::kyve_precompile::kyve_read;
    use reth::primitives::Bytes;

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
