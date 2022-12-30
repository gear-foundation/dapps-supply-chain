#![no_std]

use gear_lib::non_fungible_token::token::TokenMetadata;
use gstd::{errors::Result as GstdResult, exec, msg, prelude::*, util, ActorId, MessageId};
use hashbrown::{HashMap, HashSet};
use tx_manager::TransactionManager;

mod io;
mod tx_manager;
mod utils;

pub use io::*;

fn get_mut_item(
    items: &mut HashMap<ItemId, Item>,
    item_id: ItemId,
    expected_item_state: ItemState,
) -> Result<&mut Item, SupplyChainError> {
    let item = items
        .get_mut(&item_id)
        .ok_or(SupplyChainError::ItemNotFound)?;

    if item.info.state != expected_item_state {
        return Err(SupplyChainError::UnexpectedItemState);
    }

    Ok(item)
}

fn role_to_set_item_dr(role: Role) -> fn(&mut Item, ActorId) {
    const FNS: [fn(&mut Item, ActorId); 2] = [Item::set_distributor, Item::set_retailer];

    FNS[role as usize - 1]
}

type IsPdr = fn(&Item, ActorId) -> Result<(), SupplyChainError>;

fn role_to_is_pdr(role: Role) -> IsPdr {
    const FNS: [IsPdr; 3] = [Item::is_producer, Item::is_distributor, Item::is_retailer];

    FNS[role as usize]
}

fn role_to_get_item_pdr(role: Role) -> fn(&Item) -> ActorId {
    const FNS: [fn(&Item) -> ActorId; 3] = [
        Item::get_producer,
        Item::get_distributor,
        Item::get_retailer,
    ];

    FNS[role as usize]
}

#[derive(Default)]
struct Item {
    info: ItemInfo,
    shipping_time: u64,
}

impl Item {
    fn set_retailer(&mut self, retailer: ActorId) {
        self.info.retailer = retailer
    }

    fn set_distributor(&mut self, distributor: ActorId) {
        self.info.distributor = distributor
    }

    fn set_state_and_get_event(
        &mut self,
        item_id: ItemId,
        item_state: ItemState,
    ) -> SupplyChainEvent {
        self.info.state = item_state;

        SupplyChainEvent {
            item_id,
            item_state,
        }
    }

    fn is_pdr(pdr: ActorId, actor_id: ActorId) -> Result<(), SupplyChainError> {
        if pdr != actor_id {
            Err(SupplyChainError::AccessRestricted)
        } else {
            Ok(())
        }
    }

    fn is_producer(&self, actor_id: ActorId) -> Result<(), SupplyChainError> {
        Self::is_pdr(self.info.producer, actor_id)
    }

    fn is_distributor(&self, actor_id: ActorId) -> Result<(), SupplyChainError> {
        Self::is_pdr(self.info.distributor, actor_id)
    }

    fn is_retailer(&self, actor_id: ActorId) -> Result<(), SupplyChainError> {
        Self::is_pdr(self.info.retailer, actor_id)
    }

    fn get_producer(&self) -> ActorId {
        self.info.producer
    }

    fn get_retailer(&self) -> ActorId {
        self.info.retailer
    }

    fn get_distributor(&self) -> ActorId {
        self.info.distributor
    }
}

#[derive(Default)]
struct SupplyChain {
    items: HashMap<ItemId, Item>,

    producers: HashSet<ActorId>,
    distributors: HashSet<ActorId>,
    retailers: HashSet<ActorId>,

    fungible_token: ActorId,
    non_fungible_token: ActorId,
}

