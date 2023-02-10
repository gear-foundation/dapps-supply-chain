use gmeta::{metawasm, Metadata};
use gstd::{prelude::*, ActorId};
use supply_chain_io::*;

#[metawasm]
pub trait Metawasm {
    type State = <ContractMetadata as Metadata>::State;

    fn item_info(item_id: ItemId, state: Self::State) -> Option<ItemInfo> {
        state
            .items
            .into_iter()
            .find_map(|(some_item_id, item_info)| (some_item_id == item_id).then_some(item_info))
    }

    fn participants(state: Self::State) -> Participants {
        Participants {
            producers: state.producers,
            distributors: state.distributors,
            retailers: state.retailers,
        }
    }

    fn roles(actor: ActorId, state: Self::State) -> Vec<Role> {
        let mut roles = vec![Role::Consumer];

        if state.producers.contains(&actor) {
            roles.push(Role::Producer);
        }
        if state.distributors.contains(&actor) {
            roles.push(Role::Distributor);
        }
        if state.retailers.contains(&actor) {
            roles.push(Role::Retailer);
        }

        roles
    }

    fn existing_items(state: Self::State) -> Vec<(ItemId, ItemInfo)> {
        state.items
    }

    fn fungible_token(state: Self::State) -> ActorId {
        state.fungible_token
    }

    fn non_fungible_token(state: Self::State) -> ActorId {
        state.non_fungible_token
    }

    fn is_action_cached(actor_action: ActorIdInnerSupplyChainAction, state: Self::State) -> bool {
        let (actor, action) = actor_action;

        if let Some(action) = action.into() {
            state.cached_actions.contains(&(actor, action))
        } else {
            false
        }
    }
}

pub type ActorIdInnerSupplyChainAction = (ActorId, InnerAction);
