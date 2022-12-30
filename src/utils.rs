use crate::io::*;
use ft_logic_io::Action;
use ft_main_io::{FTokenAction, FTokenEvent};
use gear_lib::non_fungible_token::{
    io::NFTTransfer,
    token::{TokenId, TokenMetadata},
};
use gstd::{
    errors::Result as GstdResult,
    msg::{self, CodecMessageFuture},
    prelude::*,
    ActorId,
};
use nft_io::{NFTAction, NFTEvent};

fn send<T: Decode>(actor_id: ActorId, payload: impl Encode) -> GstdResult<CodecMessageFuture<T>> {
    msg::send_for_reply_as(actor_id, payload, 0)
}

fn nft_event_to_nft_transfer(
    nft_event: GstdResult<NFTEvent>,
) -> Result<NFTTransfer, SupplyChainError> {
    if let NFTEvent::Transfer(nft_transfer) = nft_event? {
        Ok(nft_transfer)
    } else {
        Err(SupplyChainError::NFTTransferFailed)
    }
}

pub async fn mint_nft(
    transaction_id: u64,
    non_fungible_token: ActorId,
    token_metadata: TokenMetadata,
) -> Result<TokenId, SupplyChainError> {
    let transfer = nft_event_to_nft_transfer(
        send(
            non_fungible_token,
            NFTAction::Mint {
                transaction_id,
                token_metadata,
            },
        )?
        .await,
    )
    .map_err(|error| {
        if error == SupplyChainError::NFTTransferFailed {
            SupplyChainError::NFTMintingFailed
        } else {
            error
        }
    })?;

    Ok(transfer.token_id)
}

pub async fn transfer_nft(
    transaction_id: u64,
    non_fungible_token: ActorId,
    to: ActorId,
    token_id: TokenId,
) -> Result<(), SupplyChainError> {
    nft_event_to_nft_transfer(
        send(
            non_fungible_token,
            NFTAction::Transfer {
                transaction_id,
                to,
                token_id,
            },
        )?
        .await,
    )?;

    Ok(())
}

pub async fn transfer_ftokens(
    transaction_id: u64,
    fungible_token: ActorId,
    sender: ActorId,
    recipient: ActorId,
    amount: u128,
) -> Result<(), SupplyChainError> {
    let payload = FTokenAction::Message {
        transaction_id,
        payload: Action::Transfer {
            sender,
            recipient,
            amount,
        }
        .encode(),
    };

    if FTokenEvent::Ok != send(fungible_token, payload)?.await? {
        Err(SupplyChainError::FTTransferFailed)
    } else {
        Ok(())
    }
}