impl SupplyChain {
    async fn produce(
        &mut self,
        transaction_id: u64,
        msg_source: ActorId,
        token_metadata: TokenMetadata,
    ) -> Result<SupplyChainEvent, SupplyChainError> {
        let item_id =
            utils::mint_nft(transaction_id, self.non_fungible_token, token_metadata).await?;

        utils::transfer_nft(
            transaction_id + 1,
            self.non_fungible_token,
            msg_source,
            item_id,
        )
        .await?;

        self.items.insert(
            item_id,
            Item {
                info: ItemInfo {
                    producer: msg_source,
                    ..Default::default()
                },
                ..Default::default()
            },
        );

        Ok(SupplyChainEvent {
            item_id,
            item_state: Default::default(),
        })
    }

    async fn purchase(
        &mut self,
        transaction_id: u64,
        msg_source: ActorId,
        item_id: ItemId,
        expected_by: Role,
        by: Role,
        delivery_time: u64,
    ) -> Result<SupplyChainEvent, SupplyChainError> {
        let item = get_mut_item(
            &mut self.items,
            item_id,
            ItemState {
                state: ItemEventState::ForSale,
                by: expected_by,
            },
        )?;

        utils::transfer_ftokens(
            transaction_id,
            self.fungible_token,
            msg_source,
            exec::program_id(),
            item.info.price,
        )
        .await?;

        role_to_set_item_dr(by)(item, msg_source);
        item.info.delivery_time = delivery_time;

        Ok(item.set_state_and_get_event(
            item_id,
            ItemState {
                state: ItemEventState::Purchased,
                by,
            },
        ))
    }

    async fn put_up_for_sale(
        &mut self,
        transaction_id: u64,
        msg_source: ActorId,
        item_id: ItemId,
        expected_item_event_state: ItemEventState,
        by: Role,
        price: u128,
    ) -> Result<SupplyChainEvent, SupplyChainError> {
        let item = get_mut_item(
            &mut self.items,
            item_id,
            ItemState {
                state: expected_item_event_state,
                by,
            },
        )?;
        role_to_is_pdr(by)(item, msg_source)?;

        utils::transfer_nft(
            transaction_id,
            self.non_fungible_token,
            exec::program_id(),
            item_id,
        )
        .await?;
        item.info.price = price;

        Ok(item.set_state_and_get_event(
            item_id,
            ItemState {
                state: ItemEventState::ForSale,
                by,
            },
        ))
    }

    async fn approve(
        &mut self,
        transaction_id: u64,
        msg_source: ActorId,
        item_id: ItemId,
        expected_by: Role,
        by: Role,
        approve: bool,
    ) -> Result<SupplyChainEvent, SupplyChainError> {
        let item = get_mut_item(
            &mut self.items,
            item_id,
            ItemState {
                state: ItemEventState::Purchased,
                by: expected_by,
            },
        )?;
        role_to_is_pdr(by)(item, msg_source)?;

        let item_state = if approve {
            ItemState {
                state: ItemEventState::Approved,
                by,
            }
        } else {
            utils::transfer_ftokens(
                transaction_id,
                self.fungible_token,
                exec::program_id(),
                role_to_get_item_pdr(expected_by)(item),
                item.info.price,
            )
            .await?;
            ItemState {
                state: ItemEventState::ForSale,
                by,
            }
        };

        Ok(item.set_state_and_get_event(item_id, item_state))
    }

    fn ship(
        &mut self,
        msg_source: ActorId,
        item_id: ItemId,
        by: Role,
    ) -> Result<SupplyChainEvent, SupplyChainError> {
        let item = get_mut_item(
            &mut self.items,
            item_id,
            ItemState {
                state: ItemEventState::Approved,
                by,
            },
        )?;
        role_to_is_pdr(by)(item, msg_source)?;

        item.shipping_time = exec::block_timestamp();

        Ok(item.set_state_and_get_event(
            item_id,
            ItemState {
                state: ItemEventState::Shipped,
                by,
            },
        ))
    }

