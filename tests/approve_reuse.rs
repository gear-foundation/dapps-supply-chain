use utils::{prelude::*, NonFungibleToken, Sft};

pub mod utils;

const ITEM_PRICE_BY_PRODUCER: u128 = ITEM_PRICE;
const ITEM_PRICE_BY_DISTRIBUTOR: u128 = ITEM_PRICE * 2;

#[test]
fn approve_reuse_and_ft_transfer() {
    let system = utils::initialize_system();

    let nft = NonFungibleToken::initialize(&system);
    let mut sft = Sft::initialize(&system);
    let supply_chain = SupplyChain::initialize(&system, sft.actor_id(), nft.actor_id());

    for (from, amount) in [
        (DISTRIBUTOR, ITEM_PRICE_BY_PRODUCER),
        (RETAILER, ITEM_PRICE_BY_DISTRIBUTOR),
    ] {
        sft.mint(from, amount);
        sft.approve(from, supply_chain.actor_id(), amount * 2);
    }

    supply_chain.produce(PRODUCER).contains(0);
    supply_chain
        .put_up_for_sale_by_producer(PRODUCER, 0, ITEM_PRICE_BY_PRODUCER)
        .contains(0);
    supply_chain
        .meta_state()
        .item_price(0)
        .eq(Some(ITEM_PRICE_BY_PRODUCER));

    // There may be a case when a buyer puts an inconvenient delivery time for a
    // seller.
    supply_chain
        .purchase_by_distributor(DISTRIBUTOR, 0, DELIVERY_TIME)
        .contains(0);
    sft.balance(supply_chain.actor_id())
        .contains(ITEM_PRICE_BY_PRODUCER);
    // Then the seller can cancel this purchase and put its item back up for
    // sale.
    supply_chain
        .approve_by_producer(PRODUCER, 0, false)
        .contains((0, false));
    sft.balance(DISTRIBUTOR).contains(ITEM_PRICE_BY_PRODUCER);
    // Thereafter the same buyer or another can purchase this item again and put
    // a more convenient delivery time for the seller...
    supply_chain
        .purchase_by_distributor(DISTRIBUTOR, 0, DELIVERY_TIME)
        .contains(0);
    sft.balance(supply_chain.actor_id())
        .contains(ITEM_PRICE_BY_PRODUCER);
    // ...who will approve this purchase and ship the item later.
    supply_chain
        .approve_by_producer(PRODUCER, 0, true)
        .contains((0, true));

    supply_chain.ship_by_producer(PRODUCER, 0).contains(0);
    supply_chain
        .receive_by_distributor(DISTRIBUTOR, 0)
        .contains(0);
    supply_chain.process(DISTRIBUTOR, 0).contains(0);
    supply_chain.package(DISTRIBUTOR, 0).contains(0);
    supply_chain
        .put_up_for_sale_by_distributor(DISTRIBUTOR, 0, ITEM_PRICE_BY_DISTRIBUTOR)
        .contains(0);
    supply_chain
        .meta_state()
        .item_price(0)
        .eq(Some(ITEM_PRICE_BY_DISTRIBUTOR));

    // There may be a case when a buyer puts an inconvenient delivery time for a
    // seller.
    supply_chain
        .purchase_by_retailer(RETAILER, 0, DELIVERY_TIME)
        .contains(0);
    sft.balance(supply_chain.actor_id())
        .contains(ITEM_PRICE_BY_DISTRIBUTOR);
    // Then the seller can cancel this purchase and put its item back up for
    // sale.
    supply_chain
        .approve_by_distributor(DISTRIBUTOR, 0, false)
        .contains((0, false));
    sft.balance(RETAILER).contains(ITEM_PRICE_BY_DISTRIBUTOR);
    // Thereafter the same buyer or another can purchase this item again and put
    // a more convenient delivery time for the seller...
    supply_chain
        .purchase_by_retailer(RETAILER, 0, DELIVERY_TIME)
        .contains(0);
    sft.balance(supply_chain.actor_id())
        .contains(ITEM_PRICE_BY_DISTRIBUTOR);
    // ...who will approve this purchase and ship the item later.
    supply_chain
        .approve_by_distributor(DISTRIBUTOR, 0, true)
        .contains((0, true));
}
