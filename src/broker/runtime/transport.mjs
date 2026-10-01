// Credential-free client for the engine's scoped filesystem acquisition channel.
import {randomUUID} from 'node:crypto';
import {readFile, writeFile, rename, unlink} from 'node:fs/promises';
import {join} from 'node:path';
import {setTimeout as delay} from 'node:timers/promises';

export async function fetch(url, root = '/broker') {
  const id = randomUUID().replaceAll('-', '');
  const pending = join(root, `${id}.pending`);
  await writeFile(pending, JSON.stringify({url}), {flag: 'wx'});
  await rename(pending, join(root, `${id}.request`));
  const response = join(root, `${id}.response`);
  const deadline = performance.now() + 55_000;
  let info;
  for (;;) {
    try { info = JSON.parse(await readFile(response, 'utf8')); break; }
    catch (error) { if (error.code !== 'ENOENT') throw error; }
    if (performance.now() > deadline) throw new Error('scoped acquisition broker did not respond');
    await delay(10);
  }
  const bodyPath = join(root, `${id}.body`);
  const body = await readFile(bodyPath);
  await unlink(response);
  await unlink(bodyPath);
  return {info, body};
}