    async fn receive(
        &mut self,
        transaction_id: u64,
        msg_source: ActorId,
        item_id: ItemId,
        expected_by: Role,
        by: Role,
    ) -> Result<SupplyChainEvent, SupplyChainError> {
        let item = get_mut_item(
            &mut self.items,
            item_id,
            ItemState {
                state: ItemEventState::Shipped,
                by: expected_by,
            },
        )?;
        role_to_is_pdr(by)(item, msg_source)?;

        let program_id = exec::program_id();
        let elapsed_time = exec::block_timestamp() - item.shipping_time;
        // By default, all fungible tokens are transferred to a seller,
        let (mut to, mut amount) = (role_to_get_item_pdr(expected_by)(item), item.info.price);

        // but if the seller spends more time than was agreed...
        if elapsed_time > item.info.delivery_time {
            // ...and is extremely late (more than or exactly 2 times in this example),
            if elapsed_time >= item.info.delivery_time * 2 {
                // then all fungible tokens are refunded to a buyer...
                to = msg_source;
            } else {
                // ...or another half is transferred to the seller...
                amount /= 2;

                // ...and a half of tokens is refunded to the buyer.
                utils::transfer_ftokens(
                    transaction_id + 1,
                    self.fungible_token,
                    program_id,
                    msg_source,
                    item.info.price - amount,
                )
                .await?;
            }
        }

        utils::transfer_ftokens(transaction_id, self.fungible_token, program_id, to, amount)
            .await?;
        utils::transfer_nft(transaction_id, self.non_fungible_token, msg_source, item_id).await?;

        Ok(item.set_state_and_get_event(
            item_id,
            ItemState {
                state: ItemEventState::Received,
                by,
            },
        ))
    }

    fn process_or_package(
        &mut self,
        msg_source: ActorId,
        item_id: ItemId,
        expected_item_event_state: ItemEventState,
        state: ItemEventState,
    ) -> Result<SupplyChainEvent, SupplyChainError> {
        let item = get_mut_item(
            &mut self.items,
            item_id,
            ItemState {
                state: expected_item_event_state,
                by: Role::Distributor,
            },
        )?;
        item.is_distributor(msg_source)?;

        Ok(item.set_state_and_get_event(
            item_id,
            ItemState {
                state,
                by: Role::Distributor,
            },
        ))
    }
}

static mut STATE: Option<(SupplyChain, TransactionManager)> = None;

fn state() -> &'static mut (SupplyChain, TransactionManager) {
    unsafe { STATE.get_or_insert(Default::default()) }
}

fn reply(payload: impl Encode) -> GstdResult<MessageId> {
    msg::reply(payload, 0)
}

#[no_mangle]
extern "C" fn init() {
    let result = process_init();
    let is_err = result.is_err();

    reply(result).expect("Failed to encode or reply with `Result<(), SupplyChainError>`");

    if is_err {
        exec::exit(ActorId::zero());
    }
}

fn process_init() -> Result<(), SupplyChainError> {
    let SupplyChainInit {
        producers,
        distributors,
        retailers,
        fungible_token,
        non_fungible_token,
    } = msg::load()?;

    if producers
        .iter()
        .chain(&distributors)
        .chain(&retailers)
        .chain(&[fungible_token, non_fungible_token])
        .any(|actor| actor.is_zero())
    {
        return Err(SupplyChainError::ZeroActorId);
    }

    let [producers, distributors, retailers] =
        [producers, distributors, retailers].map(|actors| actors.into_iter().collect());

    unsafe {
        STATE = Some((
            SupplyChain {
                producers,
                distributors,
                retailers,
                fungible_token,
                non_fungible_token,
                ..Default::default()
            },
            Default::default(),
        ));
    }

    Ok(())
}

#[gstd::async_main]
async fn main() {
    reply(process_handle().await)
        .expect("Failed to encode or reply with `Result<SupplyChainEvent, SupplyChainError>`");
}

