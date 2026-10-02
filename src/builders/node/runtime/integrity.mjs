// Integrity shared by registry archive consumers; native graph semantics stay owned.
import {createHash, timingSafeEqual} from 'node:crypto';

export function verify(bytes, integrity) {
  if (typeof integrity !== 'string' || !/^sha512-[A-Za-z0-9+/]{86}==$/.test(integrity)) throw new Error('registry archive requires sha512 integrity');
  const expected = Buffer.from(integrity.slice('sha512-'.length), 'base64');
  const actual = createHash('sha512').update(bytes).digest();
  if (expected.length !== actual.length || !timingSafeEqual(expected, actual)) throw new Error('registry tarball integrity mismatch');
  return createHash('sha256').update(bytes).digest('hex');
}
