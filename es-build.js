import { build } from 'esbuild';

build({
  entryPoints: ['src/**/*.ts'],
  outdir: 'dist',
  bundle: false,
  platform: 'node',
  target: 'node16',
  format: 'esm'
}).catch(() => process.exit(1));
