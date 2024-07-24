use bundlr_sdk::{
    currency::solana::{Solana, SolanaBuilder},
    tags::Tag,
    Bundlr, BundlrBuilder,
};
use dotenv::dotenv;
use eyre::eyre;
use reqwest::Url;
use std::env;

#[derive(Clone, Debug)]
pub struct IrysProvider {}

pub fn get_irys_pk() -> Result<String, env::VarError> {
    dotenv().ok();
    let key = "irys_pk";
    env::var(key)
}

async fn init_bundlr() -> eyre::Result<Bundlr<Solana>> {
    let irys_wallet_pk: String = get_irys_pk().unwrap();
    let url = Url::parse("https://node1.bundlr.network").unwrap();

    let currency = SolanaBuilder::new().wallet(&irys_wallet_pk).build().map_err(|e| {
        eyre::eyre!(
            "failed to initialize irys provider, failed to create bundlr wallet instance: {}",
            e
        )
    })?;

    let bundlr = BundlrBuilder::new()
        .url(url)
        .currency(currency)
        .fetch_pub_info()
        .await
        .map_err(|e| eyre::eyre!("failed to fetch bundlr public info: {}", e))?
        .build()
        .map_err(|e| eyre::eyre!("bundlr successfully initialized: {}", e))?;

    Ok(bundlr)
}

impl IrysProvider {
    pub fn new() -> IrysProvider {
        IrysProvider {}
    }

    pub async fn upload_data_to_irys(
        &self,
        data: Vec<u8>,
        param_tags: Vec<Tag>,
    ) -> eyre::Result<String> {
        let mut tags = vec![Tag::new("Protocol", "WeaveVM-Testnet-V0")];

        tags.extend(param_tags);

        let bundlr =
            init_bundlr().await.map_err(|e| eyre!("failed to initialize bundlr: {}", e))?;

        let mut tx = bundlr
            .create_transaction(data, tags)
            .map_err(|e| eyre!("failed to create transaction: {}", e))?;

        bundlr
            .sign_transaction(&mut tx)
            .await
            .map_err(|e| eyre!("failed to sign transaction: {}", e))?;

        let result = bundlr
            .send_transaction(tx)
            .await
            .map_err(|e| eyre!("failed to send transaction: {}", e))?;

        let id = result["id"]
            .as_str()
            .ok_or_else(|| eyre!("missing 'id' field in response"))?
            .to_string();

        eyre::Ok(id)
    }
}

#[derive(Clone, Debug)]
pub struct IrysRequest {
    tags: Vec<Tag>,
    data: Vec<u8>,
}

impl IrysRequest {
    pub fn new() -> Self {
        IrysRequest { tags: vec![], data: vec![] }
    }

    pub fn set_tag(&mut self, name: &str, value: &str) -> &mut IrysRequest {
        self.tags.push(Tag::new(name, value));
        self
    }

    pub fn set_data(&mut self, data: Vec<u8>) -> &mut IrysRequest {
        self.data = data;
        self
    }

    pub async fn send(&self) -> eyre::Result<String> {
        let provider = IrysProvider::new();
        self.send_with_provider(&provider).await
    }

    pub async fn send_with_provider(&self, provider: &IrysProvider) -> eyre::Result<String> {
        provider.upload_data_to_irys(self.data.clone(), self.tags.clone()).await
    }
}
