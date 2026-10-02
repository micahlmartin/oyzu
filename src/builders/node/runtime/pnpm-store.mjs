// Export only pnpm 10.11's native v10 content/index store after frozen install.
// checkedAt is a verification shortcut, not package identity. Resetting it to
// zero makes pnpm recheck copied bytes instead of trusting acquisition times.
import {chmodSync, copyFileSync, existsSync, lstatSync, mkdirSync, readdirSync, readFileSync, writeFileSync} from 'node:fs';
import {join} from 'node:path';

function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (value !== null && typeof value === 'object') {
    return Object.fromEntries(Object.keys(value).sort().map(key => [key, canonical(value[key])]));
  }
  return value;
}

export function exportStore({output, inventory, temporary}) {
  if (inventory.version !== '10.11.0') throw new Error('pnpm dependency contexts require qualified pnpm 10.11.0 store semantics');
  const source = join(temporary, 'store');
  const destination = join(output, 'store');
  mkdirSync(destination);
  // Native pnpm need not create a store when the locked graph is empty.
  if (!existsSync(source) && inventory.packages.length === 0) return;
  let count = 0, bytes = 0;
  function copy(relative = '') {
    const from = join(source, relative), to = join(destination, relative);
    const stat = lstatSync(from);
    if (stat.isSymbolicLink() || (!stat.isDirectory() && !stat.isFile())) throw new Error('pnpm store export rejects links and special files');
    if (stat.isDirectory()) {
      if (relative !== '' && !/^(?:v10|v10\/(?:files|index)|v10\/(?:files|index)\/[a-f0-9]{2})$/.test(relative)) {
        throw new Error('unsupported native pnpm store directory');
      }
      if (relative) mkdirSync(to);
      for (const name of readdirSync(from).sort()) copy(relative ? `${relative}/${name}` : name);
      return;
    }
    if (++count > 100000 || (bytes += stat.size) > 10 * 1024 ** 3) throw new Error('pnpm store export exceeds capture limits');
    if (/^v10\/files\/[a-f0-9]{2}\/[a-f0-9]{126}(?:-exec)?$/.test(relative)) {
      copyFileSync(from, to);
      chmodSync(to, stat.mode & 0o777);
      return;
    }
    if (!/^v10\/index\/[a-f0-9]{2}\/[a-f0-9]{62}-[^/\\]+\.json$/.test(relative) || stat.size > 16 * 1024 ** 2) {
      throw new Error('unsupported native pnpm store file');
    }
    const index = JSON.parse(readFileSync(from, 'utf8'));
    if (typeof index.name !== 'string' || typeof index.version !== 'string'
        || Object.keys(index).some(key => !['name', 'version', 'requiresBuild', 'files'].includes(key))
        || !index.files || typeof index.files !== 'object' || Array.isArray(index.files)) {
      throw new Error('pnpm store export requires an unbuilt native file index');
    }
    for (const file of Object.values(index.files)) {
      if (!file || typeof file !== 'object' || Array.isArray(file) || !Number.isFinite(file.checkedAt)
          || Object.keys(file).some(key => !['checkedAt', 'integrity', 'mode', 'size'].includes(key))) {
        throw new Error('unsupported pnpm file verification metadata');
      }
      file.checkedAt = 0;
    }
    writeFileSync(to, JSON.stringify(canonical(index)) + '\n', {flag:'wx'});
  }
  copy();
}
