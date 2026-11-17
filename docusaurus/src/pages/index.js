import React from 'react';
import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';
import useBaseUrl from '@docusaurus/useBaseUrl';
import styles from './index.module.css';

const sections = [
  {
    title: 'Pesquisar ações e units',
    text: 'Encontre listagens spot verificadas por ticker e selecione a identidade junto com o ISIN.',
    link: '/docs/api-endpoints/equities',
    label: 'Abrir o picker',
  },
  {
    title: 'Entender os identificadores',
    text: 'Veja como ticker, ISIN, classe de segurança e estado de listagem se relacionam.',
    link: '/docs/schemas/equities',
    label: 'Ler os schemas',
  },
  {
    title: 'Consultar fechamentos',
    text: 'Leia o último fechamento ou um histórico de até um ano por ISIN.',
    link: '/docs/api-endpoints/equities',
    label: 'Ver endpoints',
  },
  {
    title: 'Verificar cobertura',
    text: 'Confira a atualização do conjunto verificado e as limitações dos preços nominais.',
    link: '/docs/operations/equity-data',
    label: 'Ler sobre cobertura',
  },
];

export default function Home() {
  const markUrl = useBaseUrl('/img/tucano-b3-mark.svg');
  return (
    <Layout
      title="Documentação"
      description="Guias para consumidores e operadores do Tucano B3."
    >
      <main className={styles.home}>
        <section className={styles.hero}>
          <div className={styles.heroCopy}>
            <p className={styles.kicker}>DADOS OFICIAIS · HISTÓRICO VERIFICÁVEL</p>
            <h1>Tucano B3</h1>
            <p className={styles.lead}>
              Consulte dados de mercado com precisão decimal, identidade contextualizada
              no tempo e proveniência até o registro da publicação de origem.
            </p>
            <div className={styles.actions}>
              <Link className="button button--primary button--lg" to="/docs/intro">
                Abrir documentação
              </Link>
              <Link className={styles.secondaryAction} to="/docs/api-endpoints/equities">
                Fazer uma consulta <span aria-hidden="true">→</span>
              </Link>
            </div>
          </div>
          <div className={styles.heroMark} aria-hidden="true">
            <img src={markUrl} alt="" />
          </div>
        </section>

        <section className={styles.status} aria-label="Escopo da documentação">
          <div>
            <span className={styles.statusLabel}>REST</span>
            <span>COTAHIST e ações verificadas</span>
          </div>
          <div>
            <span className={styles.statusLabel}>INTEGRIDADE</span>
            <span>Ticker, ISIN e cobertura</span>
          </div>
          <div>
            <span className={styles.statusLabel}>PREÇO</span>
            <span>Nominal e sem ajuste corporativo</span>
          </div>
        </section>

        <section className={styles.section}>
          <div className={styles.sectionHeader}>
            <p className={styles.kicker}>NAVEGAR POR TAREFA</p>
            <h2>Encontre o guia certo</h2>
          </div>
          <div className={styles.grid}>
            {sections.map((section) => (
              <article className={styles.card} key={section.title}>
                <h3>{section.title}</h3>
                <p>{section.text}</p>
                <Link to={section.link}>
                  {section.label} <span aria-hidden="true">→</span>
                </Link>
              </article>
            ))}
          </div>
        </section>

        <aside className={styles.notice}>
          <div>
            <p className={styles.kicker}>INTERPRETAÇÃO DE PREÇOS</p>
            <h2>COTAHIST é nominal e sem ajuste retroativo</h2>
            <p>
              Os valores representam a moeda e a forma de cotação da época. O dataset
              verificado aplica FATCOT para expressar o fechamento por ação ou unit e
              não ajusta preços por eventos corporativos.
            </p>
          </div>
          <Link to="/docs/operations/equity-data">Ler sobre a cobertura →</Link>
        </aside>

        <footer className={styles.homeFooter}>
          <span>Guias em português brasileiro</span>
          <Link to="/docs/api-endpoints/equities">Rotas e OpenAPI</Link>
        </footer>
      </main>
    </Layout>
  );
}
