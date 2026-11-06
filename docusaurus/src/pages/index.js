import React from 'react';
import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';
import useBaseUrl from '@docusaurus/useBaseUrl';
import styles from './index.module.css';

const sections = [
  {
    title: 'Começar a consultar',
    text: 'Configure credenciais, entenda paginação e faça uma consulta COTAHIST com revisão fixa.',
    link: '/docs/getting-started/quickstart',
    label: 'Primeira consulta',
  },
  {
    title: 'Entender os dados',
    text: 'Veja como fontes, fotografias, identidade temporal e proveniência se relacionam.',
    link: '/docs/concepts/data-model',
    label: 'Conceitos do domínio',
  },
  {
    title: 'Consultar a API',
    text: 'Explore filtros, permissões, respostas e estados de erro de cada grupo de endpoints.',
    link: '/docs/api-endpoints/common-patterns',
    label: 'Referência REST',
  },
  {
    title: 'Operar ingestão',
    text: 'Acompanhe descoberta, jobs duráveis, revisões, membros de arquivos e quarentena.',
    link: '/docs/operations/overview',
    label: 'Guias de operação',
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
              <Link className={styles.secondaryAction} to="/docs/getting-started/quickstart">
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
            <span>COTAHIST, cadastro e operações</span>
          </div>
          <div>
            <span className={styles.statusLabel}>INTEGRIDADE</span>
            <span>Revisão, identidade e proveniência</span>
          </div>
          <div>
            <span className={styles.statusLabel}>MCP</span>
            <span>Consultas analíticas somente leitura</span>
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
              Os valores representam a moeda e a forma de cotação da época. O serviço
              não ajusta preços por inflação ou eventos corporativos, nem converte
              PREULT em um fechamento genérico.
            </p>
          </div>
          <Link to="/docs/data-sources/cotahist">Ler sobre COTAHIST →</Link>
        </aside>

        <footer className={styles.homeFooter}>
          <span>Guias em português brasileiro</span>
          <Link to="/docs/api-endpoints/health">OpenAPI e saúde da API</Link>
        </footer>
      </main>
    </Layout>
  );
}
