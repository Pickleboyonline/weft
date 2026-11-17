import type {ReactNode} from 'react';
import Link from '@docusaurus/Link';
import Layout from '@theme/Layout';
import CodeBlock from '@theme/CodeBlock';
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

import styles from './index.module.css';

const ideas = [
  {
    title: 'One program is the spec',
    body: 'Write data, commands, views and the UI as if everything ran on one machine. That program is what everything else is checked against.',
  },
  {
    title: 'Where it runs is a separate file',
    body: 'deploy.weft says which machines exist, where each table lives and how fresh each screen must be. The logic stays the same.',
  },
  {
    title: 'A simulator checks the rest',
    body: 'Many clients, dropped messages and offline devices, run deterministically and compared with the one-machine program. A failure comes with a seed that replays it.',
  },
];

const sample = `command add_item!(list: Id, title: Text) {
  cleaned = title.trim()
  if cleaned.is_empty() { reject EmptyTitle }
  if not lists.has(list) { reject NoSuchList }   # deleted on another device?
  items.insert!(Item { list: list, title: cleaned })
}

view items_in(list: Id, show: Filter) -> List(Item) =
  items.filter(|i| i.list == list and keeps(show, i)).sort_by(.created)`;

export default function Home(): ReactNode {
  return (
    <Layout description="weft: write a system once, as if it ran on one machine; a compiler builds and checks the distributed version.">
      <header className={styles.hero}>
        <div className={styles.heroInner}>
          <ThemedImage
            className={styles.mark}
            alt=""
            sources={{
              light: useBaseUrl('/img/weft-mark-light.svg'),
              dark: useBaseUrl('/img/weft-mark-dark.svg'),
            }}
          />
          <h1 className={styles.title}>One program. Woven together.</h1>
          <p className={styles.subtitle}>
            weft is a design for a language where you write a system once, as if it ran on one
            machine, and a compiler builds the distributed version (server, browser clients,
            storage, sync) and checks it against what you wrote.
          </p>
          <p className={styles.status}>A design in progress: nothing compiles yet.</p>
          <div className={styles.buttons}>
            <Link className="button button--primary button--lg" to="/docs/">
              The big idea
            </Link>
            <Link className="button button--secondary button--lg" to="/docs/at-a-glance">
              Weft at a glance
            </Link>
          </div>
        </div>
      </header>
      <main className={styles.main}>
        <section className={styles.ideas}>
          {ideas.map((idea) => (
            <div key={idea.title} className={styles.idea}>
              <h2>{idea.title}</h2>
              <p>{idea.body}</p>
            </div>
          ))}
        </section>
        <section className={styles.sample}>
          <h2>What it looks like</h2>
          <p>
            A command and a view from the todo-lists example. No routes, no fetch calls, no SQL:
            the <code>!</code> marks what changes the world, and <code>reject</code> reasons become
            part of the command's type.
          </p>
          <CodeBlock language="weft" title="src/logic.weft">
            {sample}
          </CodeBlock>
        </section>
      </main>
    </Layout>
  );
}
