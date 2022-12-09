use utils::{prelude::*, NonFungibleToken, Sft};

pub mod utils;

// Pairs of participants are needed here to test ownership of items.
const PRODUCER: [u64; 2] = [5, 6];
const DISTRIBUTOR: [u64; 2] = [7, 8];
const RETAILER: [u64; 2] = [9, 10];

#[test]
fn ownership_and_role() {
    let system = utils::initialize_system();

    let nft = NonFungibleToken::initialize(&system);
    let mut ft = Sft::initialize(&system);

    for from in [DISTRIBUTOR[0], RETAILER[0]] {
        ft.mint(from, ITEM_PRICE);
    }

    let supply_chain = SupplyChain::initialize_custom(
        &system,
        SupplyChainInit {
            producers: [PRODUCER[0].into(), PRODUCER[1].into()].into(),
            distributors: [DISTRIBUTOR[0].into(), DISTRIBUTOR[1].into()].into(),
            retailers: [RETAILER[0].into(), RETAILER[1].into()].into(),

            ft_actor_id: ft.actor_id(),
            nft_actor_id: nft.actor_id(),
        },
    )
    .succeed();

    for from in [DISTRIBUTOR[0], RETAILER[0]] {
        ft.approve(from, supply_chain.actor_id(), ITEM_PRICE);
    }

    // Should fail because `msg::source()` must be a producer.
    supply_chain.produce(FOREIGN_USER).failed();
    supply_chain.produce(PRODUCER[0]).contains(0);

    // Should fail because `msg::source()` must be a producer.
    supply_chain
        .put_up_for_sale_by_producer(FOREIGN_USER, 0, ITEM_PRICE)
        .failed();
    // Should fail because `msg::source()` must be the producer of the item.
    supply_chain
        .put_up_for_sale_by_producer(PRODUCER[1], 0, ITEM_PRICE)
        .failed();
    supply_chain
        .put_up_for_sale_by_producer(PRODUCER[0], 0, ITEM_PRICE)
        .contains(0);

    // Should fail because `msg::source()` must be a distributor.
    supply_chain
        .purchase_by_distributor(FOREIGN_USER, 0, DELIVERY_TIME)
        .failed();
    supply_chain
        .purchase_by_distributor(DISTRIBUTOR[0], 0, DELIVERY_TIME)
        .contains(0);

    // Should fail because `msg::source()` must be a producer.
    supply_chain
        .approve_by_producer(FOREIGN_USER, 0, true)
        .failed();
    // Should fail because `msg::source()` must be the producer of the item.
    supply_chain
        .approve_by_producer(PRODUCER[1], 0, true)
        .failed();
    supply_chain
        .approve_by_producer(PRODUCER[0], 0, true)
        .contains((0, true));

    // Should fail because `msg::source()` must be a producer.
    supply_chain.ship_by_producer(FOREIGN_USER, 0).failed();
    // Should fail because `msg::source()` must be the producer of the item.
    supply_chain.ship_by_producer(PRODUCER[1], 0).failed();
    supply_chain.ship_by_producer(PRODUCER[0], 0).contains(0);

    // Should fail because `msg::source()` must be a distributor.
    supply_chain
        .receive_by_distributor(FOREIGN_USER, 0)
        .failed();
    // Should fail because `msg::source()` must be the distributor of the item.
    supply_chain
        .receive_by_distributor(DISTRIBUTOR[1], 0)
        .failed();
    supply_chain
        .receive_by_distributor(DISTRIBUTOR[0], 0)
        .contains(0);

    // Should fail because `msg::source()` must be a distributor.
    supply_chain.process(FOREIGN_USER, 0).failed();
    // Should fail because `msg::source()` must be the distributor of the item.
    supply_chain.process(DISTRIBUTOR[1], 0).failed();
    supply_chain.process(DISTRIBUTOR[0], 0).contains(0);

    // Should fail because `msg::source()` must be a distributor.
    supply_chain.package(FOREIGN_USER, 0).failed();
    // Should fail because `msg::source()` must be the distributor of the item.
    supply_chain.package(DISTRIBUTOR[1], 0).failed();
    supply_chain.package(DISTRIBUTOR[0], 0).contains(0);

    // Should fail because `msg::source()` must be a distributor.
    supply_chain
        .put_up_for_sale_by_distributor(FOREIGN_USER, 0, ITEM_PRICE)
        .failed();
    // Should fail because `msg::source()` must be the distributor of the item.
    supply_chain
        .put_up_for_sale_by_distributor(DISTRIBUTOR[1], 0, ITEM_PRICE)
        .failed();
    supply_chain
        .put_up_for_sale_by_distributor(DISTRIBUTOR[0], 0, ITEM_PRICE)
        .contains(0);

    // Should fail because `msg::source()` must be a retailer.
    supply_chain
        .purchase_by_retailer(FOREIGN_USER, 0, DELIVERY_TIME)
        .failed();
    supply_chain
        .purchase_by_retailer(RETAILER[0], 0, DELIVERY_TIME)
        .contains(0);

    // Should fail because `msg::source()` must be a distributor.
    supply_chain
        .approve_by_distributor(FOREIGN_USER, 0, true)
        .failed();
    // Should fail because `msg::source()` must be the distributor of the item.
    supply_chain
        .approve_by_distributor(DISTRIBUTOR[1], 0, true)
        .failed();
    supply_chain
        .approve_by_distributor(DISTRIBUTOR[0], 0, true)
        .contains((0, true));

    // Should fail because `msg::source()` must be a distributor.
    supply_chain.ship_by_distributor(FOREIGN_USER, 0).failed();
    // Should fail because `msg::source()` must be the distributor of the item.
    supply_chain.ship_by_distributor(DISTRIBUTOR[1], 0).failed();
    supply_chain
        .ship_by_distributor(DISTRIBUTOR[0], 0)
        .contains(0);

    // Should fail because `msg::source()` must be a retailer.
    supply_chain.receive_by_retailer(FOREIGN_USER, 0).failed();
    // Should fail because `msg::source()` must be the retailer of the item.
    supply_chain.receive_by_retailer(RETAILER[1], 0).failed();
    supply_chain.receive_by_retailer(RETAILER[0], 0).contains(0);

    // Should fail because `msg::source()` must be a retailer.
    supply_chain
        .put_up_for_sale_by_retailer(FOREIGN_USER, 0, ITEM_PRICE)
        .failed();
    // Should fail because `msg::source()` must be the retailer of the item.
    supply_chain
        .put_up_for_sale_by_retailer(RETAILER[1], 0, ITEM_PRICE)
        .failed();
}

#[test]
fn query_roles() {
    let system = utils::initialize_system();

    let ft = Sft::initialize(&system);
    let nft = NonFungibleToken::initialize(&system);

    let mut supply_chain = SupplyChain::initialize_custom(
        &system,
        SupplyChainInit {
            producers: [FOREIGN_USER.into()].into(),
            distributors: [FOREIGN_USER.into()].into(),
            retailers: [FOREIGN_USER.into()].into(),

            ft_actor_id: ft.actor_id(),
            nft_actor_id: nft.actor_id(),
        },
    )
    .succeed();
    supply_chain.meta_state().roles(FOREIGN_USER).eq([
        Role::Consumer,
        Role::Producer,
        Role::Distributor,
        Role::Retailer,
    ]
    .into());

    supply_chain = SupplyChain::initialize_custom(
        &system,
        SupplyChainInit {
            producers: [].into(),
            distributors: [].into(),
            retailers: [].into(),

            ft_actor_id: ft.actor_id(),
            nft_actor_id: nft.actor_id(),
        },
    )
    .succeed();
    supply_chain
        .meta_state()
        .roles(FOREIGN_USER)
        .eq([Role::Consumer].into());
}
