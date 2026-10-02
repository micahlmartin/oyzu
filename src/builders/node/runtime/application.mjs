// Stage one known native output directory. The engine owns its complete bounded
// inventory, hashing, unsafe-entry rejection and consumer materialization.
import {cpSync, existsSync, lstatSync} from 'node:fs';
import {isAbsolute, join, resolve} from 'node:path';

const [source, destination, ...extra] = process.argv.slice(2);
if (!source || !destination || extra.length || isAbsolute(source)
    || /[\\:]/.test(source) || source.split('/').some(p => !p || p === '.' || p === '..')) {
  throw new Error('Expected a contained native output directory and artifact destination');
}
let input = process.cwd();
for (const part of source.split('/')) {
  input = join(input, part);
  const entry = lstatSync(input);
  if (entry.isSymbolicLink() || !entry.isDirectory()) throw new Error('Native application output must be a directory without linked parents');
}
const output = resolve(destination);
if (output === input || output.startsWith(input + '/') || output.startsWith(input + '\\') || existsSync(output)) {
  throw new Error('Application artifact destination must be new and outside native output');
}
let count = 0, bytes = 0;
cpSync(input, output, {recursive:true, dereference:false, force:false, errorOnExist:true,
  filter(path) {
    const entry = lstatSync(path);
    if (entry.isSymbolicLink() || (!entry.isDirectory() && !entry.isFile())) {
      throw new Error('Native output contains a link or unsupported entry');
    }
    if (entry.isFile()) bytes += entry.size;
    if (++count > 100001 || bytes > 10 * 1024 ** 3) throw new Error('Native output exceeds staging limits');
    return true;
  },
});
