use utils::{prelude::*, NonFungibleToken, Sft};

pub mod utils;

const DELIVERY_TIME_IN_BLOCKS: u32 = (DELIVERY_TIME / 1000) as _;

#[test]
fn delivery_wo_delay() {
    const NO_DELAY: u32 = DELIVERY_TIME_IN_BLOCKS;

    let system = utils::initialize_system();

    let nft = NonFungibleToken::initialize(&system);
    let mut ft = Sft::initialize(&system);
    let supply_chain = SupplyChain::initialize(&system, ft.actor_id(), nft.actor_id());

    for from in [DISTRIBUTOR, RETAILER] {
        ft.mint(from, ITEM_PRICE);
        ft.approve(from, supply_chain.actor_id(), ITEM_PRICE);
    }

    supply_chain.produce(PRODUCER).contains(0);
    supply_chain
        .put_up_for_sale_by_producer(PRODUCER, 0, ITEM_PRICE)
        .contains(0);
    supply_chain
        .purchase_by_distributor(DISTRIBUTOR, 0, DELIVERY_TIME)
        .contains(0);
    supply_chain
        .approve_by_producer(PRODUCER, 0, true)
        .contains((0, true));
    supply_chain.ship_by_producer(PRODUCER, 0).contains(0);

    system.spend_blocks(NO_DELAY);
    supply_chain
        .receive_by_distributor(DISTRIBUTOR, 0)
        .contains(0);
    // Since the delivery is completed on time,
    // all fungible tokens are transferred to the producer (seller).
    ft.balance(PRODUCER).contains(ITEM_PRICE);
    ft.balance(DISTRIBUTOR).contains(0);

    supply_chain.process(DISTRIBUTOR, 0).contains(0);
    supply_chain.package(DISTRIBUTOR, 0).contains(0);
    supply_chain
        .put_up_for_sale_by_distributor(DISTRIBUTOR, 0, ITEM_PRICE)
        .contains(0);
    supply_chain
        .purchase_by_retailer(RETAILER, 0, DELIVERY_TIME)
        .contains(0);
    supply_chain
        .approve_by_distributor(DISTRIBUTOR, 0, true)
        .contains((0, true));
    supply_chain.ship_by_distributor(DISTRIBUTOR, 0).contains(0);

    system.spend_blocks(NO_DELAY);
    supply_chain.receive_by_retailer(RETAILER, 0).contains(0);
    // Since the delivery is completed on time,
    // all fungible tokens are transferred to the distributor (seller).
    ft.balance(DISTRIBUTOR).contains(ITEM_PRICE);
    ft.balance(RETAILER).contains(0);
}

