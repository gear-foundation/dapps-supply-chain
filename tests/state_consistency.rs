use ft_main_io::InitFToken;
use gclient::{Error, EventListener, EventProcessor, GearApi, Result};
use gear_lib::non_fungible_token::token::TokenMetadata;
use gstd::{prelude::*, ActorId};
use nft_io::InitNFT;
use primitive_types::H256;
use subxt::{
    error::{DispatchError, ModuleError, ModuleErrorData},
    ext::sp_runtime::AccountId32,
    Error as SubxtError,
};
use supply_chain::*;

const ALICE: [u8; 32] = [
    212, 53, 147, 199, 21, 253, 211, 28, 97, 20, 26, 189, 4, 169, 159, 214, 130, 44, 133, 88, 133,
    76, 205, 227, 154, 86, 132, 231, 165, 109, 162, 125,
];

async fn upload_program(
    client: &GearApi,
    listener: &mut EventListener,
    path: &str,
    payload: impl Encode,
) -> Result<[u8; 32]> {
    let encoded_payload = payload.encode();
    let gas_limit = client
        .calculate_upload_gas(
            None,
            gclient::code_from_os(path)?,
            encoded_payload,
            0,
            true,
            None,
        )
        .await?
        .min_limit;
    let (message_id, program_id, _) = client
        .upload_program_by_path(path, gclient::bytes_now(), payload, gas_limit, 0)
        .await?;

    assert!(listener.message_processed(message_id).await?.succeed());

    println!("Initialized `{path}`.");

    Ok(program_id.into())
}

async fn upload_code(client: &GearApi, path: &str) -> Result<H256> {
    let code_id = match client.upload_code_by_path(path).await {
        Ok((code_id, _)) => code_id.into(),
        Err(Error::Subxt(SubxtError::Runtime(DispatchError::Module(ModuleError {
            error_data:
                ModuleErrorData {
                    error: [6, 0, 0, 0],
                    ..
                },
            ..
        })))) => sp_core_hashing::blake2_256(&gclient::code_from_os(path)?),
        Err(other_error) => return Err(other_error),
    };

    println!("Uploaded `{path}`.");

    Ok(code_id.into())
}

#[tokio::test]
async fn state_consistency() -> Result<()> {
    let client = GearApi::gear().await?;
    let mut listener = client.subscribe().await?;

    let storage_code_hash = upload_code(&client, "target/ft_storage.wasm").await?;
    let ft_logic_code_hash = upload_code(&client, "target/ft_logic.wasm").await?;

    let ft_actor_id: ActorId = upload_program(
        &client,
        &mut listener,
        "target/ft_main.wasm",
        InitFToken {
            storage_code_hash,
            ft_logic_code_hash,
        },
    )
    .await?
    .into();

    let nft_actor_id: ActorId = upload_program(
        &client,
        &mut listener,
        "target/nft.opt.wasm",
        InitNFT {
            name: Default::default(),
            symbol: Default::default(),
            base_uri: Default::default(),
            royalties: Default::default(),
        },
    )
    .await?
    .into();

    let supply_chain_id = upload_program(
        &client,
        &mut listener,
        "target/wasm32-unknown-unknown/debug/supply_chain.opt.wasm",
        SupplyChainInit {
            producers: [ALICE.into()].into(),
            distributors: [ALICE.into()].into(),
            retailers: [ALICE.into()].into(),

            ft_actor_id,
            nft_actor_id,
        },
    )
    .await?;

    let payload = SupplyChainAction::Producer(ProducerAction::Produce {
        token_metadata: TokenMetadata::default(),
    });

    let gas_limit = client
        .calculate_handle_gas(
            None,
            supply_chain_id.into(),
            payload.encode(),
            0,
            true,
            None,
        )
        .await?
        .min_limit;

    let (message_id, _) = client
        .send_message(supply_chain_id.into(), payload, gas_limit, 0)
        .await?;

    let (d, h, o) = listener.reply_bytes_on(message_id).await?;

    println!("{h:?}");

    let reply = SupplyChainEvent::decode(
        &mut client
            .get_from_mailbox(AccountId32::from(ALICE), d)
            .await?
            .expect("Reply to the message was not found")
            .0
            .payload(),
    )
    .expect("Failed to decode the reply");

    println!("{reply:?}");

    Ok(())
}
