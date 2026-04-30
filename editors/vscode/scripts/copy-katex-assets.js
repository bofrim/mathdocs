const fs = require('fs');
const path = require('path');

const katexCssPath = require.resolve('katex/dist/katex.min.css');
const katexDist = path.dirname(katexCssPath);
const targetDist = path.join(__dirname, '..', 'dist', 'katex');

fs.rmSync(targetDist, { recursive: true, force: true });
fs.mkdirSync(targetDist, { recursive: true });
fs.copyFileSync(katexCssPath, path.join(targetDist, 'katex.min.css'));
fs.cpSync(path.join(katexDist, 'fonts'), path.join(targetDist, 'fonts'), {
  recursive: true
});
