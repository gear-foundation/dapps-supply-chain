#![no_std]

use gear_lib::non_fungible_token::token::TokenMetadata;
use gstd::{async_main, exec, msg, prelude::*, util, ActorId};

mod io;
mod utils;

pub use io::*;

fn get_mut_item(
    items: &mut BTreeMap<ItemId, Item>,
    item_id: ItemId,
    expected_item_state: ItemState,
) -> &mut Item {
    let item = items
        .get_mut(&item_id)
        .unwrap_or_else(|| panic!("Item must exist in a supply chain"));

    assert_eq!(item.info.state, expected_item_state);

    item
}

fn role_to_set_item_dr(role: Role) -> fn(&mut Item, ActorId) {
    const FNS: [fn(&mut Item, ActorId); 2] = [Item::set_distributor, Item::set_retailer];

    FNS[role as usize - 1]
}

fn role_to_assert_pdr(role: Role) -> fn(&Item, ActorId) {
    const FNS: [fn(&Item, ActorId); 3] = [
        Item::assert_producer,
        Item::assert_distributor,
        Item::assert_retailer,
    ];

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

    fn assert_producer(&self, actor_id: ActorId) {
        assert_eq!(self.info.producer, actor_id)
    }

    fn assert_distributor(&self, actor_id: ActorId) {
        assert_eq!(self.info.distributor, actor_id)
    }

    fn assert_retailer(&self, actor_id: ActorId) {
        assert_eq!(self.info.retailer, actor_id)
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
    items: BTreeMap<ItemId, Item>,

    producers: BTreeSet<ActorId>,
    distributors: BTreeSet<ActorId>,
    retailers: BTreeSet<ActorId>,

    ft_actor_id: ActorId,
    nft_actor_id: ActorId,
}

impl SupplyChain {
    async fn produce(
        &mut self,
        msg_source: ActorId,
        transaction_id: u64,
        token_metadata: TokenMetadata,
    ) -> SupplyChainEvent {
        let item_id = utils::mint_nft(transaction_id, self.nft_actor_id, token_metadata).await;

        utils::transfer_nft(transaction_id, self.nft_actor_id, msg_source, item_id).await;

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

        SupplyChainEvent {
            item_id,
            item_state: Default::default(),
        }
    }

    async fn purchase(
        &mut self,
        msg_source: ActorId,
        transaction_id: u64,
        item_id: ItemId,
        expected_by: Role,
        by: Role,
        delivery_time: u64,
    ) -> SupplyChainEvent {
        let item = get_mut_item(
            &mut self.items,
            item_id,
            ItemState {
                state: ItemEventState::ForSale,
                by: expected_by,
            },
        );

        utils::transfer_ftokens(
            transaction_id,
            self.ft_actor_id,
            msg_source,
            exec::program_id(),
            item.info.price,
        )
        .await;

        role_to_set_item_dr(by)(item, msg_source);
        item.info.delivery_time = delivery_time;

        item.set_state_and_get_event(
            item_id,
            ItemState {
                state: ItemEventState::Purchased,
                by,
            },
        )
    }

    async fn put_up_for_sale(
        &mut self,
        msg_source: ActorId,
        transaction_id: u64,
        item_id: ItemId,
        expected_item_event_state: ItemEventState,
        by: Role,
        price: u128,
    ) -> SupplyChainEvent {
        let item = get_mut_item(
            &mut self.items,
            item_id,
            ItemState {
                state: expected_item_event_state,
                by,
            },
        );
        role_to_assert_pdr(by)(item, msg_source);

        utils::transfer_nft(
            transaction_id,
            self.nft_actor_id,
            exec::program_id(),
            item_id,
        )
        .await;
        item.info.price = price;

        item.set_state_and_get_event(
            item_id,
            ItemState {
                state: ItemEventState::ForSale,
                by,
            },
        )
    }

    async fn approve(
        &mut self,
        msg_source: ActorId,
        transaction_id: u64,
        item_id: ItemId,
        expected_by: Role,
        by: Role,
        approve: bool,
    ) -> SupplyChainEvent {
        let item = get_mut_item(
            &mut self.items,
            item_id,
            ItemState {
                state: ItemEventState::Purchased,
                by: expected_by,
            },
        );
        role_to_assert_pdr(by)(item, msg_source);

        let item_state = if approve {
            ItemState {
                state: ItemEventState::Approved,
                by,
            }
        } else {
            utils::transfer_ftokens(
                transaction_id,
                self.ft_actor_id,
                exec::program_id(),
                role_to_get_item_pdr(expected_by)(item),
                item.info.price,
            )
            .await;
            ItemState {
                state: ItemEventState::ForSale,
                by,
            }
        };

        item.set_state_and_get_event(item_id, item_state)
    }

    fn ship(&mut self, msg_source: ActorId, item_id: ItemId, by: Role) -> SupplyChainEvent {
        let item = get_mut_item(
            &mut self.items,
            item_id,
            ItemState {
                state: ItemEventState::Approved,
                by,
            },
        );
        role_to_assert_pdr(by)(item, msg_source);

        item.shipping_time = exec::block_timestamp();

        item.set_state_and_get_event(
            item_id,
            ItemState {
                state: ItemEventState::Shipped,
                by,
            },
        )
    }

    async fn receive(
        &mut self,
        msg_source: ActorId,
        transaction_id: u64,
        item_id: ItemId,
        expected_by: Role,
        by: Role,
    ) -> SupplyChainEvent {
        let item = get_mut_item(
            &mut self.items,
            item_id,
            ItemState {
                state: ItemEventState::Shipped,
                by: expected_by,
            },
        );
        role_to_assert_pdr(by)(item, msg_source);

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
                    self.ft_actor_id,
                    program_id,
                    msg_source,
                    item.info.price - amount,
                )
                .await;
            }
        }

        utils::transfer_ftokens(transaction_id, self.ft_actor_id, program_id, to, amount).await;
        utils::transfer_nft(transaction_id, self.nft_actor_id, msg_source, item_id).await;

        item.set_state_and_get_event(
            item_id,
            ItemState {
                state: ItemEventState::Received,
                by,
            },
        )
    }

    fn process_or_package(
        &mut self,
        msg_source: ActorId,
        item_id: ItemId,
        expected_item_event_state: ItemEventState,
        state: ItemEventState,
    ) -> SupplyChainEvent {
        let item = get_mut_item(
            &mut self.items,
            item_id,
            ItemState {
                state: expected_item_event_state,
                by: Role::Distributor,
            },
        );
        item.assert_distributor(msg_source);

        item.set_state_and_get_event(
            item_id,
            ItemState {
                state,
                by: Role::Distributor,
            },
        )
    }
}

