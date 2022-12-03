use common::{InitResult, MetaStateReply, Program, RunResult, TransactionProgram};
use gstd::{prelude::*, ActorId};
use gtest::{Program as InnerProgram, System};
use supply_chain::*;

mod common;
mod nft;
mod sft;

pub mod prelude;

pub use common::initialize_system;
pub use nft::NonFungibleToken;
pub use sft::Sft;

pub const FOREIGN_USER: u64 = 1029384756123;
pub const PRODUCER: u64 = 5;
pub const DISTRIBUTOR: u64 = 7;
pub const RETAILER: u64 = 9;

type SupplyChainRunResult<T> = RunResult<T, SupplyChainEvent>;
type SupplyChainInitResult<'a> = InitResult<SupplyChain<'a>>;

pub struct SupplyChain<'a>(InnerProgram<'a>);

impl Program for SupplyChain<'_> {
    fn inner_program(&self) -> &InnerProgram {
        &self.0
    }
}

impl<'a> SupplyChain<'a> {
    pub fn initialize(system: &'a System, ft_actor_id: ActorId, nft_actor_id: ActorId) -> Self {
        Self::initialize_custom(
            system,
            SupplyChainInit {
                producers: [PRODUCER.into()].into(),
                distributors: [DISTRIBUTOR.into()].into(),
                retailers: [RETAILER.into()].into(),

                ft_actor_id,
                nft_actor_id,
            },
        )
        .succeed()
    }

    pub fn initialize_custom(system: &'a System, config: SupplyChainInit) -> SupplyChainInitResult {
        let program = InnerProgram::current(system);

        let is_failed = program.send(FOREIGN_USER, config).main_failed();

        InitResult(Self(program), is_failed)
    }

    pub fn meta_state(&self) -> SupplyChainMetaState {
        SupplyChainMetaState(&self.0)
    }

    pub fn produce(&self, from: u64) -> SupplyChainRunResult<u128> {
        RunResult(
            self.0.send(
                from,
                SupplyChainAction::Producer(ProducerAction::Produce {
                    token_metadata: Default::default(),
                }),
            ),
            |item_id| SupplyChainEvent {
                item_id: item_id.into(),
                item_state: ItemState {
                    state: ItemEventState::Produced,
                    by: Role::Producer,
                },
            },
        )
    }

    pub fn put_up_for_sale_by_producer(
        &self,
        from: u64,
        item_id: u128,
        price: u128,
    ) -> SupplyChainRunResult<u128> {
        RunResult(
            self.0.send(
                from,
                SupplyChainAction::Producer(ProducerAction::PutUpForSale {
                    item_id: item_id.into(),
                    price,
                }),
            ),
            |item_id| SupplyChainEvent {
                item_id: item_id.into(),
                item_state: ItemState {
                    state: ItemEventState::ForSale,
                    by: Role::Producer,
                },
            },
        )
    }

    pub fn purchase_by_distributor(
        &self,
        from: u64,
        item_id: u128,
        delivery_time: u64,
    ) -> SupplyChainRunResult<u128> {
        RunResult(
            self.0.send(
                from,
                SupplyChainAction::Distributor(DistributorAction::Purchase {
                    item_id: item_id.into(),
                    delivery_time,
                }),
            ),
            |item_id| SupplyChainEvent {
                item_id: item_id.into(),
                item_state: ItemState {
                    state: ItemEventState::Purchased,
                    by: Role::Distributor,
                },
            },
        )
    }

    pub fn approve_by_producer(
        &self,
        from: u64,
        item_id: u128,
        approve: bool,
    ) -> SupplyChainRunResult<(u128, bool)> {
        RunResult(
            self.0.send(
                from,
                SupplyChainAction::Producer(ProducerAction::Approve {
                    item_id: item_id.into(),
                    approve,
                }),
            ),
            |(item_id, approved)| SupplyChainEvent {
                item_id: item_id.into(),
                item_state: ItemState {
                    state: bool_to_event(approved),
                    by: Role::Producer,
                },
            },
        )
    }

