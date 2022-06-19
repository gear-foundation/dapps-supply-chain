pub mod utils;
use utils::*;

#[test]
fn interact_with_unexistend_item() {
    const NONEXISTEND_ITEM: u128 = 999999;

    let system = init_system();
    let supply_chain_program = init_supply_chain_program(&system);
    fail::put_up_for_sale_by_producer(&supply_chain_program, PRODUCER[0], NONEXISTEND_ITEM);
    fail::purchare_by_distributor(&supply_chain_program, DISTRIBUTOR[0], NONEXISTEND_ITEM);
    fail::approve_by_producer(&supply_chain_program, PRODUCER[0], NONEXISTEND_ITEM);
    fail::ship_by_producer(&supply_chain_program, PRODUCER[0], NONEXISTEND_ITEM);
    fail::receive_by_distributor(&supply_chain_program, DISTRIBUTOR[0], NONEXISTEND_ITEM);
    fail::process_by_distributor(&supply_chain_program, DISTRIBUTOR[0], NONEXISTEND_ITEM);
    fail::package_by_distributor(&supply_chain_program, DISTRIBUTOR[0], NONEXISTEND_ITEM);
    fail::put_up_for_sale_by_distributor(&supply_chain_program, DISTRIBUTOR[0], NONEXISTEND_ITEM);
    fail::purchare_by_retailer(&supply_chain_program, RETAILER[0], NONEXISTEND_ITEM);
    fail::approve_by_distributor(&supply_chain_program, DISTRIBUTOR[0], NONEXISTEND_ITEM);
    fail::ship_by_distributor(&supply_chain_program, DISTRIBUTOR[0], NONEXISTEND_ITEM);
    fail::receive_by_retailer(&supply_chain_program, RETAILER[0], NONEXISTEND_ITEM);
    fail::put_up_for_sale_by_retailer(&supply_chain_program, RETAILER[0], NONEXISTEND_ITEM);
    fail::purchare_by_consumer(&supply_chain_program, CONSUMER[0], NONEXISTEND_ITEM);
    // TODO: replace with the state check function when it becomes available.
    // fail::get_item_info(&supply_chain_program, NONEXISTEND_ITEM);
}

#[test]
fn init_with_zero_address() {
    use gstd::ActorId;
    const ZERO_ID: ActorId = ActorId::new([0; 32]);

    let system = init_system();

    let mut supply_chain_program = Program::current(&system);
    assert!(supply_chain_program
        .send(
            FOREIGN_USER,
            InitSupplyChain {
                ft_program_id: FT_PROGRAM_ID.into(),
                nft_program_id: NFT_PROGRAM_ID.into(),

                producers: BTreeSet::from([ZERO_ID, PRODUCER[1].into()]),
                distributors: BTreeSet::from([DISTRIBUTOR[0].into(), DISTRIBUTOR[1].into()]),
                retailers: BTreeSet::from([RETAILER[0].into(), RETAILER[1].into()]),
            },
        )
        .main_failed());

    supply_chain_program = Program::current(&system);
    assert!(supply_chain_program
        .send(
            FOREIGN_USER,
            InitSupplyChain {
                ft_program_id: FT_PROGRAM_ID.into(),
                nft_program_id: NFT_PROGRAM_ID.into(),

                producers: BTreeSet::from([PRODUCER[0].into(), PRODUCER[1].into()]),
                distributors: BTreeSet::from([DISTRIBUTOR[0].into(), ZERO_ID]),
                retailers: BTreeSet::from([RETAILER[0].into(), RETAILER[1].into()]),
            },
        )
        .main_failed());

    supply_chain_program = Program::current(&system);
    assert!(supply_chain_program
        .send(
            FOREIGN_USER,
            InitSupplyChain {
                ft_program_id: FT_PROGRAM_ID.into(),
                nft_program_id: NFT_PROGRAM_ID.into(),

                producers: BTreeSet::from([PRODUCER[0].into(), PRODUCER[1].into()]),
                distributors: BTreeSet::from([DISTRIBUTOR[0].into(), DISTRIBUTOR[1].into()]),
                retailers: BTreeSet::from([ZERO_ID, RETAILER[1].into()]),
            },
        )
        .main_failed());
}
