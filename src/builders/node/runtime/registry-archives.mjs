// Immutable archive capture/replay shared by native managers. Callers supply
// admitted identities; this module does not interpret locks or install packages.
import {existsSync, mkdirSync, readFileSync, writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {fetch} from './broker_transport.mjs';
import {verify} from './integrity.mjs';

export async function archives({mode, output, broker, captured}, entries) {
  const packages = [];
  if (mode === 'acquire') mkdirSync(join(output, 'tarballs'), {recursive:true});
  for (const entry of entries) {
    let body, sourceId;
    if (mode === 'acquire') {
      const response = await fetch(entry.url, broker);
      if (response.info.status !== 200) throw new Error(`registry acquisition denied or failed: ${entry.id}`);
      ({body} = response); sourceId = response.info.sourceId;
    } else {
      const stored = captured.packages.find(p => p.id === entry.id && p.integrity === entry.integrity && p.url === entry.url);
      if (!stored || !/^[a-f0-9]{64}$/.test(stored.sha256)) throw new Error(`missing captured registry package: ${entry.id}`);
      body = readFileSync(join(output, 'tarballs', `${stored.sha256}.tgz`));
      sourceId = stored.sourceId;
      if (verify(body, entry.integrity) !== stored.sha256 || body.length !== stored.size) throw new Error(`captured registry package identity changed: ${entry.id}`);
    }
    const sha256 = verify(body, entry.integrity);
    if (mode === 'acquire') {
      const path = join(output, 'tarballs', `${sha256}.tgz`);
      if (!existsSync(path)) writeFileSync(path, body, {flag:'wx'});
    }
    packages.push({...entry, sha256, size:body.length, sourceId});
  }
  return packages;
}
