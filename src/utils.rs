use ft_logic_io::Action;
use ft_main_io::{FTokenAction, FTokenEvent};
use gear_lib::non_fungible_token::{
    io::NFTTransfer,
    token::{TokenId, TokenMetadata},
};
use gstd::{
    errors::ContractError,
    msg::{self, CodecMessageFuture},
    prelude::*,
    ActorId,
};
use nft_io::{NFTAction, NFTEvent};

fn send<T: Decode>(
    actor_id: ActorId,
    payload: impl Encode,
) -> Result<CodecMessageFuture<T>, ContractError> {
    msg::send_for_reply_as(actor_id, payload, 0)
}

fn nft_event_to_nft_transfer(nft_event: Result<NFTEvent, ContractError>) -> NFTTransfer {
    if let NFTEvent::Transfer(nft_transfer) =
        nft_event.expect("Failed to load or decode `NFTEvent::Transfer`")
    {
        nft_transfer
    } else {
        panic!("Received an unexpected `NFTEvent` variant");
    }
}

pub async fn mint_nft(
    transaction_id: u64,
    nft_actor_id: ActorId,
    token_metadata: TokenMetadata,
) -> TokenId {
    nft_event_to_nft_transfer(
        send(
            nft_actor_id,
            NFTAction::Mint {
                transaction_id,
                token_metadata,
            },
        )
        .expect("Failed to encode or send `NFTAction::Mint`")
        .await,
    )
    .token_id
}

pub async fn transfer_nft(
    transaction_id: u64,
    nft_actor_id: ActorId,
    to: ActorId,
    token_id: TokenId,
) {
    nft_event_to_nft_transfer(
        send(
            nft_actor_id,
            NFTAction::Transfer {
                transaction_id,
                to,
                token_id,
            },
        )
        .expect("Failed to encode or send `NFTAction::Transfer`")
        .await,
    );
}

pub async fn transfer_ftokens(
    transaction_id: u64,
    ft_actor_id: ActorId,
    sender: ActorId,
    recipient: ActorId,
    amount: u128,
) {
    let payload = FTokenAction::Message {
        transaction_id,
        payload: Action::Transfer {
            sender,
            recipient,
            amount,
        }
        .encode(),
    };

    if FTokenEvent::Ok
        != send(ft_actor_id, payload)
            .expect("Failed to encode or send `FTokenAction::Message`")
            .await
            .expect("Failed to load or decode `FTokenEvent`")
    {
        panic!("Received an unexpected `FTokenEvent` variant");
    }
}
