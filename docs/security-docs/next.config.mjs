import { createMDX } from 'fumadocs-mdx/next';

const withMDX = createMDX();

const isProd = process.env.NODE_ENV === 'production';

/** @type {import('next').NextConfig} */
const config = {
  reactStrictMode: true,
  output: 'export',
  images: {
    unoptimized: true,
  },
  // Configuração para GitHub Pages - apenas em produção
  basePath: isProd ? '/2025-2B-T12-EC08-SecDoc' : '',
  assetPrefix: isProd ? '/2025-2B-T12-EC08-SecDoc/' : '',
};

export default withMDX(config);
