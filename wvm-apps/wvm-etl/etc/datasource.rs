use crate::{csv::CsvWriter, types::IndexerContractMapping};
use async_trait::async_trait;

use csv::ReaderBuilder;
use indexmap::IndexMap;
use phf::phf_ordered_map;
use polars::prelude::*;
use std::{any::Any, collections::HashMap, fs::File};


#[async_trait]
pub trait DatasourceWritable {
    async fn write_data(&self, table_name: &str, csv_writer: &CsvWriter);
    fn as_any(&self) -> &dyn Any;
}

pub static COMMON_COLUMNS: phf::OrderedMap<&'static str, &'static str> = phf_ordered_map! {
    "indexed_id" => "string",  // will need to generate uuid in rust; postgres allows for autogenerate
    "contract_address" => "string",
    "tx_hash" => "string",
    "block_number" => "int",
    "block_hash" => "string",
    "timestamp" => "int"
};


pub fn prepare_blockstate_table_config() -> HashMap<String, IndexMap<String, String>> {
    let mut table_column_definition: HashMap<String, IndexMap<String, String>> = HashMap::new();

    table_column_definition.insert(table_name, COMMON_COLUMNS);

    table_column_definition
}

///
/// Maps solidity types (in indexer config) to a placeholder type
/// The configuration / reth uses solidity types, we map these basic types
/// to an equivalent bigquery type further downstream
///
/// # Arguments
///
/// * `abi_type` - the ABI type, specified as a string
pub fn solidity_type_to_db_type(abi_type: &str) -> &str {
    match abi_type {
        "address" => "string",
        "bool" | "bytes" | "string" | "int256" | "uint256" => "string",
        "uint8" | "uint16" | "uint32" | "uint64" | "uint128" | "int8" | "int16" | "int32"
        | "int64" | "int128" => "int",
        _ => panic!("Unsupported type {}", abi_type),
    }
}