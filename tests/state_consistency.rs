use fmt::Debug;
use ft_logic_io::Action;
use ft_main_io::{FTokenAction, FTokenEvent, InitFToken};
use gclient::{Error, EventListener, EventProcessor, GearApi, Result};
use gear_lib::non_fungible_token::token::TokenMetadata;
use gstd::prelude::*;
use nft_io::InitNFT;
use pretty_assertions::assert_eq;
use primitive_types::H256;
use subxt::{
    error::{DispatchError, ModuleError, ModuleErrorData},
    Error as SubxtError,
};
use supply_chain::*;

const ALICE: [u8; 32] = [
    212, 53, 147, 199, 21, 253, 211, 28, 97, 20, 26, 189, 4, 169, 159, 214, 130, 44, 133, 88, 133,
    76, 205, 227, 154, 86, 132, 231, 165, 109, 162, 125,
];

fn decode<T: Decode>(payload: Vec<u8>) -> Result<T> {
    Ok(T::decode(&mut payload.as_slice())?)
}

async fn upload_code(client: &GearApi, path: &str) -> Result<H256> {
    let code_id = match client.upload_code_by_path(path).await {
        Ok((code_id, _)) => code_id.into(),
        Err(Error::Subxt(SubxtError::Runtime(DispatchError::Module(ModuleError {
            error_data:
                ModuleErrorData {
                    pallet_index: 14,
                    error: [6, 0, 0, 0],
                },
            ..
        })))) => sp_core_hashing::blake2_256(&gclient::code_from_os(path)?),
        Err(other_error) => return Err(other_error),
    };

    println!("Uploaded `{path}`.");

    Ok(code_id.into())
}

async fn upload_program_and_wait_reply<T: Decode>(
    client: &GearApi,
    listener: &mut EventListener,
    path: &str,
    payload: impl Encode,
) -> Result<([u8; 32], T)> {
    let (message_id, program_id) = common_upload_program(client, path, payload).await?;
    let (_, raw_reply, _) = listener.reply_bytes_on(message_id.into()).await?;

    let reply = decode(raw_reply.expect("Received an error message instead of a reply"))?;

    println!("Initialized `{path}`.");

    Ok((program_id, reply))
}

async fn upload_program(
    client: &GearApi,
    listener: &mut EventListener,
    path: &str,
    payload: impl Encode,
) -> Result<[u8; 32]> {
    let (message_id, program_id) = common_upload_program(client, path, payload).await?;

    assert!(listener
        .message_processed(message_id.into())
        .await?
        .succeed());
    println!("Initialized `{path}`.");

    Ok(program_id)
}

async fn common_upload_program(
    client: &GearApi,
    path: &str,
    payload: impl Encode,
) -> Result<([u8; 32], [u8; 32])> {
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

    Ok((message_id.into(), program_id.into()))
}

async fn send_message_with_custom_limit<T: Decode>(
    client: &GearApi,
    listener: &mut EventListener,
    destination: [u8; 32],
    payload: impl Encode + Debug,
    modify_gas_limit: fn(u64) -> u64,
) -> Result<Result<T, String>> {
    let encoded_payload = payload.encode();
    let destination = destination.into();

    let gas_limit = client
        .calculate_handle_gas(None, destination, encoded_payload, 0, true, None)
        .await?
        .min_limit;
    let modified_gas_limit = modify_gas_limit(gas_limit);

    println!("Sending a payload: `{payload:?}`.");
    println!("Calculated gas limit: {gas_limit}.");
    println!("Modified gas limit: {modified_gas_limit}.");

    let (message_id, _) = client
        .send_message(destination, payload, modified_gas_limit, 0)
        .await?;

    println!("Sending completed.");

    let (_, raw_reply, _) = listener.reply_bytes_on(message_id).await?;

    Ok(match raw_reply {
        Ok(raw_reply) => Ok(decode(raw_reply)?),
        Err(error) => Err(error),
    })
}

async fn send_message<T: Decode>(
    client: &GearApi,
    listener: &mut EventListener,
    destination: [u8; 32],
    payload: impl Encode + Debug,
) -> Result<T> {
    Ok(
        send_message_with_custom_limit(client, listener, destination, payload, |gas| gas * 2)
            .await?
            .expect("Received an error message instead of a reply"),
    )
}