struct TransactionManager<T> {
    transaction_id_nonce: u64,
    transactions: BTreeMap<T, u64>,
}

impl<T> Default for TransactionManager<T> {
    fn default() -> Self {
        Self {
            transaction_id_nonce: Default::default(),
            transactions: Default::default(),
        }
    }
}

impl<T: Ord + Clone> TransactionManager<T> {
    fn asquire_transaction<'a>(&'a mut self, key: &'a T) -> TransactionGuard<T> {
        let transaction_id = if let Some(id) = self.transactions.get(key) {
            *id
        } else {
            let id = self.transaction_id_nonce;

            self.transaction_id_nonce = self.transaction_id_nonce.wrapping_add(1);
            self.transactions.insert(key.clone(), id);

            id
        };

        TransactionGuard {
            transactions: &mut self.transactions,
            key,
            transaction_id,
        }
    }
}

struct TransactionGuard<'a, T: Ord> {
    transactions: &'a mut BTreeMap<T, u64>,
    key: &'a T,
    transaction_id: u64,
}

impl<T: Ord> Drop for TransactionGuard<'_, T> {
    fn drop(&mut self) {
        self.transactions.remove(self.key);
    }
}

#[derive(Default)]
struct State {
    state: SupplyChain,
    transaction_manager: TransactionManager<(ActorId, SupplyChainAction)>,
}

static mut STATE: Option<State> = None;

fn get_mut_state() -> &'static mut State {
    unsafe { STATE.get_or_insert(Default::default()) }
}

#[no_mangle]
extern "C" fn init() {
    let SupplyChainInit {
        producers,
        distributors,
        retailers,
        ft_actor_id,
        nft_actor_id,
    } = msg::load().expect("Failed to load or decode `SupplyChainInit`");

    if [&producers, &distributors, &retailers]
        .iter()
        .any(|actor_ids| actor_ids.contains(&ActorId::zero()))
    {
        panic!("Each `ActorId` of `producers`, `distributors`, and `retailers` mustn't equal `ActorId::zero()`");
    }

    let state = SupplyChain {
        producers,
        distributors,
        retailers,
        ft_actor_id,
        nft_actor_id,
        ..Default::default()
    };

    unsafe {
        STATE = Some(State {
            state,
            ..Default::default()
        });
    }
}

