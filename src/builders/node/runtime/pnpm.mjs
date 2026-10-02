import {pathToFileURL} from 'node:url';
import {run} from './manager-runtime.mjs';
import {inputs, perform} from './pnpm-registry.mjs';
import {exportStore} from './pnpm-store.mjs';

export const profile = {
  id:'pnpm', lock:'pnpm-lock.yaml',
  command:['pnpm','--config.manage-package-manager-versions=false'],
  validate:inputs,
  perform, exportStore,
};
if (process.argv[1] && pathToFileURL(process.argv[1]).href === import.meta.url) await run(profile);
