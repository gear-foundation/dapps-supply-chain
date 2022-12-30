use gear_lib::non_fungible_token::token::{TokenId, TokenMetadata};
use gstd::{errors::ContractError, prelude::*, ActorId};

/// An item ID.
///
/// Should equal [`TokenId`] of an item's NFT.
pub type ItemId = TokenId;

/// Initializes the Supply chain contract.
///
/// # Requirements
/// - Each [`ActorId`] of `producers`, `distributors`, and `retailers` mustn't
/// equal [`ActorId::zero()`].
#[derive(Encode, Decode, Hash, TypeInfo, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct SupplyChainInit {
    /// IDs of actors that'll have the right to interact with a supply chain on
    /// behalf of a producer.
    pub producers: Vec<ActorId>,
    /// IDs of actors that'll have the right to interact with a supply chain on
    /// behalf of a distributor.
    pub distributors: Vec<ActorId>,
    /// IDs of actors that'll have the right to interact with a supply chain on
    /// behalf of a retailer.
    pub retailers: Vec<ActorId>,

    /// A FT contract [`ActorId`].
    pub fungible_token: ActorId,
    /// An NFT contract [`ActorId`].
    pub non_fungible_token: ActorId,
}

/// Sends the contract info about what it should do.
#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct SupplyChainAction {
    pub action: InnerSupplyChainAction,
    pub kind: ActionKind,
}

impl SupplyChainAction {
    pub fn new(action: InnerSupplyChainAction) -> Self {
        Self {
            action,
            kind: ActionKind::New,
        }
    }

    pub fn to_retry(self) -> Self {
        Self {
            action: self.action,
            kind: ActionKind::Retry,
        }
    }
}

#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum InnerSupplyChainAction {
    Producer(ProducerAction),
    Distributor(DistributorAction),
    Retailer(RetailerAction),
    Consumer(ConsumerAction),
}

#[derive(
    Default, Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash,
)]
pub enum ActionKind {
    #[default]
    New,
    Retry,
}

/// Actions for a producer.
///
/// Should be used inside [`SupplyChainAction`].
#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum ProducerAction {
    /// Produces one item and a corresponding NFT with given `token_metadata`.
    ///
    /// Transfers the created NFT for the item to a producer
    /// ([`msg::source()`]).
    ///
    /// # Requirements
    /// - [`msg::source()`] must be a producer in a supply chain.
    ///
    /// On success, replies with [`SupplyChainEvent`] where [`ItemState`]
    /// contains [`ItemEventState::Produced`] & [`Role::Producer`].
    ///
    /// [`msg::source()`]: gstd::msg::source
    Produce {
        /// Item’s NFT metadata.
        token_metadata: TokenMetadata,
    },

    /// Puts a produced item up for sale to distributors for given `price` on
    /// behalf of a producer.
    ///
    /// Transfers an item's NFT to the Supply chain contract
    /// ([`exec::program_id()`](gstd::exec::program_id)).
    ///
    /// # Requirements
    /// - The item must exist in a supply chain.
    /// - [`msg::source()`](gstd::msg::source) must be the producer of the item.
    /// - Item's [`ItemState`] must contain [`ItemEventState::Produced`] &
    /// [`Role::Producer`].
    ///
    /// On success, replies with [`SupplyChainEvent`] where [`ItemState`]
    /// contains [`ItemEventState::ForSale`] & [`Role::Producer`].
    PutUpForSale {
        item_id: ItemId,
        /// An item's price.
        price: u128,
    },

    /// Approves or not a distributor's purchase on behalf of a producer.
    ///
    /// If the purchase is approved, then item's [`ItemEventState`] changes to
    /// [`Approved`](ItemEventState::Approved) and, from that moment, an item
    /// can be shipped (by [`ProducerAction::Ship`]).
    ///
    /// If the purchase is **not** approved, then fungible tokens for it are
    /// refunded from the Supply chain contract
    /// ([`exec::program_id()`](gstd::exec::program_id)) to the item's
    /// distributor and item's [`ItemEventState`] changes back to
    /// [`ForSale`](ItemEventState::ForSale).
    ///
    /// # Requirements
    /// - The item must exist in a supply chain.
    /// - [`msg::source()`](gstd::msg::source) must be the producer of the item.
    /// - Item's [`ItemState`] must contain [`ItemEventState::Produced`] &
    /// [`Role::Distributor`].
    ///
    /// On success, replies with [`SupplyChainEvent`] where [`ItemState`]
    /// contains [`ItemEventState::Approved`]/[`ItemEventState::ForSale`] &
    /// [`Role::Producer`].
    Approve {
        item_id: ItemId,
        /// Yes ([`true`]) or no ([`false`]).
        approve: bool,
    },

    /// Starts a shipping of a purchased item to a distributor on behalf of a
    /// producer.
    ///
    /// Starts the countdown for the delivery time specified for the item in
    /// [`DistributorAction::Purchase`].
    ///
    /// # Requirements
    /// - The item must exist in a supply chain.
    /// - [`msg::source()`](gstd::msg::source) must be the producer of the item.
    /// - Item's [`ItemState`] must contain [`ItemEventState::Approved`] &
    /// [`Role::Producer`].
    ///
    /// On success, replies with [`SupplyChainEvent`] where [`ItemState`]
    /// contains [`ItemEventState::Shipped`] & [`Role::Producer`].
    Ship(ItemId),
}

