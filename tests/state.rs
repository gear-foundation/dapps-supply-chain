use utils::{prelude::*, NonFungibleToken, Sft};

pub mod utils;

#[test]
fn state() {
    let system = utils::initialize_system();

    let nft = NonFungibleToken::initialize(&system);
    let mut sft = Sft::initialize(&system);

    for from in [DISTRIBUTOR, RETAILER, CONSUMER] {
        // Double the balances to catch bugs.
        sft.mint(from, ITEM_PRICE * 2).contains(true);
    }

    let supply_chain = SupplyChain::initialize(&system, sft.actor_id(), nft.actor_id());

    for from in [DISTRIBUTOR, RETAILER, CONSUMER] {
        sft.approve(from, supply_chain.actor_id(), ITEM_PRICE * 2)
            .contains(true);
    }

    supply_chain.produce(PRODUCER).contains(0);

    supply_chain
        .put_up_for_sale_by_producer(PRODUCER, 0, ITEM_PRICE)
        .contains(0);
    // Should fail because item's `ItemState` must contain
    // `ItemEventState::Produced` & `Role::Producer`.
    supply_chain
        .put_up_for_sale_by_producer(PRODUCER, 0, ITEM_PRICE)
        .failed();

    supply_chain
        .purchase_by_distributor(DISTRIBUTOR, 0, DELIVERY_TIME)
        .contains(0);
    // Should fail because item's `ItemState` must contain
    // `ItemEventState::ForSale` & `Role::Producer`.
    supply_chain
        .purchase_by_distributor(DISTRIBUTOR, 0, DELIVERY_TIME)
        .failed();

    supply_chain
        .approve_by_producer(PRODUCER, 0, true)
        .contains((0, true));
    // Should fail because item's `ItemState` must contain
    // `ItemEventState::Purchased` & `Role::Distributor`.
    supply_chain.approve_by_producer(PRODUCER, 0, true).failed();

    supply_chain.ship_by_producer(PRODUCER, 0).contains(0);
    // Should fail because item's `ItemState` must contain
    // `ItemEventState::Approved` & `Role::Producer`.
    supply_chain.ship_by_producer(PRODUCER, 0).failed();

    supply_chain
        .receive_by_distributor(DISTRIBUTOR, 0)
        .contains(0);
    // Should fail because item's `ItemState` must contain
    // `ItemEventState::Shipped` & `Role::Producer`.
    supply_chain.receive_by_distributor(DISTRIBUTOR, 0).failed();

    supply_chain.process(DISTRIBUTOR, 0).contains(0);
    // Should fail because item's `ItemState` must contain
    // `ItemEventState::Received` & `Role::Distributor`.
    supply_chain.process(DISTRIBUTOR, 0).failed();

    supply_chain.package(DISTRIBUTOR, 0).contains(0);
    // Should fail because item's `ItemState` must contain
    // `ItemEventState::Processed` & `Role::Distributor`.
    supply_chain.package(DISTRIBUTOR, 0).failed();

    supply_chain
        .put_up_for_sale_by_distributor(DISTRIBUTOR, 0, ITEM_PRICE)
        .contains(0);
    // Should fail because item's `ItemState` must contain
    // `ItemEventState::Packaged` & `Role::Distributor`.
    supply_chain
        .put_up_for_sale_by_distributor(DISTRIBUTOR, 0, ITEM_PRICE)
        .failed();

    supply_chain
        .purchase_by_retailer(RETAILER, 0, DELIVERY_TIME)
        .contains(0);
    // Should fail because item's `ItemState` must contain
    // `ItemEventState::ForSale` & `Role::Distributor`.
    supply_chain
        .purchase_by_retailer(RETAILER, 0, DELIVERY_TIME)
        .failed();

    supply_chain
        .approve_by_distributor(DISTRIBUTOR, 0, true)
        .contains((0, true));
    // Should fail because item's `ItemState` must contain
    // `ItemEventState::Purchased` & `Role::Retailer`.
    supply_chain
        .approve_by_distributor(DISTRIBUTOR, 0, true)
        .failed();

    supply_chain.ship_by_distributor(DISTRIBUTOR, 0).contains(0);
    // Should fail because item's `ItemState` must contain
    // `ItemEventState::Approved` & `Role::Distributor`.
    supply_chain.ship_by_distributor(DISTRIBUTOR, 0).failed();

    supply_chain.receive_by_retailer(RETAILER, 0).contains(0);
    // Should fail because item's `ItemState` must contain
    // `ItemEventState::Shipped` & `Role::Distributor`.
    supply_chain.receive_by_retailer(RETAILER, 0).failed();

    supply_chain
        .put_up_for_sale_by_retailer(RETAILER, 0, ITEM_PRICE)
        .contains(0);
    // Should fail because item's `ItemState` must contain
    // `ItemEventState::Received` & `Role::Retailer`.
    supply_chain
        .put_up_for_sale_by_retailer(RETAILER, 0, ITEM_PRICE)
        .failed();

    supply_chain.purchase_by_consumer(CONSUMER, 0).contains(0);
    // Should fail because item's `ItemState` must contain
    // `ItemEventState::ForSale` & `Role::Retailer`.
    supply_chain.purchase_by_consumer(CONSUMER, 0).failed();
}
