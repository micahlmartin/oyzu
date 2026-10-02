// Normalize transport metadata after native Yarn selects and packs the files.
// tar-stream handles USTAR/PAX parsing; archive members are never extracted.
import {createReadStream, createWriteStream, mkdtempSync, renameSync, rmSync} from 'node:fs';
import {createRequire} from 'node:module';
import {dirname, join, resolve} from 'node:path';
import {Transform} from 'node:stream';
import {pipeline} from 'node:stream/promises';
import {createGunzip, createGzip} from 'node:zlib';

const [filename, modulePath] = process.argv.slice(2);
if (!filename || !modulePath) throw new Error('expected native archive and provisioned tar-stream module');
const tar = createRequire(import.meta.url)(resolve(modulePath));
const directory = mkdtempSync(join(dirname(filename), '.oyzu-normalize-'));
const temporary = join(directory, 'package.tgz');
const extract = tar.extract();
const pack = tar.pack();
let bytes = 0;
let count = 0;
const bound = new Transform({transform(chunk, encoding, callback) {
  bytes += chunk.length;
  callback(bytes > 256 * 1024 * 1024 ? new Error('Node archive exceeds 256 MiB normalization limit') : null, chunk);
}});
const streams = [createReadStream(filename), createGunzip(), bound, extract, pack, createGzip(), createWriteStream(temporary, {flags:'wx'})];
const processing = (async () => {
  for await (const entry of extract) {
    if (++count > 100000) throw new Error('Node archive exceeds entry limit');
    const header = {...entry.header, mtime:new Date(0), uid:0, gid:0, uname:'', gname:''};
    if (header.pax) {
      header.pax = {...header.pax};
      // Structural PAX values have already been resolved onto the header. Let
      // the writer regenerate those instead of duplicating path/link records.
      for (const key of ['path','linkpath','size','mtime','atime','ctime','birthtime','uid','gid','uname','gname']) delete header.pax[key];
      if (!Object.keys(header.pax).length) delete header.pax;
    }
    // Retain native names, ordering, modes, links and bytes. No path is used as
    // a filesystem destination, including extended-header and symlink paths.
    const chunks = [];
    for await (const chunk of entry) chunks.push(chunk);
    await new Promise((accept, reject) => pack.entry(header, Buffer.concat(chunks), error => error ? reject(error) : accept()));
  }
  pack.finalize();
})();
const pending = [processing, pipeline(...streams.slice(0,4)), pipeline(...streams.slice(4))];
try {
  await Promise.all(pending);
  renameSync(temporary, filename);
} catch (error) {
  for (const stream of streams) stream.destroy();
  await Promise.allSettled(pending);
  throw error;
} finally { rmSync(directory, {recursive:true, force:true}); }