async fn send_message_for_sc(
    client: &GearApi,
    listener: &mut EventListener,
    destination: [u8; 32],
    payload: impl Encode + Debug,
) -> Result<Result<SupplyChainEvent, SupplyChainError>> {
    send_message(client, listener, destination, payload).await
}

async fn send_message_with_insufficient_gas(
    client: &GearApi,
    listener: &mut EventListener,
    destination: [u8; 32],
    payload: impl Encode + Debug,
) -> Result<String> {
    Ok(
        send_message_with_custom_limit::<()>(client, listener, destination, payload, |gas| {
            gas - gas / 100
        })
        .await?
        .expect_err("Received a reply instead of an error message"),
    )
}

#[tokio::test]
#[ignore]
async fn state_consistency() -> Result<()> {
    let client = GearApi::dev()
        .await
        .expect("The node must be running for a gclient test");
    let mut listener = client.subscribe().await?;

    let storage_code_hash = upload_code(&client, "target/ft_storage.wasm").await?;
    let ft_logic_code_hash = upload_code(&client, "target/ft_logic.wasm").await?;

    let ft_actor_id = upload_program(
        &client,
        &mut listener,
        "target/ft_main.wasm",
        InitFToken {
            storage_code_hash,
            ft_logic_code_hash,
        },
    )
    .await?;

    let nft_actor_id = upload_program(
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
    .await?;

    let (supply_chain_actor_id, reply) =
        upload_program_and_wait_reply::<Result<(), SupplyChainError>>(
            &client,
            &mut listener,
            "target/wasm32-unknown-unknown/debug/supply_chain.opt.wasm",
            SupplyChainInit {
                producers: vec![ALICE.into()],
                distributors: vec![ALICE.into()],
                retailers: vec![ALICE.into()],

                fungible_token: ft_actor_id.into(),
                non_fungible_token: nft_actor_id.into(),
            },
        )
        .await?;
    assert_eq!(reply, Ok(()));

    let item_id = 0.into();
    let price = 123456;
    let delivery_time = 600000;
    let approve = true;
    let mut payload =
        SupplyChainAction::new(InnerSupplyChainAction::Producer(ProducerAction::Produce {
            token_metadata: TokenMetadata::default(),
        }));

    assert!(
        FTokenEvent::Ok
            == send_message(
                &client,
                &mut listener,
                ft_actor_id,
                FTokenAction::Message {
                    transaction_id: 0,
                    payload: Action::Mint {
                        recipient: ALICE.into(),
                        amount: price
                    }
                    .encode(),
                },
            )
            .await?
    );
    assert!(
        FTokenEvent::Ok
            == send_message(
                &client,
                &mut listener,
                ft_actor_id,
                FTokenAction::Message {
                    transaction_id: 1,
                    payload: Action::Approve {
                        approved_account: supply_chain_actor_id.into(),
                        amount: price * 3,
                    }
                    .encode(),
                },
            )
            .await?
    );

    // SupplyChainAction::Producer(ProducerAction::Produce)

    println!(
        "{}",
        send_message_with_insufficient_gas(
            &client,
            &mut listener,
            supply_chain_actor_id,
            payload.clone(),
        )
        .await?
    );
    assert_eq!(
        send_message_for_sc(
            &client,
            &mut listener,
            supply_chain_actor_id,
            payload.to_retry()
        )
        .await?,
        Ok(SupplyChainEvent {
            item_id,
            item_state: ItemState {
                state: ItemEventState::Produced,
                by: Role::Producer
            }
        })
    );

    // SupplyChainAction::Producer(ProducerAction::PutUpForSale)

    payload = SupplyChainAction::new(InnerSupplyChainAction::Producer(
        ProducerAction::PutUpForSale { item_id, price },
    ));

    println!(
        "{}",
        send_message_with_insufficient_gas(
            &client,
            &mut listener,
            supply_chain_actor_id,
            payload.clone(),
        )
        .await?
    );
    assert_eq!(
        send_message_for_sc(
            &client,
            &mut listener,
            supply_chain_actor_id,
            payload.to_retry()
        )
        .await?,
        Ok(SupplyChainEvent {
            item_id,
            item_state: ItemState {
                state: ItemEventState::ForSale,
                by: Role::Producer
            }
        }),
    );

    // SupplyChainAction::Distributor(DistributorAction::Purchase)

    payload = SupplyChainAction::new(InnerSupplyChainAction::Distributor(
        DistributorAction::Purchase {
            item_id,
            delivery_time,
        },
    ));

    println!(
        "{}",
        send_message_with_insufficient_gas(
            &client,
            &mut listener,
            supply_chain_actor_id,
            payload.clone(),
        )
        .await?
    );
    assert_eq!(
        send_message_for_sc(
            &client,
            &mut listener,
            supply_chain_actor_id,
            payload.to_retry()
        )
        .await?,
        Ok(SupplyChainEvent {
            item_id,
            item_state: ItemState {
                state: ItemEventState::Purchased,
                by: Role::Distributor
            }
        }),
    );

    // SupplyChainAction::Producer(ProducerAction::Approve)

    assert_eq!(
        send_message_for_sc(
            &client,
            &mut listener,
            supply_chain_actor_id,
            SupplyChainAction::new(InnerSupplyChainAction::Producer(ProducerAction::Approve {
                item_id,
                approve
            }))
        )
        .await?,
        Ok(SupplyChainEvent {
            item_id,
            item_state: ItemState {
                state: ItemEventState::Approved,
                by: Role::Producer
            }
        })
    );

    // SupplyChainAction::Producer(ProducerAction::Ship)

    assert_eq!(
        send_message_for_sc(
            &client,
            &mut listener,
            supply_chain_actor_id,
            SupplyChainAction::new(InnerSupplyChainAction::Producer(ProducerAction::Ship(
                item_id
            )))
        )
        .await?,
        Ok(SupplyChainEvent {
            item_id,
            item_state: ItemState {
                state: ItemEventState::Shipped,
                by: Role::Producer
            }
        }),
    );

    // SupplyChainAction::Distributor(DistributorAction::Receive)

    payload = SupplyChainAction::new(InnerSupplyChainAction::Distributor(
        DistributorAction::Receive(item_id),
    ));

    println!(
        "{}",
        send_message_with_insufficient_gas(
            &client,
            &mut listener,
            supply_chain_actor_id,
            payload.clone(),
        )
        .await?
    );
    assert_eq!(
        send_message_for_sc(
            &client,
            &mut listener,
            supply_chain_actor_id,
            payload.to_retry()
        )
        .await?,
        Ok(SupplyChainEvent {
            item_id,
            item_state: ItemState {
                state: ItemEventState::Received,
                by: Role::Distributor
            }
        }),
    );

    // SupplyChainAction::Distributor(DistributorAction::Process)

    assert_eq!(
        send_message_for_sc(
            &client,
            &mut listener,
            supply_chain_actor_id,
            SupplyChainAction::new(InnerSupplyChainAction::Distributor(
                DistributorAction::Process(item_id)
            ))
        )
        .await?,
        Ok(SupplyChainEvent {
            item_id,
            item_state: ItemState {
                state: ItemEventState::Processed,
                by: Role::Distributor
            }
        }),
    );

    // SupplyChainAction::Distributor(DistributorAction::Package)

    assert_eq!(
        send_message_for_sc(
            &client,
            &mut listener,
            supply_chain_actor_id,
            SupplyChainAction::new(InnerSupplyChainAction::Distributor(
                DistributorAction::Package(item_id)
            ))
        )
        .await?,
        Ok(SupplyChainEvent {
            item_id,
            item_state: ItemState {
                state: ItemEventState::Packaged,
                by: Role::Distributor
            }
        }),
    );

    // SupplyChainAction::Distributor(DistributorAction::PutUpForSale)

    payload = SupplyChainAction::new(InnerSupplyChainAction::Distributor(
        DistributorAction::PutUpForSale { item_id, price },
    ));

    println!(
        "{}",
        send_message_with_insufficient_gas(
            &client,
            &mut listener,
            supply_chain_actor_id,
            payload.clone(),
        )
        .await?
    );
    assert_eq!(
        send_message_for_sc(
            &client,
            &mut listener,
            supply_chain_actor_id,
            payload.to_retry()
        )
        .await?,
        Ok(SupplyChainEvent {
            item_id,
            item_state: ItemState {
                state: ItemEventState::ForSale,
                by: Role::Distributor
            }
        }),
    );

    // SupplyChainAction::Retailer(RetailerAction::Purchase)

    payload = SupplyChainAction::new(InnerSupplyChainAction::Retailer(RetailerAction::Purchase {
        item_id,
        delivery_time,
    }));

    println!(
        "{}",
        send_message_with_insufficient_gas(
            &client,
            &mut listener,
            supply_chain_actor_id,
            payload.clone(),
        )
        .await?
    );
    assert_eq!(
        send_message_for_sc(
            &client,
            &mut listener,
            supply_chain_actor_id,
            payload.to_retry()
        )
        .await?,
        Ok(SupplyChainEvent {
            item_id,
            item_state: ItemState {
                state: ItemEventState::Purchased,
                by: Role::Retailer
            }
        })
    );

    // SupplyChainAction::Distributor(DistributorAction::Approve)

    assert_eq!(
        send_message_for_sc(
            &client,
            &mut listener,
            supply_chain_actor_id,
            SupplyChainAction::new(InnerSupplyChainAction::Distributor(
                DistributorAction::Approve { item_id, approve }
            ))
        )
        .await?,
        Ok(SupplyChainEvent {
            item_id,
            item_state: ItemState {
                state: ItemEventState::Approved,
                by: Role::Distributor
            }
        }),
    );

    // SupplyChainAction::Distributor(DistributorAction::Ship)

    assert_eq!(
        send_message_for_sc(
            &client,
            &mut listener,
            supply_chain_actor_id,
            SupplyChainAction::new(InnerSupplyChainAction::Distributor(
                DistributorAction::Ship(item_id)
            ))
        )
        .await?,
        Ok(SupplyChainEvent {
            item_id,
            item_state: ItemState {
                state: ItemEventState::Shipped,
                by: Role::Distributor
            }
        }),
    );

    // SupplyChainAction::Retailer(RetailerAction::Receive)

    payload = SupplyChainAction::new(InnerSupplyChainAction::Retailer(RetailerAction::Receive(
        item_id,
    )));

    println!(
        "{}",
        send_message_with_insufficient_gas(
            &client,
            &mut listener,
            supply_chain_actor_id,
            payload.clone(),
        )
        .await?
    );
    assert_eq!(
        send_message_for_sc(
            &client,
            &mut listener,
            supply_chain_actor_id,
            payload.to_retry()
        )
        .await?,
        Ok(SupplyChainEvent {
            item_id,
            item_state: ItemState {
                state: ItemEventState::Received,
                by: Role::Retailer
            }
        }),
    );

    // SupplyChainAction::Retailer(RetailerAction::PutUpForSale)

    payload = SupplyChainAction::new(InnerSupplyChainAction::Retailer(
        RetailerAction::PutUpForSale { item_id, price },
    ));

    println!(
        "{}",
        send_message_with_insufficient_gas(
            &client,
            &mut listener,
            supply_chain_actor_id,
            payload.clone(),
        )
        .await?
    );
    assert_eq!(
        send_message_for_sc(
            &client,
            &mut listener,
            supply_chain_actor_id,
            payload.to_retry()
        )
        .await?,
        Ok(SupplyChainEvent {
            item_id,
            item_state: ItemState {
                state: ItemEventState::ForSale,
                by: Role::Retailer
            }
        }),
    );

    // SupplyChainAction::Consumer(ConsumerAction::Purchase)

    payload = SupplyChainAction::new(InnerSupplyChainAction::Consumer(ConsumerAction::Purchase(
        item_id,
    )));

    println!(
        "{}",
        send_message_with_insufficient_gas(
            &client,
            &mut listener,
            supply_chain_actor_id,
            payload.clone(),
        )
        .await?
    );
    assert_eq!(
        send_message_for_sc(
            &client,
            &mut listener,
            supply_chain_actor_id,
            payload.to_retry()
        )
        .await?,
        Ok(SupplyChainEvent {
            item_id,
            item_state: ItemState {
                state: ItemEventState::Purchased,
                by: Role::Consumer
            }
        }),
    );

    Ok(())
}
