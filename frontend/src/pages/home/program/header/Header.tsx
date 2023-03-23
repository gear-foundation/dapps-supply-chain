import { Button } from '@gear-js/ui';
import { HexString } from '@polkadot/util/types';
import styles from './Header.module.scss';

type Props = {
  programId: HexString;
  onBackButtonClick: () => void;
};

function Header({ programId, onBackButtonClick }: Props) {
  return (
    <header className={styles.header}>
      <Button text="Back" size="small" color="secondary" className={styles.button} onClick={onBackButtonClick} block />
      <p className={styles.text}>
        Current supply chain program ID: <span className={styles.programId}>{programId}</span>
      </p>
    </header>
  );
}

export { Header };
