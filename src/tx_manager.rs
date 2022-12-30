use crate::io::*;
use gstd::{prelude::*, ActorId};
use hashbrown::HashMap;

const MAX_NUMBER_OF_TXS: usize = 2usize.pow(16) - 1;

#[derive(Default, Debug, PartialEq)]
pub struct TransactionManager {
    txs_for_actor: BTreeMap<u64, ActorId>,
    actors_for_tx: HashMap<ActorId, (u64, u8)>,

    tx_id_nonce: u64,
}

impl TransactionManager {
    pub fn asquire_transactions(
        &mut self,
        kind: ActionKind,
        msg_source: ActorId,
        amount: u8,
    ) -> Result<TransactionGuard, SupplyChainError> {
        let tx_id = match kind {
            ActionKind::New => {
                let id = self.tx_id_nonce;

                self.tx_id_nonce = id.wrapping_add(amount as u64);

                if self.txs_for_actor.len() == MAX_NUMBER_OF_TXS {
                    let (tx, actor) = self
                        .txs_for_actor
                        .range(self.tx_id_nonce..)
                        .next()
                        .unwrap_or_else(|| self.txs_for_actor.first_key_value().unwrap());
                    let (tx, actor) = (*tx, *actor);

                    self.txs_for_actor.remove(&tx);
                    self.actors_for_tx.remove(&actor);
                }

                self.txs_for_actor.insert(id, msg_source);
                self.actors_for_tx.insert(msg_source, (id, amount));

                id
            }
            ActionKind::Retry => {
                let (id, real_amount) = *self
                    .actors_for_tx
                    .get(&msg_source)
                    .ok_or(SupplyChainError::TransactionNotFound)?;

                if amount != real_amount {
                    return Err(SupplyChainError::UnexpectedTransactionAmount);
                }

                id
            }
        };

        Ok(TransactionGuard {
            manager: self,
            msg_source,
            tx_id,
        })
    }

    pub fn asquire_transaction(
        &mut self,
        kind: ActionKind,
        msg_source: ActorId,
    ) -> Result<TransactionGuard, SupplyChainError> {
        self.asquire_transactions(kind, msg_source, 1)
    }
}

#[derive(Debug, PartialEq)]
pub struct TransactionGuard<'a> {
    manager: &'a mut TransactionManager,
    msg_source: ActorId,
    pub tx_id: u64,
}

impl Drop for TransactionGuard<'_> {
    fn drop(&mut self) {
        let manager = &mut self.manager;

        manager.txs_for_actor.remove(&self.tx_id);
        manager.actors_for_tx.remove(&self.msg_source);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors() {
        let mut tx_manager = TransactionManager::default();

        assert_eq!(
            tx_manager.asquire_transaction(ActionKind::Retry, ActorId::zero()),
            Err(SupplyChainError::TransactionNotFound)
        );

        mem::forget(
            tx_manager
                .asquire_transaction(ActionKind::New, ActorId::zero())
                .unwrap(),
        );

        assert_eq!(
            tx_manager.asquire_transactions(ActionKind::Retry, ActorId::zero(), 2),
            Err(SupplyChainError::UnexpectedTransactionAmount)
        );
    }

    #[test]

    fn tx_cycle() {
        const MAX_TX_AMOUNT: u64 = u8::MAX as _;

        let mut tx_count = MAX_NUMBER_OF_TXS as u64 + 1;
        let mut latest_tx_id = tx_count * MAX_TX_AMOUNT;

        let make_tx = |start, mut tx_id| {
            tx_id = start + tx_id * MAX_TX_AMOUNT;

            let actor = ActorId::from(tx_id);

            ((tx_id, actor), (actor, (tx_id, u8::MAX)))
        };

        // `|` - an empty slot.
        // `-` - a tx being removed.
        // `X` - an occupied slot.
        // `+` - a tx being added.

        // ( | - X X X X X X X X X + | | | | | | | | )

        let mut prepared_txs = (1..tx_count).map(|tx_id| make_tx(0, tx_id)).unzip();
        let mut expected_txs = prepared_txs.clone();
        let mut tx_manager = TransactionManager {
            txs_for_actor: prepared_txs.0,
            actors_for_tx: prepared_txs.1,

            tx_id_nonce: latest_tx_id,
        };

        mem::forget(
            tx_manager
                .asquire_transaction(ActionKind::New, ActorId::zero())
                .unwrap(),
        );

        expected_txs.0.remove(&MAX_TX_AMOUNT);
        expected_txs.1.remove(&ActorId::from(MAX_TX_AMOUNT));

        expected_txs.0.insert(latest_tx_id, ActorId::zero());
        expected_txs.1.insert(ActorId::zero(), (latest_tx_id, 1));

        // DON'T use the assert_eq!() here because in case of a failure it'll
        // clog up stdout.
        assert!(
            (tx_manager.txs_for_actor, tx_manager.actors_for_tx)
                == (expected_txs.0, expected_txs.1)
        );

        // ( X X X X + | | | | | | | | | - X X X X X )

        latest_tx_id /= 2;
        tx_count /= 2;
        let middle_tx_id = u64::MAX - latest_tx_id;

        prepared_txs = (0..tx_count)
            .map(|tx_id| make_tx(middle_tx_id, tx_id))
            .unzip();

        prepared_txs.extend((1..tx_count).map(|tx_id| make_tx(0, tx_id)));
        expected_txs = prepared_txs.clone();
        tx_manager = TransactionManager {
            txs_for_actor: prepared_txs.0,
            actors_for_tx: prepared_txs.1,

            tx_id_nonce: latest_tx_id,
        };

        mem::forget(
            tx_manager
                .asquire_transaction(ActionKind::New, ActorId::zero())
                .unwrap(),
        );

        expected_txs.0.remove(&middle_tx_id);
        expected_txs.1.remove(&ActorId::from(middle_tx_id));

        expected_txs.0.insert(latest_tx_id, ActorId::zero());
        expected_txs.1.insert(ActorId::zero(), (latest_tx_id, 1));

        // DON'T use the assert_eq!() here because in case of a failure it'll
        // clog up stdout.
        assert!(
            (tx_manager.txs_for_actor, tx_manager.actors_for_tx)
                == (expected_txs.0, expected_txs.1)
        );
    }
}
