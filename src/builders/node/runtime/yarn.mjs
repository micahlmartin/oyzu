import {pathToFileURL} from 'node:url';
import {run} from './manager-runtime.mjs';
import {inputs, perform} from './yarn-registry.mjs';

export const profile = {
  id:'yarn', lock:'yarn.lock', command:['yarn'],
  validate:inputs, perform,
};
if (process.argv[1] && pathToFileURL(process.argv[1]).href === import.meta.url) await run(profile);