#[async_main]
async fn main() {
    let contract_action = msg::load().expect("Failed to load or decode `SupplyChainAction`");
    let msg_source = msg::source();

    let State {
        state: supply_chain,
        transaction_manager,
    } = get_mut_state();

    let event = match contract_action {
        SupplyChainAction::Consumer(action) => match action {
            ConsumerAction::Purchase(item_id) => {
                let key = (msg_source, contract_action);
                let transaction_guard = transaction_manager.asquire_transaction(&key);

                let item = get_mut_item(
                    &mut supply_chain.items,
                    item_id,
                    ItemState {
                        state: ItemEventState::ForSale,
                        by: Role::Retailer,
                    },
                );

                utils::transfer_ftokens(
                    transaction_guard.transaction_id,
                    supply_chain.ft_actor_id,
                    msg_source,
                    item.info.retailer,
                    item.info.price,
                )
                .await;
                utils::transfer_nft(
                    transaction_guard.transaction_id,
                    supply_chain.nft_actor_id,
                    msg_source,
                    item_id,
                )
                .await;

                item.set_state_and_get_event(
                    item_id,
                    ItemState {
                        state: ItemEventState::Purchased,
                        by: Role::Consumer,
                    },
                )
            }
        },
        SupplyChainAction::Producer(action) => {
            if !supply_chain.producers.contains(&msg_source) {
                panic!("`msg::source()` must be a producer");
            }

            match action {
                ProducerAction::Produce { token_metadata } => {
                    let key = (
                        msg_source,
                        SupplyChainAction::Producer(ProducerAction::Produce {
                            token_metadata: token_metadata.clone(),
                        }),
                    );
                    let transaction_guard = transaction_manager.asquire_transaction(&key);

                    supply_chain
                        .produce(msg_source, transaction_guard.transaction_id, token_metadata)
                        .await
                }
                ProducerAction::PutUpForSale { item_id, price } => {
                    let key = (msg_source, SupplyChainAction::Producer(action));
                    let transaction_guard = transaction_manager.asquire_transaction(&key);

                    supply_chain
                        .put_up_for_sale(
                            msg_source,
                            transaction_guard.transaction_id,
                            item_id,
                            ItemEventState::Produced,
                            Role::Producer,
                            price,
                        )
                        .await
                }
                ProducerAction::Approve { item_id, approve } => {
                    let key = (msg_source, SupplyChainAction::Producer(action));
                    let transaction_guard = transaction_manager.asquire_transaction(&key);

                    supply_chain
                        .approve(
                            msg_source,
                            transaction_guard.transaction_id,
                            item_id,
                            Role::Distributor,
                            Role::Producer,
                            approve,
                        )
                        .await
                }
                ProducerAction::Ship(item_id) => {
                    supply_chain.ship(msg_source, item_id, Role::Producer)
                }
            }
        }
        SupplyChainAction::Distributor(action) => {
            if !supply_chain.distributors.contains(&msg_source) {
                panic!("`msg::source()` must be a distributor");
            }

            let key = (msg_source, contract_action);

            match action {
                DistributorAction::Purchase {
                    item_id,
                    delivery_time,
                } => {
                    let transaction_guard = transaction_manager.asquire_transaction(&key);

                    supply_chain
                        .purchase(
                            msg_source,
                            transaction_guard.transaction_id,
                            item_id,
                            Role::Producer,
                            Role::Distributor,
                            delivery_time,
                        )
                        .await
                }
                DistributorAction::Receive(item_id) => {
                    let transaction_guard = transaction_manager.asquire_transaction(&key);

                    supply_chain
                        .receive(
                            msg_source,
                            transaction_guard.transaction_id,
                            item_id,
                            Role::Producer,
                            Role::Distributor,
                        )
                        .await
                }
                DistributorAction::Process(item_id) => supply_chain.process_or_package(
                    msg_source,
                    item_id,
                    ItemEventState::Received,
                    ItemEventState::Processed,
                ),
                DistributorAction::Package(item_id) => supply_chain.process_or_package(
                    msg_source,
                    item_id,
                    ItemEventState::Processed,
                    ItemEventState::Packaged,
                ),
                DistributorAction::PutUpForSale { item_id, price } => {
                    let transaction_guard = transaction_manager.asquire_transaction(&key);

                    supply_chain
                        .put_up_for_sale(
                            msg_source,
                            transaction_guard.transaction_id,
                            item_id,
                            ItemEventState::Packaged,
                            Role::Distributor,
                            price,
                        )
                        .await
                }
                DistributorAction::Approve { item_id, approve } => {
                    let transaction_guard = transaction_manager.asquire_transaction(&key);

                    supply_chain
                        .approve(
                            msg_source,
                            transaction_guard.transaction_id,
                            item_id,
                            Role::Retailer,
                            Role::Distributor,
                            approve,
                        )
                        .await
                }
                DistributorAction::Ship(item_id) => {
                    supply_chain.ship(msg_source, item_id, Role::Distributor)
                }
            }
        }
        SupplyChainAction::Retailer(action) => {
            if !supply_chain.retailers.contains(&msg_source) {
                panic!("`msg::source()` must be a retailer");
            }

            let key = (msg_source, contract_action);
            let transaction_guard = transaction_manager.asquire_transaction(&key);

            match action {
                RetailerAction::Purchase {
                    item_id,
                    delivery_time,
                } => {
                    supply_chain
                        .purchase(
                            msg_source,
                            transaction_guard.transaction_id,
                            item_id,
                            Role::Distributor,
                            Role::Retailer,
                            delivery_time,
                        )
                        .await
                }
                RetailerAction::Receive(item_id) => {
                    supply_chain
                        .receive(
                            msg_source,
                            transaction_guard.transaction_id,
                            item_id,
                            Role::Distributor,
                            Role::Retailer,
                        )
                        .await
                }
                RetailerAction::PutUpForSale { item_id, price } => {
                    supply_chain
                        .put_up_for_sale(
                            msg_source,
                            transaction_guard.transaction_id,
                            item_id,
                            ItemEventState::Received,
                            Role::Retailer,
                            price,
                        )
                        .await
                }
            }
        }
    };

    msg::reply(event, 0).expect("Failed to encode or reply with `SupplyChainEvent`");
}