    pub fn ship_by_producer(&self, from: u64, item_id: u128) -> SupplyChainRunResult<u128> {
        RunResult(
            self.0.send(
                from,
                SupplyChainAction::Producer(ProducerAction::Ship(item_id.into())),
            ),
            |item_id| SupplyChainEvent {
                item_id: item_id.into(),
                item_state: ItemState {
                    state: ItemEventState::Shipped,
                    by: Role::Producer,
                },
            },
        )
    }

    pub fn receive_by_distributor(&self, from: u64, item_id: u128) -> SupplyChainRunResult<u128> {
        RunResult(
            self.0.send(
                from,
                SupplyChainAction::Distributor(DistributorAction::Receive(item_id.into())),
            ),
            |item_id| SupplyChainEvent {
                item_id: item_id.into(),
                item_state: ItemState {
                    state: ItemEventState::Received,
                    by: Role::Distributor,
                },
            },
        )
    }

    pub fn process(&self, from: u64, item_id: u128) -> SupplyChainRunResult<u128> {
        RunResult(
            self.0.send(
                from,
                SupplyChainAction::Distributor(DistributorAction::Process(item_id.into())),
            ),
            |item_id| SupplyChainEvent {
                item_id: item_id.into(),
                item_state: ItemState {
                    state: ItemEventState::Processed,
                    by: Role::Distributor,
                },
            },
        )
    }

    pub fn package(&self, from: u64, item_id: u128) -> SupplyChainRunResult<u128> {
        RunResult(
            self.0.send(
                from,
                SupplyChainAction::Distributor(DistributorAction::Package(item_id.into())),
            ),
            |item_id| SupplyChainEvent {
                item_id: item_id.into(),
                item_state: ItemState {
                    state: ItemEventState::Packaged,
                    by: Role::Distributor,
                },
            },
        )
    }

    pub fn put_up_for_sale_by_distributor(
        &self,
        from: u64,
        item_id: u128,
        price: u128,
    ) -> SupplyChainRunResult<u128> {
        RunResult(
            self.0.send(
                from,
                SupplyChainAction::Distributor(DistributorAction::PutUpForSale {
                    item_id: item_id.into(),
                    price,
                }),
            ),
            |item_id| SupplyChainEvent {
                item_id: item_id.into(),
                item_state: ItemState {
                    state: ItemEventState::ForSale,
                    by: Role::Distributor,
                },
            },
        )
    }

    pub fn purchase_by_retailer(
        &self,
        from: u64,
        item_id: u128,
        delivery_time: u64,
    ) -> SupplyChainRunResult<u128> {
        RunResult(
            self.0.send(
                from,
                SupplyChainAction::Retailer(RetailerAction::Purchase {
                    item_id: item_id.into(),
                    delivery_time,
                }),
            ),
            |item_id| SupplyChainEvent {
                item_id: item_id.into(),
                item_state: ItemState {
                    state: ItemEventState::Purchased,
                    by: Role::Retailer,
                },
            },
        )
    }

    pub fn approve_by_distributor(
        &self,
        from: u64,
        item_id: u128,
        approve: bool,
    ) -> SupplyChainRunResult<(u128, bool)> {
        RunResult(
            self.0.send(
                from,
                SupplyChainAction::Distributor(DistributorAction::Approve {
                    item_id: item_id.into(),
                    approve,
                }),
            ),
            |(item_id, approved)| SupplyChainEvent {
                item_id: item_id.into(),
                item_state: ItemState {
                    state: bool_to_event(approved),
                    by: Role::Distributor,
                },
            },
        )
    }

    pub fn ship_by_distributor(&self, from: u64, item_id: u128) -> SupplyChainRunResult<u128> {
        RunResult(
            self.0.send(
                from,
                SupplyChainAction::Distributor(DistributorAction::Ship(item_id.into())),
            ),
            |item_id| SupplyChainEvent {
                item_id: item_id.into(),
                item_state: ItemState {
                    state: ItemEventState::Shipped,
                    by: Role::Distributor,
                },
            },
        )
    }

