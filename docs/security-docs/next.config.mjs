import { createMDX } from 'fumadocs-mdx/next';

const withMDX = createMDX();

/** @type {import('next').NextConfig} */
const config = {
  reactStrictMode: true,
  output: 'export',
  images: {
    unoptimized: true,
  },
  // Configuração para GitHub Pages
  // Se o repositório não for username.github.io, descomente e configure o basePath
  basePath: '/2025-2B-T12-EC08-SecDoc',
  assetPrefix: '/2025-2B-T12-EC08-SecDoc/',
};

export default withMDX(config);
