use utils::{prelude::*, NonFungibleToken, Sft};

pub mod utils;

#[test]
fn interact_with_unexistent_item() {
    const NONEXISTENT_ITEM: u128 = 99999999;

    let system = utils::initialize_system();

    let ft = Sft::initialize(&system);
    let nft = NonFungibleToken::initialize(&system);
    let supply_chain = SupplyChain::initialize(&system, ft.actor_id(), nft.actor_id());

    // Should fail because an item must exist in a supply chain.
    supply_chain
        .put_up_for_sale_by_producer(PRODUCER, NONEXISTENT_ITEM, ITEM_PRICE)
        .failed();
    // Should fail because an item must exist in a supply chain.
    supply_chain
        .purchase_by_distributor(DISTRIBUTOR, NONEXISTENT_ITEM, DELIVERY_TIME)
        .failed();
    // Should fail because an item must exist in a supply chain.
    supply_chain
        .approve_by_producer(PRODUCER, NONEXISTENT_ITEM, true)
        .failed();
    // Should fail because an item must exist in a supply chain.
    supply_chain
        .ship_by_producer(PRODUCER, NONEXISTENT_ITEM)
        .failed();
    // Should fail because an item must exist in a supply chain.
    supply_chain
        .receive_by_distributor(DISTRIBUTOR, NONEXISTENT_ITEM)
        .failed();
    // Should fail because an item must exist in a supply chain.
    supply_chain.process(DISTRIBUTOR, NONEXISTENT_ITEM).failed();
    // Should fail because an item must exist in a supply chain.
    supply_chain.package(DISTRIBUTOR, NONEXISTENT_ITEM).failed();
    // Should fail because an item must exist in a supply chain.
    supply_chain
        .put_up_for_sale_by_distributor(DISTRIBUTOR, NONEXISTENT_ITEM, ITEM_PRICE)
        .failed();
    // Should fail because an item must exist in a supply chain.
    supply_chain
        .purchase_by_retailer(RETAILER, NONEXISTENT_ITEM, DELIVERY_TIME)
        .failed();
    // Should fail because an item must exist in a supply chain.
    supply_chain
        .approve_by_distributor(DISTRIBUTOR, NONEXISTENT_ITEM, true)
        .failed();
    // Should fail because an item must exist in a supply chain.
    supply_chain
        .ship_by_distributor(DISTRIBUTOR, NONEXISTENT_ITEM)
        .failed();
    // Should fail because an item must exist in a supply chain.
    supply_chain
        .receive_by_retailer(RETAILER, NONEXISTENT_ITEM)
        .failed();
    // Should fail because an item must exist in a supply chain.
    supply_chain
        .put_up_for_sale_by_retailer(RETAILER, NONEXISTENT_ITEM, ITEM_PRICE)
        .failed();
    // Should fail because an item must exist in a supply chain.
    supply_chain
        .purchase_by_consumer(CONSUMER, NONEXISTENT_ITEM)
        .failed();

    // Should return `None` (`Default::default()`) because an item must exist in
    // a supply chain.
    supply_chain
        .meta_state()
        .item_info(NONEXISTENT_ITEM)
        .eq(None);
    supply_chain.meta_state().existing_items().eq([].into());
}

#[test]
fn initialization() {
    let system = utils::initialize_system();

    let ft = Sft::initialize(&system);
    let nft = NonFungibleToken::initialize(&system);

    let mut supply_chain_config = SupplyChainInit {
        producers: [ActorId::zero()].into(),
        distributors: [ActorId::zero()].into(),
        retailers: [ActorId::zero()].into(),

        ft_actor_id: ft.actor_id(),
        nft_actor_id: nft.actor_id(),
    };
    // Should fail because each [`ActorId`] of `producers`, `distributors`, and
    // `retailers` mustn't equal `ActorId::zero()`.
    SupplyChain::initialize_custom(&system, supply_chain_config.clone()).failed();

    supply_chain_config.producers = [PRODUCER.into()].into();
    // Should fail because each [`ActorId`] of `producers`, `distributors`, and
    // `retailers` mustn't equal `ActorId::zero()`.
    SupplyChain::initialize_custom(&system, supply_chain_config.clone()).failed();

    supply_chain_config.distributors = [DISTRIBUTOR.into()].into();
    // Should fail because each [`ActorId`] of `producers`, `distributors`, and
    // `retailers` mustn't equal `ActorId::zero()`.
    SupplyChain::initialize_custom(&system, supply_chain_config.clone()).failed();

    supply_chain_config.retailers = [RETAILER.into()].into();
    let supply_chain =
        SupplyChain::initialize_custom(&system, supply_chain_config.clone()).succeed();

    supply_chain
        .meta_state()
        .participants()
        .eq(SupplyChainStateReply::Participants {
            producers: supply_chain_config.producers,
            distributors: supply_chain_config.distributors,
            retailers: supply_chain_config.retailers,
        });
    supply_chain.meta_state().ft_program().eq(ft.actor_id());
    supply_chain.meta_state().nft_program().eq(nft.actor_id());
}

#[test]
fn query_existing_items() {
    let system = utils::initialize_system();

    let ft = Sft::initialize(&system);
    let nft = NonFungibleToken::initialize(&system);
    let supply_chain = SupplyChain::initialize(&system, ft.actor_id(), nft.actor_id());

    let mut items_info = BTreeMap::new();

    for item_id in 0..=5 {
        supply_chain.produce(PRODUCER).contains(item_id);
        items_info.insert(
            item_id.into(),
            ItemInfo {
                producer: PRODUCER.into(),
                distributor: Default::default(),
                retailer: Default::default(),

                state: ItemState {
                    state: Default::default(),
                    by: Role::Producer,
                },
                price: Default::default(),
                delivery_time: Default::default(),
            },
        );
    }

    supply_chain.meta_state().existing_items().eq(items_info);
}