/// Actions for a distributor.
///
/// Should be used inside [`SupplyChainAction`].
#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum DistributorAction {
    /// Purchases an item from a producer on behalf of a distributor.
    ///
    /// Transfers fungible tokens for purchasing the item to the Supply chain
    /// contract ([`exec::program_id()`](gstd::exec::program_id)) until the item
    /// is received (by [`DistributorAction::Receive`]).
    ///
    /// **Note:** the item's producer must approve or not this purchase by
    /// [`ProducerAction::Approve`].
    ///
    /// # Requirements
    /// - The item must exist in a supply chain.
    /// - [`msg::source()`](gstd::msg::source) must be a distributor.
    /// - Item's [`ItemState`] must contain [`ItemEventState::ForSale`] &
    /// [`Role::Producer`].
    ///
    /// On success, replies with [`SupplyChainEvent`] where [`ItemState`]
    /// contains [`ItemEventState::Purchased`] & [`Role::Distributor`].
    Purchase {
        item_id: ItemId,
        /// Milliseconds during which the producer of an item should deliver it.
        /// A countdown starts after [`ProducerAction::Ship`] is executed.
        delivery_time: u64,
    },

    /// Receives a shipped item from a producer on behalf of a distributor.
    ///
    /// Depending on the time spent on a delivery, transfers fungible tokens for
    /// purchasing the item from the Supply chain contract
    /// ([`exec::program_id()`](gstd::exec::program_id)) to the item's producer
    /// or, as a penalty for being late, refunds a half or all of them to the
    /// item's distributor ([`msg::source()`]).
    ///
    /// Transfers an item's NFT to the distributor ([`msg::source()`]).
    ///
    /// # Requirements
    /// - The item must exist in a supply chain.
    /// - [`msg::source()`] must be the distributor of the item.
    /// - Item's [`ItemState`] must contain [`ItemEventState::Shipped`] &
    /// [`Role::Producer`].
    ///
    /// On success, replies with [`SupplyChainEvent`] where [`ItemState`]
    /// contains [`ItemEventState::Received`] & [`Role::Distributor`].
    ///
    /// [`msg::source()`]: gstd::msg::source
    Receive(ItemId),

    /// Processes a received item on behalf of a distributor.
    ///
    /// # Requirements
    /// - The item must exist in a supply chain.
    /// - [`msg::source()`](gstd::msg::source) must be the distributor of the
    /// item.
    /// - Item's [`ItemState`] must contain [`ItemEventState::Received`] &
    /// [`Role::Distributor`].
    ///
    /// On success, replies with [`SupplyChainEvent`] where [`ItemState`]
    /// contains [`ItemEventState::Processed`] & [`Role::Distributor`].
    Process(ItemId),

    /// Packages a processed item on behalf of a distributor.
    ///
    /// # Requirements
    /// - The item must exist in a supply chain.
    /// - [`msg::source()`](gstd::msg::source) must be the distributor of the
    /// item.
    /// - Item's [`ItemState`] must contain [`ItemEventState::Processed`] &
    /// [`Role::Distributor`].
    ///
    /// On success, replies with [`SupplyChainEvent`] where [`ItemState`]
    /// contains [`ItemEventState::Packaged`] & [`Role::Distributor`].
    Package(ItemId),

    /// Puts a packaged item up for sale to retailers for given `price` on
    /// behalf of a distributor.
    ///
    /// Transfers an item's NFT to the Supply chain contract
    /// ([`exec::program_id()`](gstd::exec::program_id)).
    ///
    /// # Requirements
    /// - The item must exist in a supply chain.
    /// - [`msg::source()`](gstd::msg::source) must be the distributor of the
    /// item.
    /// - Item's [`ItemState`] must contain [`ItemEventState::Packaged`] &
    /// [`Role::Distributor`].
    ///
    /// On success, replies with [`SupplyChainEvent`] where [`ItemState`]
    /// contains [`ItemEventState::ForSale`] & [`Role::Distributor`].
    PutUpForSale {
        item_id: ItemId,
        /// An item's price.
        price: u128,
    },

    /// Approves or not a retailer's purchase on behalf of a distributor.
    ///
    /// If the purchase is approved, then item's [`ItemEventState`] changes to
    /// [`Approved`](ItemEventState::Approved) and, from that moment, an item
    /// can be shipped (by [`DistributorAction::Ship`]).
    ///
    /// If the purchase is **not** approved, then fungible tokens for it are
    /// refunded from the Supply chain contract
    /// ([`exec::program_id()`](gstd::exec::program_id)) to the item's retailer
    /// and item's [`ItemEventState`] changes back to
    /// [`ForSale`](ItemEventState::ForSale).
    ///
    /// # Requirements
    /// - The item must exist in a supply chain.
    /// - [`msg::source()`](gstd::msg::source) must be the distributor of the
    /// item.
    /// - Item's [`ItemState`] must contain [`ItemEventState::Purchased`] &
    /// [`Role::Retailer`].
    ///
    /// On success, replies with [`SupplyChainEvent`] where [`ItemState`]
    /// contains [`ItemEventState::Approved`]/[`ItemEventState::ForSale`] &
    /// [`Role::Distributor`].
    Approve {
        item_id: ItemId,
        /// Yes ([`true`]) or no ([`false`]).
        approve: bool,
    },

    /// Starts a shipping of a purchased item to a retailer on behalf of a
    /// distributor.
    ///
    /// Starts the countdown for the delivery time specified for the item in
    /// [`RetailerAction::Purchase`].
    ///
    /// # Requirements
    /// - The item must exist in a supply chain.
    /// - [`msg::source()`](gstd::msg::source) must be the distributor of the
    /// item.
    /// - Item's [`ItemState`] must contain [`ItemEventState::Approved`] &
    /// [`Role::Distributor`].
    ///
    /// On success, replies with [`SupplyChainEvent`] where [`ItemState`]
    /// contains [`ItemEventState::Shipped`] & [`Role::Distributor`].
    Ship(ItemId),
}