    pub fn receive_by_retailer(&self, from: u64, item_id: u128) -> SupplyChainRunResult<u128> {
        RunResult(
            self.0.send(
                from,
                SupplyChainAction::Retailer(RetailerAction::Receive(item_id.into())),
            ),
            |item_id| SupplyChainEvent {
                item_id: item_id.into(),
                item_state: ItemState {
                    state: ItemEventState::Received,
                    by: Role::Retailer,
                },
            },
        )
    }

    pub fn put_up_for_sale_by_retailer(
        &self,
        from: u64,
        item_id: u128,
        price: u128,
    ) -> SupplyChainRunResult<u128> {
        RunResult(
            self.0.send(
                from,
                SupplyChainAction::Retailer(RetailerAction::PutUpForSale {
                    item_id: item_id.into(),
                    price,
                }),
            ),
            |item_id| SupplyChainEvent {
                item_id: item_id.into(),
                item_state: ItemState {
                    state: ItemEventState::ForSale,
                    by: Role::Retailer,
                },
            },
        )
    }

    pub fn purchase_by_consumer(&self, from: u64, item_id: u128) -> SupplyChainRunResult<u128> {
        RunResult(
            self.0.send(
                from,
                SupplyChainAction::Consumer(ConsumerAction::Purchase(item_id.into())),
            ),
            |item_id| SupplyChainEvent {
                item_id: item_id.into(),
                item_state: ItemState {
                    state: ItemEventState::Purchased,
                    by: Role::Consumer,
                },
            },
        )
    }
}

pub struct SupplyChainMetaState<'a>(&'a InnerProgram<'a>);

impl SupplyChainMetaState<'_> {
    pub fn item_price(self, item_id: u128) -> MetaStateReply<Option<u128>> {
        MetaStateReply(self.item_info(item_id).0.map(|item_info| item_info.price))
    }

    pub fn item_info(self, item_id: u128) -> MetaStateReply<Option<ItemInfo>> {
        if let SupplyChainStateReply::ItemInfo(reply) = self
            .0
            .meta_state(SupplyChainStateQuery::ItemInfo(item_id.into()))
            .unwrap()
        {
            MetaStateReply(reply)
        } else {
            unreachable!()
        }
    }

    pub fn participants(self) -> MetaStateReply<SupplyChainStateReply> {
        MetaStateReply(
            self.0
                .meta_state(SupplyChainStateQuery::Participants)
                .unwrap(),
        )
    }

    pub fn ft_program(self) -> MetaStateReply<ActorId> {
        if let SupplyChainStateReply::FtContractActorId(reply) = self
            .0
            .meta_state(SupplyChainStateQuery::FtContractActorId)
            .unwrap()
        {
            MetaStateReply(reply)
        } else {
            unreachable!()
        }
    }

    pub fn nft_program(self) -> MetaStateReply<ActorId> {
        if let SupplyChainStateReply::NftContractActorId(reply) = self
            .0
            .meta_state(SupplyChainStateQuery::NftContractActorId)
            .unwrap()
        {
            MetaStateReply(reply)
        } else {
            unreachable!()
        }
    }

    pub fn existing_items(self) -> MetaStateReply<BTreeMap<ItemId, ItemInfo>> {
        if let SupplyChainStateReply::ExistingItems(reply) = self
            .0
            .meta_state(SupplyChainStateQuery::ExistingItems)
            .unwrap()
        {
            MetaStateReply(reply)
        } else {
            unreachable!()
        }
    }

    pub fn roles(self, actor_id: u64) -> MetaStateReply<BTreeSet<Role>> {
        if let SupplyChainStateReply::Roles(reply) = self
            .0
            .meta_state(SupplyChainStateQuery::Roles(actor_id.into()))
            .unwrap()
        {
            MetaStateReply(reply)
        } else {
            unreachable!()
        }
    }
}

fn bool_to_event(is_approved: bool) -> ItemEventState {
    const EVENTS: [ItemEventState; 2] = [ItemEventState::ForSale, ItemEventState::Approved];

    EVENTS[is_approved as usize]
}
