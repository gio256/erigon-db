use crate::k256::ecdsa::SigningKey;
use ethers::{prelude::*, solc::Solc, utils::format_ether};
use eyre::eyre;

// Temporary script used for seeding test data
#[tokio::main]
async fn main() {
    let dst: Address = "0xa94f5374Fce5edBC8E2a8697C15331677e6EbF0B"
        .parse()
        .unwrap();

    let endpoint = "http://localhost:8545";
    let provider = Provider::<Http>::try_from(endpoint)
        .map_err(|e| eyre!("Could not establish provider: {}", e))
        .unwrap();
    let client = std::sync::Arc::new(provider);
    let chainid = client.get_chainid().await.unwrap().as_u32() as u16;
    // address: 0x67b1d87101671b127f5f8714789C7192f7ad340e
    let src: Wallet<SigningKey> =
        "26e86e45f6fc45ec6e2ecd128cec80fa1d1505e5507dcd2ae58c3130a7a97b48"
            .parse()
            .unwrap();
    dbg!(src.address());
    let src = src.with_chain_id(chainid);
    let signer = SignerMiddleware::new(client.clone(), src);

    let bal = client.get_balance(signer.address(), None).await.unwrap();
    dbg!(format_ether(bal));

    // // Send a transfer
    let tx = TransactionRequest::new().to(dst).value(100_usize);
    let receipt = signer
        .send_transaction(tx, None)
        .await
        .unwrap()
        .await
        .unwrap();
    dbg!(receipt);

    // Deploy a contract
    let compiled = Solc::default()
        .compile_source("./contracts/Store.sol")
        .unwrap();
    let contract = compiled
        .get("./contracts/Store.sol", "Store")
        .expect("no contract");
    let tx = TransactionRequest::new().data(contract.bin.unwrap().clone().into_bytes().unwrap());
    let receipt = signer
        .send_transaction(tx, None)
        .await
        .unwrap()
        .await
        .unwrap()
        .unwrap();
    //first deployed contract: 0x0d4c6c6605a729a379216c93e919711a081beba2
    println!("Store address: {:?}", receipt.contract_address.unwrap());

    //first deployed contract
    let contract: Address = "0x0d4c6c6605a729a379216c93e919711a081beba2"
        .parse()
        .unwrap();

    //dec slot
    let tx = TransactionRequest::new().to(contract);
    signer
        .send_transaction(tx, None)
        .await
        .unwrap()
        .await
        .unwrap()
        .unwrap();
}