/// Actions for a retailer.
///
/// Should be used inside [`SupplyChainAction`].
#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum RetailerAction {
    /// Purchases an item from a distributor on behalf of a retailer.
    ///
    /// Transfers fungible tokens for purchasing the item to the Supply chain
    /// contract ([`exec::program_id()`](gstd::exec::program_id)) until the item
    /// is received (by [`RetailerAction::Receive`]).
    ///
    /// **Note:** the item's distributor must approve or not this purchase by
    /// [`DistributorAction::Approve`].
    ///
    /// # Requirements
    /// - The item must exist in a supply chain.
    /// - [`msg::source()`](gstd::msg::source) must be a retailer.
    /// - Item's [`ItemState`] must contain [`ItemEventState::ForSale`] &
    /// [`Role::Distributor`].
    ///
    /// On success, replies with [`SupplyChainEvent`] where [`ItemState`]
    /// contains [`ItemEventState::Purchased`] & [`Role::Retailer`].
    Purchase {
        item_id: ItemId,
        /// Milliseconds during which the distributor of an item should deliver
        /// it. A countdown starts after [`DistributorAction::Ship`] is
        /// executed.
        delivery_time: u64,
    },

    /// Receives a shipped item from a distributor on behalf of a retailer.
    ///
    /// Depending on the time spent on a delivery, transfers fungible tokens for
    /// purchasing the item from the Supply chain contract
    /// ([`exec::program_id()`](gstd::exec::program_id)) to the item's
    /// distributor or, as a penalty for being late, refunds a half or all of
    /// them to the item's retailer ([`msg::source()`]).
    ///
    /// Transfers an item's NFT to the retailer ([`msg::source()`]).
    ///
    /// # Requirements
    /// - The item must exist in a supply chain.
    /// - [`msg::source()`] must be the retailer of the item.
    /// - Item's [`ItemState`] must contain [`ItemEventState::Shipped`] &
    /// [`Role::Distributor`].
    ///
    /// On success, replies with [`SupplyChainEvent`] where [`ItemState`]
    /// contains [`ItemEventState::Received`] & [`Role::Retailer`].
    ///
    /// [`msg::source()`]: gstd::msg::source
    Receive(ItemId),

    /// Puts a received item up for sale to consumers for given `price` on
    /// behalf of a retailer.
    ///
    /// Transfers an item's NFT to the Supply chain contract
    /// ([`exec::program_id()`](gstd::exec::program_id)).
    ///
    /// # Requirements
    /// - The item must exist in a supply chain.
    /// - [`msg::source()`](gstd::msg::source) must be the retailer of the item.
    /// - Item's [`ItemState`] must contain [`ItemEventState::Received`] &
    /// [`Role::Retailer`].
    ///
    /// On success, replies with [`SupplyChainEvent`] where [`ItemState`]
    /// contains [`ItemEventState::ForSale`] & [`Role::Retailer`].
    PutUpForSale {
        item_id: ItemId,
        /// An item's price.
        price: u128,
    },
}

