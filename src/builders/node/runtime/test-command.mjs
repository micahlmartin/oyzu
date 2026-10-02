// Framework wrappers keep argument arrays intact, including native lifecycle
// scripts. Import npm only when requested; an implicit runner needs only Node.
export async function testCommand(argv) {
  if (argv[0] !== 'npm') return argv;
  const {npmScriptCommand} = await import('./npm-native.mjs');
  return npmScriptCommand(argv.slice(1));
}