async fn process_handle() -> Result<SupplyChainEvent, SupplyChainError> {
    let SupplyChainAction {
        action,
        kind: action_kind,
    } = msg::load()?;

    let msg_source = msg::source();
    let (contract, tx_manager) = state();

    match action {
        InnerSupplyChainAction::Consumer(action) => match action {
            ConsumerAction::Purchase(item_id) => {
                let tx_guard = tx_manager.asquire_transaction(action_kind, msg_source)?;

                let item = get_mut_item(
                    &mut contract.items,
                    item_id,
                    ItemState {
                        state: ItemEventState::ForSale,
                        by: Role::Retailer,
                    },
                )?;

                utils::transfer_ftokens(
                    tx_guard.tx_id,
                    contract.fungible_token,
                    msg_source,
                    item.info.retailer,
                    item.info.price,
                )
                .await?;
                utils::transfer_nft(
                    tx_guard.tx_id,
                    contract.non_fungible_token,
                    msg_source,
                    item_id,
                )
                .await?;

                Ok(item.set_state_and_get_event(
                    item_id,
                    ItemState {
                        state: ItemEventState::Purchased,
                        by: Role::Consumer,
                    },
                ))
            }
        },
        InnerSupplyChainAction::Producer(action) => {
            if !contract.producers.contains(&msg_source) {
                return Err(SupplyChainError::AccessRestricted);
            }

            match action {
                ProducerAction::Produce { token_metadata } => {
                    let tx_guard = tx_manager.asquire_transactions(action_kind, msg_source, 2)?;

                    contract
                        .produce(tx_guard.tx_id, msg_source, token_metadata)
                        .await
                }
                ProducerAction::PutUpForSale { item_id, price } => {
                    let tx_guard = tx_manager.asquire_transaction(action_kind, msg_source)?;

                    contract
                        .put_up_for_sale(
                            tx_guard.tx_id,
                            msg_source,
                            item_id,
                            ItemEventState::Produced,
                            Role::Producer,
                            price,
                        )
                        .await
                }
                ProducerAction::Approve { item_id, approve } => {
                    let tx_guard = tx_manager.asquire_transaction(action_kind, msg_source)?;

                    contract
                        .approve(
                            tx_guard.tx_id,
                            msg_source,
                            item_id,
                            Role::Distributor,
                            Role::Producer,
                            approve,
                        )
                        .await
                }
                ProducerAction::Ship(item_id) => contract.ship(msg_source, item_id, Role::Producer),
            }
        }
        InnerSupplyChainAction::Distributor(action) => {
            if !contract.distributors.contains(&msg_source) {
                return Err(SupplyChainError::AccessRestricted);
            }

            match action {
                DistributorAction::Purchase {
                    item_id,
                    delivery_time,
                } => {
                    let tx_guard = tx_manager.asquire_transaction(action_kind, msg_source)?;

                    contract
                        .purchase(
                            tx_guard.tx_id,
                            msg_source,
                            item_id,
                            Role::Producer,
                            Role::Distributor,
                            delivery_time,
                        )
                        .await
                }
                DistributorAction::Receive(item_id) => {
                    let tx_guard = tx_manager.asquire_transactions(action_kind, msg_source, 2)?;

                    contract
                        .receive(
                            tx_guard.tx_id,
                            msg_source,
                            item_id,
                            Role::Producer,
                            Role::Distributor,
                        )
                        .await
                }
                DistributorAction::Process(item_id) => contract.process_or_package(
                    msg_source,
                    item_id,
                    ItemEventState::Received,
                    ItemEventState::Processed,
                ),
                DistributorAction::Package(item_id) => contract.process_or_package(
                    msg_source,
                    item_id,
                    ItemEventState::Processed,
                    ItemEventState::Packaged,
                ),
                DistributorAction::PutUpForSale { item_id, price } => {
                    let tx_guard = tx_manager.asquire_transaction(action_kind, msg_source)?;

                    contract
                        .put_up_for_sale(
                            tx_guard.tx_id,
                            msg_source,
                            item_id,
                            ItemEventState::Packaged,
                            Role::Distributor,
                            price,
                        )
                        .await
                }
                DistributorAction::Approve { item_id, approve } => {
                    let tx_guard = tx_manager.asquire_transaction(action_kind, msg_source)?;

                    contract
                        .approve(
                            tx_guard.tx_id,
                            msg_source,
                            item_id,
                            Role::Retailer,
                            Role::Distributor,
                            approve,
                        )
                        .await
                }
                DistributorAction::Ship(item_id) => {
                    contract.ship(msg_source, item_id, Role::Distributor)
                }
            }
        }
        InnerSupplyChainAction::Retailer(action) => {
            if !contract.retailers.contains(&msg_source) {
                return Err(SupplyChainError::AccessRestricted);
            }

            match action {
                RetailerAction::Purchase {
                    item_id,
                    delivery_time,
                } => {
                    let tx_guard = tx_manager.asquire_transaction(action_kind, msg_source)?;

                    contract
                        .purchase(
                            tx_guard.tx_id,
                            msg_source,
                            item_id,
                            Role::Distributor,
                            Role::Retailer,
                            delivery_time,
                        )
                        .await
                }
                RetailerAction::Receive(item_id) => {
                    let tx_guard = tx_manager.asquire_transactions(action_kind, msg_source, 2)?;

                    contract
                        .receive(
                            tx_guard.tx_id,
                            msg_source,
                            item_id,
                            Role::Distributor,
                            Role::Retailer,
                        )
                        .await
                }
                RetailerAction::PutUpForSale { item_id, price } => {
                    let tx_guard = tx_manager.asquire_transaction(action_kind, msg_source)?;

                    contract
                        .put_up_for_sale(
                            tx_guard.tx_id,
                            msg_source,
                            item_id,
                            ItemEventState::Received,
                            Role::Retailer,
                            price,
                        )
                        .await
                }
            }
        }
    }
}