/// Actions for a consumer.
///
/// Should be used inside [`SupplyChainAction`].
#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum ConsumerAction {
    /// Purchases an item from a retailer.
    ///
    /// Transfers fungible tokens for purchasing the item to its retailer.
    ///
    /// Transfers an item's NFT to the consumer
    /// ([`msg::source()`](gstd::msg::source)).
    ///
    /// # Requirements
    /// - The item must exist in a supply chain.
    /// - Item's [`ItemState`] must be
    /// - Item's [`ItemState`] must contain [`ItemEventState::ForSale`] &
    /// [`Role::Retailer`].
    ///
    /// On success, replies with [`SupplyChainEvent`] where [`ItemState`]
    /// contains [`ItemEventState::Purchased`] & [`Role::Consumer`].
    Purchase(ItemId),
}

/// A result of processed [`SupplyChainAction`].
#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct SupplyChainEvent {
    pub item_id: ItemId,
    pub item_state: ItemState,
}
#[derive(Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Clone, TypeInfo, Hash)]
pub enum SupplyChainError {
    ZeroActorId,
    UnexpectedTransactionAmount,
    TransactionNotFound,
    ItemNotFound,
    UnexpectedItemState,
    AccessRestricted,
    FTTransferFailed,
    NFTTransferFailed,
    NFTMintingFailed,
    ContractError(String),
}

impl From<ContractError> for SupplyChainError {
    fn from(error: ContractError) -> Self {
        Self::ContractError(error.to_string())
    }
}

/// Roles of supply chain participants.
#[derive(
    Encode, Decode, TypeInfo, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash,
)]
pub enum Role {
    Producer,
    Distributor,
    Retailer,
    #[default]
    Consumer,
}

/// Queries a program state.
///
/// On failure, returns a [`Default`] value.
#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum SupplyChainStateQuery {
    ItemInfo(ItemId),
    Participants,
    Roles(ActorId),
    ExistingItems,
    FungibleToken,
    NonFungibleToken,
}

/// A reply for queried [`SupplyChainStateQuery`].
#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub enum SupplyChainStateReply {
    ItemInfo(Option<ItemInfo>),
    Participants {
        producers: Vec<ActorId>,
        distributors: Vec<ActorId>,
        retailers: Vec<ActorId>,
    },
    FungibleToken(ActorId),
    NonFungibleToken(ActorId),
    ExistingItems(Vec<(ItemId, ItemInfo)>),
    Roles(Vec<Role>),
}

/// Item info.
#[derive(
    Encode, Decode, TypeInfo, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash,
)]
pub struct ItemInfo {
    /// Item’s producer [`ActorId`].
    pub producer: ActorId,
    /// [`ActorId`] of an item’s current or past distributor (depends on item’s
    /// `state`). If it equals [`ActorId::zero()`], then it means that an item
    /// has never had a distributor.
    pub distributor: ActorId,
    /// [`ActorId`] of an item’s current or past retailer (depends on item’s
    /// `state`). If it equals [`ActorId::zero()`], then it means that an item
    /// has never had a retailer.
    pub retailer: ActorId,

    pub state: ItemState,
    /// An item’s price. If it equals 0, then, depending on item’s `state`, an
    /// item is sold for free or has never been put up for sale.
    pub price: u128,
    /// Milliseconds during which a current seller should deliver an item.
    pub delivery_time: u64,
}

/// An item’s state.
#[derive(Encode, Decode, TypeInfo, Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct ItemState {
    pub state: ItemEventState,
    pub by: Role,
}

impl Default for ItemState {
    fn default() -> Self {
        Self {
            state: Default::default(),
            by: Role::Producer,
        }
    }
}

/// A part of [`ItemState`].
#[derive(
    Encode, Decode, TypeInfo, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash,
)]
pub enum ItemEventState {
    #[default]
    Produced,
    Purchased,
    Received,
    Processed,
    Packaged,
    ForSale,
    Approved,
    Shipped,
}
