// Bundles the real panel components into a standalone browser fixture.
import { build } from '../node_modules/.pnpm/esbuild@0.28.1/node_modules/esbuild/lib/main.js'
import { readFileSync } from 'node:fs'
import { resolve, dirname } from 'node:path'

await build({
  entryPoints: ['evidence/push-mark.fixture.jsx'],
  outfile: 'evidence/push-mark.fixture.js',
  bundle: true,
  format: 'iife',
  platform: 'browser',
  jsx: 'automatic',
  target: ['chrome120'],
  loader: { '.css': 'text' },
  alias: {
    react: resolve('node_modules/.pnpm/react@18.3.1/node_modules/react'),
    'react/jsx-runtime': resolve('node_modules/.pnpm/react@18.3.1/node_modules/react/jsx-runtime'),
    'react-dom': resolve('node_modules/.pnpm/react-dom@18.3.1_react@18.3.1/node_modules/react-dom'),
  },
  plugins: [{
    name: 'css-raw',
    setup(plugin) {
      plugin.onResolve({ filter: /\?raw$/ }, args => ({ path: resolve(dirname(args.importer), args.path.slice(0, -4)), namespace: 'css-raw' }))
      plugin.onLoad({ filter: /.*/, namespace: 'css-raw' }, args => ({
        contents: `export default ${JSON.stringify(readFileSync(args.path, 'utf8'))}`, loader: 'js',
      }))
    },
  }],
})
console.log('bundled evidence/push-mark.fixture.js')
