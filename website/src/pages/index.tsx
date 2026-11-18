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
    body: 'Write data, commands, queries and the UI as if everything ran on one machine. That program is what everything else is checked against.',
  },
  {
    title: 'Remote functions with guarantees',
    body: 'The client calls the server through Query and Command values. Calls are typed and validated, writes refresh the screens that read them, retries apply once, and commands are atomic.',
  },
  {
    title: 'A simulator checks the rest',
    body: 'Many clients against one server, with dropped messages, delays and crashes, run deterministically and compared with the one-machine program. A failure comes with a seed that replays it.',
  },
];

const sample = `add_item = Command(|list: Id(TodoList), title: Text| -> Result(Id(Item), [EmptyTitle, NoSuchList]) {
  let cleaned = title.trim()
  if cleaned.is_empty() { return Err(EmptyTitle) }
  if not lists.has(list) { return Err(NoSuchList) }   // deleted on another device?
  Ok(items.insert(Item { list, title: cleaned }))
})

items_in = Query(|list: Id(TodoList), show: Filter| -> List(Item) {
  items
    .filter(|i| i.list == list and keeps(show, i))
    .sort_by(.created)
})`;

export default function Home(): ReactNode {
  return (
    <Layout description="weft: write a system once, as if it ran on one machine; a compiler builds the server and clients and checks them against it.">
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
            machine, and a compiler builds the running version (a server with its database,
            browser clients, the typed calls between them) and checks it against what you wrote.
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
            A command and a query from the todo-lists example: the client calls both as if they
            were local. No routes, no fetch calls, no SQL: the compiler infers what each function
            reads and writes, the query compiles to one database query, and the reasons a command
            can fail are part of its type.
          </p>
          <CodeBlock language="weft" title="commands/items.weft, queries/items.weft">
            {sample}
          </CodeBlock>
        </section>
      </main>
    </Layout>
  );
}
