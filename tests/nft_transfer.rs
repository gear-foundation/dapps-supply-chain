use utils::{prelude::*, NonFungibleToken, Sft};

pub mod utils;

#[test]
fn nft_transfer() {
    let system = utils::initialize_system();

    let nft = NonFungibleToken::initialize(&system);
    let mut sft = Sft::initialize(&system);

    for from in [DISTRIBUTOR, RETAILER, CONSUMER] {
        sft.mint(from, ITEM_PRICE).contains(true);
    }

    let supply_chain = SupplyChain::initialize(&system, sft.actor_id(), nft.actor_id());

    for from in [DISTRIBUTOR, RETAILER, CONSUMER] {
        sft.approve(from, supply_chain.actor_id(), ITEM_PRICE)
            .contains(true);
    }

    supply_chain.produce(PRODUCER).contains(0);
    nft.meta_state().owner_id(0).eq(PRODUCER.into());

    supply_chain
        .put_up_for_sale_by_producer(PRODUCER, 0, ITEM_PRICE)
        .contains(0);
    nft.meta_state().owner_id(0).eq(supply_chain.actor_id());

    supply_chain
        .purchase_by_distributor(DISTRIBUTOR, 0, DELIVERY_TIME)
        .contains(0);
    supply_chain
        .approve_by_producer(PRODUCER, 0, true)
        .contains((0, true));
    supply_chain.ship_by_producer(PRODUCER, 0).contains(0);

    supply_chain
        .receive_by_distributor(DISTRIBUTOR, 0)
        .contains(0);
    nft.meta_state().owner_id(0).eq(DISTRIBUTOR.into());

    supply_chain.process(DISTRIBUTOR, 0).contains(0);
    supply_chain.package(DISTRIBUTOR, 0).contains(0);

    supply_chain
        .put_up_for_sale_by_distributor(DISTRIBUTOR, 0, ITEM_PRICE)
        .contains(0);
    nft.meta_state().owner_id(0).eq(supply_chain.actor_id());

    supply_chain
        .purchase_by_retailer(RETAILER, 0, DELIVERY_TIME)
        .contains(0);
    supply_chain
        .approve_by_distributor(DISTRIBUTOR, 0, true)
        .contains((0, true));
    supply_chain.ship_by_distributor(DISTRIBUTOR, 0).contains(0);

    supply_chain.receive_by_retailer(RETAILER, 0).contains(0);
    nft.meta_state().owner_id(0).eq(RETAILER.into());

    supply_chain
        .put_up_for_sale_by_retailer(RETAILER, 0, ITEM_PRICE)
        .contains(0);
    nft.meta_state().owner_id(0).eq(supply_chain.actor_id());

    supply_chain.purchase_by_consumer(CONSUMER, 0).contains(0);
    nft.meta_state().owner_id(0).eq(CONSUMER.into());

    supply_chain.meta_state().item_info(0).eq(Some(ItemInfo {
        producer: PRODUCER.into(),
        distributor: DISTRIBUTOR.into(),
        retailer: RETAILER.into(),

        state: ItemState {
            state: ItemEventState::Purchased,
            by: Role::Consumer,
        },
        price: ITEM_PRICE,
        delivery_time: DELIVERY_TIME,
    }));
    nft.meta_state().owner_id(0).eq(CONSUMER.into())
}