#[no_mangle]
extern "C" fn meta_state() -> *mut [i32; 2] {
    let query = msg::load().expect("Failed to load or decode `SupplyChainStateQuery`");
    let contract = &state().0;

    let reply = match query {
        SupplyChainStateQuery::ItemInfo(item_id) => {
            SupplyChainStateReply::ItemInfo(contract.items.get(&item_id).map(|item| item.info))
        }
        SupplyChainStateQuery::Participants => {
            let [producers, distributors, retailers] = [
                &contract.producers,
                &contract.distributors,
                &contract.retailers,
            ]
            .map(|actors| actors.iter().cloned().collect());

            SupplyChainStateReply::Participants {
                producers,
                distributors,
                retailers,
            }
        }
        SupplyChainStateQuery::Roles(actor_id) => SupplyChainStateReply::Roles({
            let mut roles = vec![Role::Consumer];

            if contract.producers.contains(&actor_id) {
                roles.push(Role::Producer);
            }
            if contract.distributors.contains(&actor_id) {
                roles.push(Role::Distributor);
            }
            if contract.retailers.contains(&actor_id) {
                roles.push(Role::Retailer);
            }

            roles
        }),
        SupplyChainStateQuery::ExistingItems => SupplyChainStateReply::ExistingItems(
            contract
                .items
                .iter()
                .map(|item| (*item.0, item.1.info))
                .collect(),
        ),
        SupplyChainStateQuery::FungibleToken => {
            SupplyChainStateReply::FungibleToken(contract.fungible_token)
        }
        SupplyChainStateQuery::NonFungibleToken => {
            SupplyChainStateReply::NonFungibleToken(contract.non_fungible_token)
        }
    };

    util::to_leak_ptr(reply.encode())
}

gstd::metadata! {
    title: "Supply chain",
    init:
        input: SupplyChainInit,
        output: Result<(), SupplyChainError>,
    handle:
        input: SupplyChainAction,
        output: Result<SupplyChainEvent, SupplyChainError>,
    state:
        input: SupplyChainStateQuery,
        output: SupplyChainStateReply,
}