#[no_mangle]
extern "C" fn meta_state() -> *mut [i32; 2] {
    let query = msg::load().expect("Failed to load or decode `SupplyChainStateQuery`");
    let supply_chain = &get_mut_state().state;

    let reply = match query {
        SupplyChainStateQuery::ItemInfo(item_id) => {
            SupplyChainStateReply::ItemInfo(supply_chain.items.get(&item_id).map(|item| item.info))
        }
        SupplyChainStateQuery::Participants => SupplyChainStateReply::Participants {
            producers: supply_chain.producers.clone(),
            distributors: supply_chain.distributors.clone(),
            retailers: supply_chain.retailers.clone(),
        },
        SupplyChainStateQuery::Roles(actor_id) => SupplyChainStateReply::Roles({
            let mut roles = BTreeSet::from([Role::Consumer]);

            if supply_chain.producers.contains(&actor_id) {
                roles.insert(Role::Producer);
            }
            if supply_chain.distributors.contains(&actor_id) {
                roles.insert(Role::Distributor);
            }
            if supply_chain.retailers.contains(&actor_id) {
                roles.insert(Role::Retailer);
            }

            roles
        }),
        SupplyChainStateQuery::ExistingItems => SupplyChainStateReply::ExistingItems(
            supply_chain
                .items
                .iter()
                .map(|item| (*item.0, item.1.info))
                .collect(),
        ),
        SupplyChainStateQuery::FtContractActorId => {
            SupplyChainStateReply::FtContractActorId(supply_chain.ft_actor_id)
        }
        SupplyChainStateQuery::NftContractActorId => {
            SupplyChainStateReply::NftContractActorId(supply_chain.nft_actor_id)
        }
    };

    util::to_leak_ptr(reply.encode())
}

gstd::metadata! {
    title: "Supply chain",
    init:
        input: SupplyChainInit,
    handle:
        input: SupplyChainAction,
        output: SupplyChainEvent,
    state:
        input: SupplyChainStateQuery,
        output: SupplyChainStateReply,
}