#[test]
fn delivery_with_delay() {
    // Even and odd prices required for a reliable penalty calculation check.
    const ITEM_PRICE: [u128; 2] = [123123, 12341234];
    const DELAY: u32 = DELIVERY_TIME_IN_BLOCKS * 2 - 1;

    let system = utils::initialize_system();

    let nft = NonFungibleToken::initialize(&system);
    let mut ft = Sft::initialize(&system);
    let supply_chain = SupplyChain::initialize(&system, ft.actor_id(), nft.actor_id());

    for (from, amount) in [(DISTRIBUTOR, ITEM_PRICE[0]), (RETAILER, ITEM_PRICE[1])] {
        ft.mint(from, amount);
        ft.approve(from, supply_chain.actor_id(), amount);
    }

    supply_chain.produce(PRODUCER).contains(0);
    supply_chain
        .put_up_for_sale_by_producer(PRODUCER, 0, ITEM_PRICE[0])
        .contains(0);
    supply_chain
        .purchase_by_distributor(DISTRIBUTOR, 0, DELIVERY_TIME)
        .contains(0);
    supply_chain
        .approve_by_producer(PRODUCER, 0, true)
        .contains((0, true));
    supply_chain.ship_by_producer(PRODUCER, 0).contains(0);

    system.spend_blocks(DELAY);
    supply_chain
        .receive_by_distributor(DISTRIBUTOR, 0)
        .contains(0);
    // Since the delivery is completed with the delay,
    // the half of fungible tokens is transferred to the producer (seller)
    // and the other half of them is refunded to the distributor (buyer).
    ft.balance(PRODUCER).contains(ITEM_PRICE[0] / 2);
    ft.balance(DISTRIBUTOR)
        .contains(ITEM_PRICE[0] - ITEM_PRICE[0] / 2);

    supply_chain.process(DISTRIBUTOR, 0).contains(0);
    supply_chain.package(DISTRIBUTOR, 0).contains(0);
    supply_chain
        .put_up_for_sale_by_distributor(DISTRIBUTOR, 0, ITEM_PRICE[1])
        .contains(0);
    supply_chain
        .purchase_by_retailer(RETAILER, 0, DELIVERY_TIME)
        .contains(0);
    supply_chain
        .approve_by_distributor(DISTRIBUTOR, 0, true)
        .contains((0, true));
    supply_chain.ship_by_distributor(DISTRIBUTOR, 0).contains(0);

    system.spend_blocks(DELAY);
    supply_chain.receive_by_retailer(RETAILER, 0).contains(0);
    // Since the delivery is completed with the delay,
    // the half of fungible tokens is transferred to the distributor (seller)
    // and the other half of them is refunded to the retailer (buyer).
    ft.balance(DISTRIBUTOR)
        .contains(ITEM_PRICE[0] - ITEM_PRICE[0] / 2 + ITEM_PRICE[1] / 2);
    ft.balance(RETAILER)
        .contains(ITEM_PRICE[1] - ITEM_PRICE[1] / 2);
}

#[test]
fn delivery_with_big_delay() {
    const BIG_DELAY: u32 = DELIVERY_TIME_IN_BLOCKS * 2;

    let system = utils::initialize_system();

    let nft = NonFungibleToken::initialize(&system);
    let mut ft = Sft::initialize(&system);

    let supply_chain = SupplyChain::initialize(&system, ft.actor_id(), nft.actor_id());

    for from in [DISTRIBUTOR, RETAILER] {
        ft.mint(from, ITEM_PRICE);
        ft.approve(from, supply_chain.actor_id(), ITEM_PRICE);
    }

    supply_chain.produce(PRODUCER).contains(0);
    supply_chain
        .put_up_for_sale_by_producer(PRODUCER, 0, ITEM_PRICE)
        .contains(0);
    supply_chain
        .purchase_by_distributor(DISTRIBUTOR, 0, DELIVERY_TIME)
        .contains(0);
    supply_chain
        .approve_by_producer(PRODUCER, 0, true)
        .contains((0, true));
    supply_chain.ship_by_producer(PRODUCER, 0).contains(0);

    system.spend_blocks(BIG_DELAY);
    supply_chain
        .receive_by_distributor(DISTRIBUTOR, 0)
        .contains(0);
    // Since the delivery is completed with the big delay,
    // all fungible tokens are refunded to the distributor (buyer).
    ft.balance(PRODUCER).contains(0);
    ft.balance(DISTRIBUTOR).contains(ITEM_PRICE);

    supply_chain.process(DISTRIBUTOR, 0).contains(0);
    supply_chain.package(DISTRIBUTOR, 0).contains(0);
    supply_chain
        .put_up_for_sale_by_distributor(DISTRIBUTOR, 0, ITEM_PRICE)
        .contains(0);
    supply_chain
        .purchase_by_retailer(RETAILER, 0, DELIVERY_TIME)
        .contains(0);
    supply_chain
        .approve_by_distributor(DISTRIBUTOR, 0, true)
        .contains((0, true));
    supply_chain.ship_by_distributor(DISTRIBUTOR, 0).contains(0);

    system.spend_blocks(BIG_DELAY);
    supply_chain.receive_by_retailer(RETAILER, 0).contains(0);
    // Since the delivery is completed with the big delay,
    // all fungible tokens are refunded to the retailer (buyer).
    ft.balance(DISTRIBUTOR).contains(ITEM_PRICE);
    ft.balance(RETAILER).contains(ITEM_PRICE);
}
