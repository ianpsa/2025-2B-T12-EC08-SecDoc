# Security Documentation

Documentação Security First - Portal de documentação para a organização Security/Safety da equipe Sec Robô.

## Executar o servidor de desenvolvimento

```bash
npm run dev
# ou
pnpm dev
# ou
yarn dev
```

Acesse http://localhost:3000 para visualizar a documentação.

## Estrutura do Projeto

- `lib/source.ts`: Adaptador de fonte de conteúdo, [`loader()`](https://fumadocs.dev/docs/headless/source-api) fornece a interface para acessar o conteúdo.
- `lib/layout.shared.tsx`: Opções compartilhadas para layouts.

| Rota                      | Descrição                                              |
| ------------------------- | ------------------------------------------------------ |
| `app/(home)`              | Redireciona automaticamente para /docs                 |
| `app/docs`                | Layout e páginas de documentação                       |
| `app/api/search/route.ts` | Route Handler para busca                               |

## Conteúdo

Toda a documentação está localizada em `content/docs/`:

- `index.mdx`: Página inicial da documentação
- `organizacao-equipe-security-first.mdx`: Documentação completa da organização Security/Safety

## Saiba Mais

- [Documentação Next.js](https://nextjs.org/docs)
- [Documentação Fumadocs](https://fumadocs.dev)
