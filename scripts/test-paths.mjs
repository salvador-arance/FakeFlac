// What counts as a protected test file (RP-04). Shared by the guard-protected hook (stage
// `implement`) and scripts/guard-tests.mjs (CI), so both protect the same files.
export const isTest = (p) =>
  /(^|\/)tests\//.test(p) ||
  /\.(test|spec)\.(ts|js|mjs|svelte)$/.test(p) ||
  /\.stories\.(ts|svelte)$/.test(p) ||
  p.endsWith('.feature')
