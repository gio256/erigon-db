use ethers::{abi::Abi, prelude::*, signers::LocalWallet, utils::format_ether};
use eyre::{eyre, Result};
use std::{fs, path::Path, sync::Arc, time::Duration};

/// Temporary script used for seeding test data

#[cfg(feature = "txgen")]
mod bindings;
use bindings::{factory::*, store::*};

const ENDPOINT: &str = "http://localhost:8545";
const BUILD_DIR: &str = env!("SOLC_BUILD_DIR");

macro_rules! factory {
    ($contract:literal, $client:stmt) => {
        paste::paste! {
            make_factory(
                $contract,
                crate::bindings:: [<$contract>] :: [<$contract:camel:upper _ABI>] .clone(),
                $client)
        }
    };
}

#[tokio::main]
async fn main() -> Result<()> {
    let provider = Provider::<Http>::try_from(ENDPOINT)
        .map_err(|e| eyre!("Could not establish provider: {}", e))?
        .interval(Duration::from_millis(1));
    let client = std::sync::Arc::new(provider);
    let chainid = client.get_chainid().await?.as_u32() as u16;

    // address: 0x67b1d87101671b127f5f8714789C7192f7ad340e
    let src = "26e86e45f6fc45ec6e2ecd128cec80fa1d1505e5507dcd2ae58c3130a7a97b48"
        .parse::<LocalWallet>()?
        .with_chain_id(chainid);
    let signer = Arc::new(SignerMiddleware::new(client.clone(), src));
    let dst: Address = "0xa94f5374Fce5edBC8E2a8697C15331677e6EbF0B".parse()?;

    let bal = client.get_balance(signer.address(), None).await?;
    dbg!(format_ether(bal));

    // Send a transfer
    let tx = TransactionRequest::new().to(dst).value(100_usize);
    signer.send_transaction(tx, None).await?.await?;

    let fac_fac = factory!("factory", signer.clone())?;
    let deployed = fac_fac.deploy(())?.send().await?;
    //first deployed contract: 0x0d4c6c6605a729a379216c93e919711a081beba2
    println!("Factory address: {:?}", deployed.address());
    let fac = Factory::new(deployed.address(), signer.clone());
    fac.deploy(Default::default()).send().await?.await?;

    let store = Store::new(fac.last().call().await?, signer.clone());
    store.kill().send().await?.await?;

    store
        .set(U256::from(1), U256::from(234))
        .send()
        .await?
        .await?;
    Ok(())
}

pub fn make_factory<M: Middleware>(
    name: &str,
    abi: Abi,
    client: Arc<M>,
) -> Result<ContractFactory<M>> {
    let build_dir = Path::new(BUILD_DIR);
    let bin = fs::read_to_string(&build_dir.join(format!("{}.bin", name)))?;
    Ok(ContractFactory::new(
        abi,
        Bytes::from(hex::decode(bin)?),
        client,
    ))
}
