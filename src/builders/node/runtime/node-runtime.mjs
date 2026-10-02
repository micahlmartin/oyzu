// Exact runtime admission shared by native package-manager entrypoints.
export function verifyNodeVersion(expected = process.env.OYZU_EXPECT_NODE) {
  if (expected !== undefined && expected !== process.versions.node) {
    throw new Error(`requested Node ${expected} does not match provisioned Node ${process.versions.node}`);
  }
}
