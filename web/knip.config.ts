import type { KnipConfig } from 'knip'

const config: KnipConfig = {
  // Kept for local debugging; imported (commented out) in src/routes/__root.tsx.
  ignoreDependencies: [
    '@tanstack/react-query-devtools',
    '@tanstack/router-devtools',
  ],
  // API-client DTO types and schema factories are frequently referenced only
  // within their own module; count intra-file use as used so knip flags only
  // truly orphaned exports. The vitest test entry (setup + test files) is
  // auto-detected from the `test` block in vite.config.ts by knip's vitest
  // plugin, which is why msw / @testing-library / user-event resolve.
  ignoreExportsUsedInFile: true,
}

export default config
