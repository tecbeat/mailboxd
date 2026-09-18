# Changelog

All notable changes to this project will be documented in this file.

## [1.7.3](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.7.3) - 2026-09-18

### ⚙️ Miscellaneous Tasks

- *(deps)* Update Node.js to 760e44b - ([39045fc](https://git.teccave.de/tecbeat/mailboxd/commit/39045fcba5995d10f3de60679c842cde4776f8fa))

## [1.7.2](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.7.2) - 2026-09-12

### 🐛 Bug Fixes

- *(deps)* Update all non-major dependencies - ([0079510](https://git.teccave.de/tecbeat/mailboxd/commit/007951064cbd8aafeff0d7bc40fdaf9480cdfa52))

## [1.7.1](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.7.1) - 2026-09-09

### 🐛 Bug Fixes

- *(deps)* Update node.js to 50c8e8c - ([8a2c2c4](https://git.teccave.de/tecbeat/mailboxd/commit/8a2c2c4a8c9d73569c182cd2af8117d8441052da))

### ⚙️ Miscellaneous Tasks

- *(deps)* Update Node.js to 50c8e8c - ([8e7ab32](https://git.teccave.de/tecbeat/mailboxd/commit/8e7ab3297bc26f877b9fa15e73500b980cc85c1c))

## [1.7.0](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.7.0) - 2026-09-09

### ⛰️  Features

- *(audit)* Add admin-only audit log page to the web UI - ([34280a9](https://git.teccave.de/tecbeat/mailboxd/commit/34280a9645c35ceb06210a465c5f109906ce2864))
- *(audit)* Add persistent audit log for security-relevant events - ([7482109](https://git.teccave.de/tecbeat/mailboxd/commit/748210968c6b90a07452f1d8e7515e2326e382be))
- *(backup)* Add admin backup settings page (Refs #42) - ([cfcf7d2](https://git.teccave.de/tecbeat/mailboxd/commit/cfcf7d2fa5ac298cd1db42a6212dd7f9a466e90d))
- *(backup)* Add admin backup REST API and audit events (Refs #42) - ([0c12619](https://git.teccave.de/tecbeat/mailboxd/commit/0c126196e3f06726cf56fab5b155d0932229524f))
- *(backup)* Add instance backup and staged restore core (Refs #42) - ([8dc7510](https://git.teccave.de/tecbeat/mailboxd/commit/8dc751080c665c85856a33928d506dd130ac3b96))
- *(search)* Ship default attachment text extractor (Refs #4) - ([256fcf9](https://git.teccave.de/tecbeat/mailboxd/commit/256fcf957c5c6e1be369a416f019d18042726e90))

### 🐛 Bug Fixes

- *(build)* Regenerate route tree for router-plugin 1.168.36 - ([46a85f4](https://git.teccave.de/tecbeat/mailboxd/commit/46a85f40746bd716d48b66cb03d3113893341dc0))
- *(ci)* Skip unittest gate jobs on the default branch - ([d073529](https://git.teccave.de/tecbeat/mailboxd/commit/d073529f21d461a629e9deb30fdbdf4190f03f71))
- *(deps)* Pin knip to 6.34.0 to satisfy supply-chain release-age policy - ([b650373](https://git.teccave.de/tecbeat/mailboxd/commit/b650373da73b9d85cd495eefad64c8749452fccc))
- *(deps)* Update all non-major dependencies - ([765f71d](https://git.teccave.de/tecbeat/mailboxd/commit/765f71d949fb33e6b5920a5d579e1b2e2806db1b))
- *(oidc)* Localize SSO strings, document HS256 requirement (Refs #5) - ([b48aaa8](https://git.teccave.de/tecbeat/mailboxd/commit/b48aaa8f2389764c74a40e8fbe86b3382a3e525d))

### 📚 Documentation

- *(backup)* Document backup env vars and add compose bind-mount (Refs #42) - ([fefdf7e](https://git.teccave.de/tecbeat/mailboxd/commit/fefdf7e133885a93c77ccc77059518b5f28ebcb1))

## [1.6.0](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.6.0) - 2026-09-08

### ⛰️  Features

- *(layout)* Shared PageHeader with action slot + consistent nav chrome (#22, #23, #40) - ([7232aab](https://git.teccave.de/tecbeat/mailboxd/commit/7232aab35f39d2c179fede5c78c2aded96b3dfde))
- *(profile)* Inline SSO-managed-credentials note and testable user update (#17) - ([12996e5](https://git.teccave.de/tecbeat/mailboxd/commit/12996e514d64c8a7f45f1b9b0281ac4db5c9edc1))
- *(profile)* Hide password field for SSO users - ([d2e08ae](https://git.teccave.de/tecbeat/mailboxd/commit/d2e08ae30746b4b210c9126c2d6a12b7c6e7365c))
- *(ui)* Shared EmptyState component + unified card surfaces and empty-state heights (#26, #39) - ([a3f2c61](https://git.teccave.de/tecbeat/mailboxd/commit/a3f2c61d034e67cdae1164914680c43650db033e))
- *(web)* Add semantic success color token - ([1faa298](https://git.teccave.de/tecbeat/mailboxd/commit/1faa2980f326b2fe0ba6c70991e5415351d3b895))
- *(web)* Integrate search into the filter toolbar (#24) - ([ec8050a](https://git.teccave.de/tecbeat/mailboxd/commit/ec8050a74981bf4f4a8593a457992f3907a42f06))

### 🐛 Bug Fixes

- *(build)* Force pnpm copy import method in Docker web build - ([c73ce79](https://git.teccave.de/tecbeat/mailboxd/commit/c73ce7986e6e59956afcbdac4c7fe801534c4ba6))
- *(dashboard)* Render loading state inside page chrome, mirror grid (#28) - ([c53d4e0](https://git.teccave.de/tecbeat/mailboxd/commit/c53d4e09ba5edf92731cfed348ce4ebeedd832aa))
- *(data-table)* Move vertical scrollbar outside the bordered box - ([499afd9](https://git.teccave.de/tecbeat/mailboxd/commit/499afd94fe299a69a94a850817732fc9d7a90418))
- *(deps)* Update @tanstack/router-plugin to 1.168.36 - ([039df50](https://git.teccave.de/tecbeat/mailboxd/commit/039df50e85dc2f347021b1ea20bf3f4fbcf24ba3))
- *(deps)* Sync Cargo.lock with zstd 0.14 bump - ([fe49a64](https://git.teccave.de/tecbeat/mailboxd/commit/fe49a645694a3e26202303614800a48b47060392))
- *(deps)* Align corepack pnpm pins to v12.3.4 - ([0690c6b](https://git.teccave.de/tecbeat/mailboxd/commit/0690c6b6cb89498a2a28553647541b6f45f0ddc6))
- *(deps)* Update pnpm to v12 - ([4dd5e33](https://git.teccave.de/tecbeat/mailboxd/commit/4dd5e339538ef54cb7dfbad20fb5635f4c6c2d33))
- *(deps)* Update all non-major dependencies - ([925abdc](https://git.teccave.de/tecbeat/mailboxd/commit/925abdc876a13df679dcafd8ed201573a04e8e1e))
- *(deps)* Update vitest monorepo to v5 - ([cef2a79](https://git.teccave.de/tecbeat/mailboxd/commit/cef2a79a70f807ec8ab10e7ef6f639082e827e18))
- *(format)* Route remaining timestamps through shared date formatters (#31) - ([c33e8a0](https://git.teccave.de/tecbeat/mailboxd/commit/c33e8a001bd58d723fe2a13c3f9f55be7cdd81c1))
- *(layout)* Unify page header bottom margin to mb-4 - ([e7e06eb](https://git.teccave.de/tecbeat/mailboxd/commit/e7e06eb78108f852042646e90659fccbe4dc892c))
- *(layout)* Flush secondary-nav content with primary-nav right edge - ([7fdd33c](https://git.teccave.de/tecbeat/mailboxd/commit/7fdd33ca225f3c9aad7a461bbc193e7584263418))
- *(layout)* Center shared container, keep dashboard full-width, align settings nav (#21) - ([05178b0](https://git.teccave.de/tecbeat/mailboxd/commit/05178b065e6b2d403e8b748399e99020f259ca5f))
- *(mail-list)* Align filter toolbar right edge with table scrollbar gutter - ([e6ad7cb](https://git.teccave.de/tecbeat/mailboxd/commit/e6ad7cbdea2815a3338d826a5ced2c5685b96747))
- *(nav)* Shared SidebarNav shows current section on mobile + correct placeholder key (#18, #41) - ([0e72c1f](https://git.teccave.de/tecbeat/mailboxd/commit/0e72c1f92c7d395c8e7bcede22cb832ada9fa7b3))
- *(search)* Animate table updates instead of reloading the whole page - ([b18c8e0](https://git.teccave.de/tecbeat/mailboxd/commit/b18c8e08afe2383855a72605202235ed51ec9c56))
- *(ui)* Route remaining status badges through shared color helpers (#30) - ([eebbfe2](https://git.teccave.de/tecbeat/mailboxd/commit/eebbfe26489c7d0293b1ac85c204f7ef35b7bd36))
- *(ui)* Unify page-size options across all tables (#32) - ([19fc4ec](https://git.teccave.de/tecbeat/mailboxd/commit/19fc4ecae89844f6178c44a0259d0044641123c5))
- *(web)* Correct hook dependency arrays - ([243feab](https://git.teccave.de/tecbeat/mailboxd/commit/243feab6a32a1a042b5837ef27a004b127f7da9d))
- *(web)* Stabilize sort setters and complete effect deps - ([28938db](https://git.teccave.de/tecbeat/mailboxd/commit/28938dbd8362e8194791b80d87805036462af0ce))
- *(web)* Keep IMAP password input controlled - ([a9a7e6b](https://git.teccave.de/tecbeat/mailboxd/commit/a9a7e6bb876e6672eac70d79663463fab29622bd))
- *(web)* Send JSON body for start-download to avoid 415 - ([4912162](https://git.teccave.de/tecbeat/mailboxd/commit/49121624f6c092691a1382c2a3a431445242ddb8))
- *(web)* Align badge and plain values on one right edge in system config - ([fc31fb7](https://git.teccave.de/tecbeat/mailboxd/commit/fc31fb71d2d97e4c356322e9619e045d08932fe9))
- *(web)* Label bulk-action toolbar icon buttons - ([21b4963](https://git.teccave.de/tecbeat/mailboxd/commit/21b4963dd68847279547c49a9fa1b5e78b4077cc))
- *(web)* Avoid nested interactive element in filter selector fields - ([8b0ba57](https://git.teccave.de/tecbeat/mailboxd/commit/8b0ba573c5c82cb0290086217b1c054789023067))
- *(web)* Add missing search_mailbox i18n keys - ([44fe92e](https://git.teccave.de/tecbeat/mailboxd/commit/44fe92e22095642f2b3bb12e5b0638cf7f3cbf67))
- *(web)* Keep search input focused while live results load - ([3ffcc04](https://git.teccave.de/tecbeat/mailboxd/commit/3ffcc04543467dfd32bba05305a371c750e95b0c))
- *(web)* Use text-destructive for row-action delete items (#29) - ([3cefcaa](https://git.teccave.de/tecbeat/mailboxd/commit/3cefcaa2a098cc5049d11278034e308541312ead))
- *(web)* Replace hardcoded UI strings with i18n keys (#36) - ([32bae0b](https://git.teccave.de/tecbeat/mailboxd/commit/32bae0b87b53aa685a1f85ccbfac778de0518e34))
- *(web)* Persist header theme switch to user profile (#20) - ([3af01ed](https://git.teccave.de/tecbeat/mailboxd/commit/3af01edfaea4bce121b0416123e1687e7764e47d))
- *(web)* Make blue-dark surfaces distinct from background (#19) - ([bb58ce7](https://git.teccave.de/tecbeat/mailboxd/commit/bb58ce770d9a8175b0d2cd72d04df9566d6dbade))
- *(web)* Prevent system configurations rows from overflowing - ([ef50fa0](https://git.teccave.de/tecbeat/mailboxd/commit/ef50fa0b983ba80b0c4e6585655e014d08e9c09d))
- *(web)* Align users section shell with settings for consistent headers - ([867ce5b](https://git.teccave.de/tecbeat/mailboxd/commit/867ce5b2bf293e604776ce05f2ab01a635377d1d))
- *(web)* Tighten dashboard spacing so it fits without page scroll - ([376e28b](https://git.teccave.de/tecbeat/mailboxd/commit/376e28b9b43d61264670a57eb7638526d4f31397))
- *(web)* Use theme tokens in mail views for dark theme support (#27) - ([ddbd887](https://git.teccave.de/tecbeat/mailboxd/commit/ddbd8879c16107130402a781b17f8c4789a54981))
- *(web)* Replace hardcoded palette colors with theme tokens (#29) - ([487e424](https://git.teccave.de/tecbeat/mailboxd/commit/487e424426edffc608e619b4d898b2e00aa962a7))
- *(web)* Right-align form action buttons in profile and appearance (#25) - ([21155e9](https://git.teccave.de/tecbeat/mailboxd/commit/21155e976f48bab26605836a6d12f7218e7fc88d))
- *(web)* Reserve the settings scrollbar gutter to prevent tab shift - ([166336a](https://git.teccave.de/tecbeat/mailboxd/commit/166336a9e6258998e3cc2b7c2275d5551d338e77))
- *(web)* Standardize the system configurations settings page - ([c8d685e](https://git.teccave.de/tecbeat/mailboxd/commit/c8d685e5d08707c940b152749e7d7a88d243c737))
- *(web)* Left-align API docs cards in a 3-column grid - ([7c2feb1](https://git.teccave.de/tecbeat/mailboxd/commit/7c2feb18e047e2c887d16383fa8562d1ec7aba5e))
- *(web)* Use the standard PageHeader on the API documentation page - ([852eef5](https://git.teccave.de/tecbeat/mailboxd/commit/852eef553388409774ea3ae35785f01a95628128))
- *(web)* Hug table content and move the horizontal scrollbar below the border - ([6fb3d3a](https://git.teccave.de/tecbeat/mailboxd/commit/6fb3d3ac25d8a28346c7897a010d2ba189f28d7b))
- *(web)* Highlight the active settings nav item by resolved active href - ([5101409](https://git.teccave.de/tecbeat/mailboxd/commit/5101409877f496d218f7c41b6fc46ca2e3f13c39))
- *(web)* Center page content and widen the max-width cap to 1600px - ([9fe36ac](https://git.teccave.de/tecbeat/mailboxd/commit/9fe36ac391aa4631454f2440749c03b67e164340))
- *(web)* Unify list-page scrolling with pinned toolbar and pagination - ([39d662f](https://git.teccave.de/tecbeat/mailboxd/commit/39d662fab39295551bba0b12ff6e62dc67e3fc00))
- *(web)* Let mail table flow into the shell scroll region (#29) - ([28989cd](https://git.teccave.de/tecbeat/mailboxd/commit/28989cd202956e543ae2a2f3a291824c9b758b36))
- *(web)* Establish single content scroll region in app shell (#21) - ([76dc036](https://git.teccave.de/tecbeat/mailboxd/commit/76dc036d74d345c047c189cce5747a29852a3742))

### 🚜 Refactor

- *(format)* Locale-aware formatDateTime and formatRelativeTime helpers, migrate all call sites (#31) - ([cbc3e37](https://git.teccave.de/tecbeat/mailboxd/commit/cbc3e374a5d441c505a01b8fe08e2032da30afff))
- *(layout)* Unified page container and single padding contract (#21, #38) - ([275bfc8](https://git.teccave.de/tecbeat/mailboxd/commit/275bfc86b0998a772a4f1877d484847e3dd021c5))
- *(ui)* Default EmptyState logo icon and migrate remaining empty state (#26, #39) - ([3559f39](https://git.teccave.de/tecbeat/mailboxd/commit/3559f39122386718d263d5ce92630cc63e874cdc))
- *(ui)* Consolidate table pagination into one component (#32) - ([c1c7842](https://git.teccave.de/tecbeat/mailboxd/commit/c1c784281612b27afe7829ea04a2e24aff9b23c9))
- *(ui)* Shared Spinner and unified loading states (#33) - ([abd6cf5](https://git.teccave.de/tecbeat/mailboxd/commit/abd6cf54d1288b8fe4e08628a6dde01da0f73ad7))
- *(ui)* Single source of truth for status and file-type colors (#30) - ([d2aa47b](https://git.teccave.de/tecbeat/mailboxd/commit/d2aa47b1561c4d05dbc0c18dca5c90a366276a56))
- *(web)* Extract non-component exports from mail-list modules - ([39113cb](https://git.teccave.de/tecbeat/mailboxd/commit/39113cb3a791a8242539e531ffe72cfd9cd8550b))
- *(web)* Split context providers into component-only files - ([f9ad8bb](https://git.teccave.de/tecbeat/mailboxd/commit/f9ad8bb604c14edb5a6fb1e4cbe5475d4792cdc3))
- *(web)* Sync account settings form via RHF values prop - ([df2da84](https://git.teccave.de/tecbeat/mailboxd/commit/df2da841aa59c712cd279c6dad3b1f3b12794a14))
- *(web)* Read form fields via useWatch in dialogs - ([5267989](https://git.teccave.de/tecbeat/mailboxd/commit/5267989eeaddb9947a80669efd514036cd79653b))
- *(web)* Stop rest-spreading query results in hooks - ([94ed0e8](https://git.teccave.de/tecbeat/mailboxd/commit/94ed0e87c03e963142e19d9303df762047fd4a02))
- *(web)* Place the filter reset right after the search field - ([9e14738](https://git.teccave.de/tecbeat/mailboxd/commit/9e14738ae9a789b36a6c6c664259c93e0a226715))
- *(web)* Refine filter bar wrap, font and fields trigger (#24) - ([9164aa7](https://git.teccave.de/tecbeat/mailboxd/commit/9164aa7b849b38903feb11bda53013dfd74d1b8f))
- *(web)* Polish unified filter bar layout and responsiveness (#24) - ([8a48d01](https://git.teccave.de/tecbeat/mailboxd/commit/8a48d016271e1a87023e097c8f32d54de71ba32f))
- *(web)* Unify search and filters into a single toolbar (#24) - ([97c2a4f](https://git.teccave.de/tecbeat/mailboxd/commit/97c2a4f4f1629b76df51a08c51abbf3862672c65))
- *(web)* Standardize dialog sizes, footers and naming (#34) - ([36fec5c](https://git.teccave.de/tecbeat/mailboxd/commit/36fec5c8d5031de145d416892617c2d75d24730b))
- *(web)* Standardize icon usage (#35) - ([9724527](https://git.teccave.de/tecbeat/mailboxd/commit/97245279ccff72005973d731cf94118b08785717))
- *(web)* Unify dashboard card typography (#37) - ([b8dad6c](https://git.teccave.de/tecbeat/mailboxd/commit/b8dad6c1208a733b9bd542d90e8c55f105cc0a52))

### ⚙️ Miscellaneous Tasks

- *(cleanup)* Remove dead code and normalize new-file license headers (#41) - ([65d51ab](https://git.teccave.de/tecbeat/mailboxd/commit/65d51abf3f6d98c7c1d678e681e01590230c8313))
- *(cleanup)* Standardize toast access, fix oauth2 toolbar width, remove dead code (#41) - ([a80ed29](https://git.teccave.de/tecbeat/mailboxd/commit/a80ed29d1af5388a0335fd5a7b37822a4a0dd77f))
- *(git)* Ignore pnpm store and centralize frontend ignores - ([b51c867](https://git.teccave.de/tecbeat/mailboxd/commit/b51c86743581c10ab113eda25d409499c3be5f4d))
- *(web)* Scope react-refresh rule to skip routes and test files - ([3832de9](https://git.teccave.de/tecbeat/mailboxd/commit/3832de96801128af516896546445418439d27dfb))

## [1.5.1](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.5.1) - 2026-09-02

### 🐛 Bug Fixes

- *(deps)* Drop user-event and react-refresh bumps blocked by release-age policy - ([6c103cf](https://git.teccave.de/tecbeat/mailboxd/commit/6c103cff27b0ce0ac1ec6f32873afdb6b4f9b478))
- *(deps)* Update all non-major dependencies - ([606a5da](https://git.teccave.de/tecbeat/mailboxd/commit/606a5da0eabbeb8977628a9ab9ba0263917223b0))

## [1.5.0](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.5.0) - 2026-09-02

### 🐛 Bug Fixes

- *(web)* Resolve all ESLint errors (#16) - ([b4145d8](https://git.teccave.de/tecbeat/mailboxd/commit/b4145d827345ad68f43ba836f606c8a28266f065))
- *(web)* Guard proxy and oauth2 delete dialogs against double submit - ([6749284](https://git.teccave.de/tecbeat/mailboxd/commit/67492848e65db651e5788c9ccda8f422d29c63f5))
- *(web)* Correct stale react-query invalidation keys - ([5d5e4da](https://git.teccave.de/tecbeat/mailboxd/commit/5d5e4da7d95ffc91ada2e0d9cfe2198d00d4ac75))
- *(web)* Refresh both message views when deleting messages - ([13d59d0](https://git.teccave.de/tecbeat/mailboxd/commit/13d59d0dd863f09ca4bba15e262c80cee324d190))
- *(web)* Render delete-mailbox dialog in attachment view - ([76f1098](https://git.teccave.de/tecbeat/mailboxd/commit/76f109810b1b2eb48f69f7113d375930c268152f))
- *(web)* Read current upload limits when queuing import files - ([a5a1ae2](https://git.teccave.de/tecbeat/mailboxd/commit/a5a1ae2bbd0822c904e9314d0e0e462a8d7043c5))
- *(web)* Pin typescript to 6.0.3 to keep eslint compatible - ([f5bc45c](https://git.teccave.de/tecbeat/mailboxd/commit/f5bc45ca39cd143a7189d119b947788ae03ad3eb))

### 🚜 Refactor

- *(api-tokens)* Drop redundant console.error in token dialog - ([f99d005](https://git.teccave.de/tecbeat/mailboxd/commit/f99d005320c52c538fcf8234c7906055cbaaa8a3))
- *(api-tokens)* Consolidate duplicate token UIs into shared module - ([611b7c0](https://git.teccave.de/tecbeat/mailboxd/commit/611b7c0fc3956ebc8d572212be769d384a62c492))
- *(web)* Standardize icons on lucide-react (#12) - ([b9f862c](https://git.teccave.de/tecbeat/mailboxd/commit/b9f862cdf976b49272919285d9c9cb3acea350ef))
- *(web)* Share generic DataTable scaffold across CRUD features (#15) - ([49b8de5](https://git.teccave.de/tecbeat/mailboxd/commit/49b8de5c294e962fcae2622c1e9583faaa9a2f7b))
- *(web)* Remove duplicated route, helpers, and Button primitive (#13) - ([a60edc3](https://git.teccave.de/tecbeat/mailboxd/commit/a60edc3986623e85b2f6c2a8b53c5d5a8c0813e0))
- *(web)* Unify search/attachment tables into shared MailListDataTable - ([c1afe22](https://git.teccave.de/tecbeat/mailboxd/commit/c1afe228c2cbaee311cef0b5c8024f40d054b2d5))
- *(web)* Extract shared more-filters shell and size presets - ([299f9ef](https://git.teccave.de/tecbeat/mailboxd/commit/299f9ef906ae6018729c4bd9c37fb313753c6f02))
- *(web)* Unify mail-display-dialog via the envelope keystone - ([2378b81](https://git.teccave.de/tecbeat/mailboxd/commit/2378b81896aa9a1561760e93bd2a75449f6e9724))
- *(web)* Unify mail-message-view and thread-dialog - ([b663b6b](https://git.teccave.de/tecbeat/mailboxd/commit/b663b6b024912dc9393c93615b0e31bfa9c4d33c))
- *(web)* Move attachment-preview into shared mail-list - ([df594eb](https://git.teccave.de/tecbeat/mailboxd/commit/df594eb3e176e2d0cbe3649e260fe16e0112bf91))
- *(web)* Unify nested-email-dialog on a props contract - ([da0fe99](https://git.teccave.de/tecbeat/mailboxd/commit/da0fe99116bfa02dfc4b3a6acc6fc9f9afe44db2))
- *(web)* Unify account-popover, normalize to richer variant - ([8e5bf44](https://git.teccave.de/tecbeat/mailboxd/commit/8e5bf44f148780a8ab3961a1b71cf76e89f43f7e))
- *(web)* Unify text-search-input via a feature config prop - ([df55e02](https://git.teccave.de/tecbeat/mailboxd/commit/df55e0239b575fe199ca98a672814040ceabea15))
- *(web)* Share restore-message dialog via useCurrentEnvelope - ([9ffe238](https://git.teccave.de/tecbeat/mailboxd/commit/9ffe2380495b2a62d905800596623d4356544111))
- *(web)* Merge behavior-identical mailbox dialog and popover - ([1af45df](https://git.teccave.de/tecbeat/mailboxd/commit/1af45df551e63a783ff5a5e8966709a5f4af09ee))
- *(web)* Share identical list primitives via mail-list config - ([1dc60e2](https://git.teccave.de/tecbeat/mailboxd/commit/1dc60e2ff18c52e4e22bdd9051a3eb9efb2965b0))
- *(web)* Unify search and attachment list contexts - ([3b508aa](https://git.teccave.de/tecbeat/mailboxd/commit/3b508aa14f3f2fd43fbd17c3644afd61660249fe))
- *(web)* Extract shared getFileConfig into mail-list module - ([da60184](https://git.teccave.de/tecbeat/mailboxd/commit/da6018454074ac089fa3f47df38ef09800e14c17))
- *(web)* Extract shared mail-list table primitives - ([8d4f2ed](https://git.teccave.de/tecbeat/mailboxd/commit/8d4f2edfce83a677cce84ad344c74228816feef6))
- *(web)* Extract attachment dialogs into one component - ([f458afd](https://git.teccave.de/tecbeat/mailboxd/commit/f458afd3afc12bf1d01d228da7a28caaf8640d80))
- *(web)* Extract import file validation into pure module - ([20f128b](https://git.teccave.de/tecbeat/mailboxd/commit/20f128ba512e9f900c4e116a98aeb79b9f603432))

### ⚡ Performance

- *(web)* Stop remounting the mailbox tree and memoize selection counts - ([7f0b3e2](https://git.teccave.de/tecbeat/mailboxd/commit/7f0b3e2dbabd825d185178db621fb9fed2f97e15))
- *(web)* Resolve user roles via a map instead of nested scans - ([c389eae](https://git.teccave.de/tecbeat/mailboxd/commit/c389eae801720b2ddf383cb3d90539b4b3ebbffd))

### 🧪 Testing

- *(web)* Characterize shared formatBytes and formatTimestamp (#13) - ([f5a5344](https://git.teccave.de/tecbeat/mailboxd/commit/f5a534486f789d0db2b7fb8ab68fb46ef1ad117f))

### ⚙️ Miscellaneous Tasks

- *(web)* Make eslint a hard gate (#16) - ([d78cc02](https://git.teccave.de/tecbeat/mailboxd/commit/d78cc02d0aef2ba8430ec731aaf46c0b50a860f7))
- *(web)* Remove dead code and unused dependencies (#14) - ([588e78a](https://git.teccave.de/tecbeat/mailboxd/commit/588e78a35d65cc876e0fbb46f93631d9178e3afc))
- *(web)* Use tecbeat-only license header on new files - ([368308d](https://git.teccave.de/tecbeat/mailboxd/commit/368308db702fddad0ec0f3cc0689e79abe0e2824))
- Gate frontend with typecheck, tests and lint - ([a8e7f19](https://git.teccave.de/tecbeat/mailboxd/commit/a8e7f1998184ecc13aba457d03d87e2ae101f92c))

## [1.4.0](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.4.0) - 2026-08-31

### 🐛 Bug Fixes

- *(deps)* Update all non-major dependencies - ([2627328](https://git.teccave.de/tecbeat/mailboxd/commit/2627328af9989238e3229c046669afd41b770e23))
- *(deps)* Update node.js to e67514e - ([b0c1543](https://git.teccave.de/tecbeat/mailboxd/commit/b0c1543f02070d19976b05248fe57607f0d4a656))

### 🚜 Refactor

- *(migrate)* Run storage migration in the server, drop entrypoint script - ([889ea79](https://git.teccave.de/tecbeat/mailboxd/commit/889ea799facff1c4b4483b04119f2ed8d72e2306))

## [1.3.4](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.3.4) - 2026-08-25

## [1.3.3](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.3.3) - 2026-08-25

## [1.3.2](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.3.2) - 2026-08-25

### 🐛 Bug Fixes

- *(deps)* Update rust:1.98.0-slim-bookworm docker digest to 1469a27 - ([8eb936e](https://git.teccave.de/tecbeat/mailboxd/commit/8eb936eac8f9b8c1d26a1d1257d594656299478e))
- *(deps)* Update debian:bookworm-slim docker digest to 8820086 - ([4920779](https://git.teccave.de/tecbeat/mailboxd/commit/492077927624b997699767b0662c2c2f4b4005d8))

### ⚙️ Miscellaneous Tasks

- *(deps)* Update rust:1.98.0-slim-bookworm Docker digest to 1469a27 - ([1a51c7a](https://git.teccave.de/tecbeat/mailboxd/commit/1a51c7a89c0b0bc60e3550fc21ee4e8bf1398c09))

## [1.3.1](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.3.1) - 2026-08-23

### 🐛 Bug Fixes

- *(license)* Attribute tecbeat-authored OIDC files to tecbeat only - ([a7397a6](https://git.teccave.de/tecbeat/mailboxd/commit/a7397a6dcec4dfa0fd4a7759d3c1f0fd6b769247))

## [1.3.0](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.3.0) - 2026-08-22

### 🐛 Bug Fixes

- *(deps)* Update dependency @tanstack/react-table to v9 - ([8007bfc](https://git.teccave.de/tecbeat/mailboxd/commit/8007bfc04adf9aa311c0b9992d885a6379d83168))
- *(deps)* Update dependency eslint to v10.9.0 - ([8403dc3](https://git.teccave.de/tecbeat/mailboxd/commit/8403dc341cbc3c005456c54a0230ed839297dc9e))

### 🚜 Refactor

- *(web)* Migrate data tables to @tanstack/react-table v9 API - ([14b38e3](https://git.teccave.de/tecbeat/mailboxd/commit/14b38e35a6077734d382cfdb7d4f3bd59c0cd408))

### ⚙️ Miscellaneous Tasks

- *(license)* Attribute new data-table module to tecbeat only - ([891ed75](https://git.teccave.de/tecbeat/mailboxd/commit/891ed75c66504144bd2f3729b489fd79505157f4))

## [1.2.1](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.2.1) - 2026-08-21

### 🐛 Bug Fixes

- *(deps)* Update all non-major dependencies - ([713cf7b](https://git.teccave.de/tecbeat/mailboxd/commit/713cf7ba9d6fcc574ba604d76e4939e011d151a3))
- Correct migration-guide wiki URL printed after v1 migration - ([2168650](https://git.teccave.de/tecbeat/mailboxd/commit/21686509d6128f2ca8fe570b9e78711c522bbd60))

## [1.2.0](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.2.0) - 2026-08-21

### 🐛 Bug Fixes

- *(admin)* Auto-migrate legacy storage to v2 on startup - ([43c4cd9](https://git.teccave.de/tecbeat/mailboxd/commit/43c4cd91a10392451cd99cef0f6568ae114278a8))
- *(docker)* Run storage auto-migration before starting server - ([afd58b1](https://git.teccave.de/tecbeat/mailboxd/commit/afd58b19415b951f94de14bf1db1f6c8a668cd75))

### 🚜 Refactor

- *(core)* Extract path-parameterized adopt_bichon_layout - ([56ed66f](https://git.teccave.de/tecbeat/mailboxd/commit/56ed66f44fc709fabac1a255a567ad1cc352918f))

### 📚 Documentation

- Correct v2 storage default paths in config.yml - ([d3c8c5b](https://git.teccave.de/tecbeat/mailboxd/commit/d3c8c5bbb6074e80009e22148755c61ddf6ac984))
- Link the project wiki from the README - ([a8c77f4](https://git.teccave.de/tecbeat/mailboxd/commit/a8c77f4fd1c506c0fc13cccc242a7dba6b5c4905))
- Correct v2 blob storage description and document auto-migration - ([03cacd5](https://git.teccave.de/tecbeat/mailboxd/commit/03cacd5832ff43a1e001d881c8e4312ac85e9350))

### ⚙️ Miscellaneous Tasks

- Add image-based integration and update tests - ([ce38381](https://git.teccave.de/tecbeat/mailboxd/commit/ce38381853453780973d44b87e3ec9c34d5f247b))
- Add dedicated migration test job to pipeline - ([1e5d2ce](https://git.teccave.de/tecbeat/mailboxd/commit/1e5d2cee9c38c2345767c1ba20a5e210d1468c84))

## [1.1.1](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.1.1) - 2026-08-20

### 🐛 Bug Fixes

- *(deps)* Pin rust crate tempfile to =3.27.0 - ([ebd2b81](https://git.teccave.de/tecbeat/mailboxd/commit/ebd2b81e8f2bbdc18680621dace60ceb477f5f74))

## [1.1.0](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.1.0) - 2026-08-20

### ⛰️  Features

- *(blob)* Port upstream redb-backed blob storage engine rewrite - ([ea3d727](https://git.teccave.de/tecbeat/mailboxd/commit/ea3d727124c36cac3fa517384b7e0bb422dc99ba))
- *(import)* Port import module changes from upstream 2.0.1 - ([097413a](https://git.teccave.de/tecbeat/mailboxd/commit/097413a60c21dbff5dae5be6b473a24baf4ab753))
- *(migrate)* Adopt Bichon 2.x data volume on startup - ([98cf03e](https://git.teccave.de/tecbeat/mailboxd/commit/98cf03e4c582abdac73402b14076c98123f4d795))
- *(server)* Port REST API and build script from upstream 2.0.1 - ([7318bea](https://git.teccave.de/tecbeat/mailboxd/commit/7318bea5c877a2ab9280341640d66613a9d3e1aa))
- *(settings)* Add imap_timeout_seconds and archive dir from upstream 2.0.1 - ([2201e6d](https://git.teccave.de/tecbeat/mailboxd/commit/2201e6d7ae31990d286d1af415e94c9954903d45))

### 🐛 Bug Fixes

- *(deps)* Update all non-major dependencies - ([cea4f04](https://git.teccave.de/tecbeat/mailboxd/commit/cea4f048a343ce1edec36ea3e7ba6713c79220a6))

### 🚜 Refactor

- *(admin)* Move migration engine from core to admin crate - ([80acca1](https://git.teccave.de/tecbeat/mailboxd/commit/80acca13f7505b796c2376a9e8e5b1e0ff0a05cb))
- *(core)* Rename cache module to archive and overhaul IMAP sync - ([11d7e9f](https://git.teccave.de/tecbeat/mailboxd/commit/11d7e9f6900c939e56da1ef0c615accebbbc7be5))

### 📚 Documentation

- *(config)* Add Bichon-to-mailboxd migration guide to README FAQ - ([ce6c430](https://git.teccave.de/tecbeat/mailboxd/commit/ce6c4306dd0abe3feca260ef2bca059eea93e27a))

### 🧪 Testing

- Mark network- and SMTP-integration tests as ignored - ([e2a1bec](https://git.teccave.de/tecbeat/mailboxd/commit/e2a1becd35623ef58d83c8bf2d9e8619c3d21caa))
- Fix rand 0.10 API and remove Bichon developer scratchpad tests - ([2cc1195](https://git.teccave.de/tecbeat/mailboxd/commit/2cc1195b8058a0e87754d9f5d24128a66f56b593))

### ⚙️ Miscellaneous Tasks

- *(deps)* Update Cargo.lock for 2.0.1 backend port - ([f919536](https://git.teccave.de/tecbeat/mailboxd/commit/f91953686f6a623b0b77d8d93adacdd22f258d3a))
- *(server)* Gate frontend build on embed-web feature so cargo_test runs without pnpm - ([6a5cd74](https://git.teccave.de/tecbeat/mailboxd/commit/6a5cd748785417331e0c025b0054a9e4567e144e))
- Add cargo_test job to run workspace unit tests on every push - ([d531317](https://git.teccave.de/tecbeat/mailboxd/commit/d5313173e56a2d683016f1dc998ad62fffe914ed))

## [1.0.8](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.0.8) - 2026-08-17

### 🐛 Bug Fixes

- *(deps)* Update rust:1.97.1-slim-bookworm docker digest to 2775a09 - ([e1e99b8](https://git.teccave.de/tecbeat/mailboxd/commit/e1e99b8a629eda6cdbf0cc50aad34e45474f7ff4))

## [1.0.7](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.0.7) - 2026-08-16

### 🐛 Bug Fixes

- *(deps)* Update all non-major dependencies - ([765944b](https://git.teccave.de/tecbeat/mailboxd/commit/765944b898a7a6e56ec545e219d6fcb9d5c8baf7))
- *(deps)* Update rust:1.97.1-slim-bookworm docker digest to 158b745 - ([5fa94b0](https://git.teccave.de/tecbeat/mailboxd/commit/5fa94b0a554c7e7cd1c98cfd5a764c0ea5d77733))

## [1.0.6](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.0.6) - 2026-08-09

### 🐛 Bug Fixes

- *(deps)* Update all non-major dependencies - ([794e918](https://git.teccave.de/tecbeat/mailboxd/commit/794e918a844d450ed26fdb5670f3b62bf40e8e34))

## [1.0.5](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.0.5) - 2026-08-06

### 🐛 Bug Fixes

- *(deps)* Update rust:1.97.1-slim-bookworm docker digest to 96c0af8 - ([ce46535](https://git.teccave.de/tecbeat/mailboxd/commit/ce46535ce9a49f78055b514f5ed8cb53854cf23d))
- *(deps)* Update debian:bookworm-slim docker digest to abd67ff - ([8f23c4a](https://git.teccave.de/tecbeat/mailboxd/commit/8f23c4a85104117c566258c290571ebf31540635))

## [1.0.4](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.0.4) - 2026-08-04

### 🐛 Bug Fixes

- *(deps)* Drop reqwest 0.13, native_model 0.6, and too-new npm bumps - ([11486d9](https://git.teccave.de/tecbeat/mailboxd/commit/11486d90db6b3a9e463827232baf45ff6da04d4b))
- *(deps)* Update all non-major dependencies - ([08052e8](https://git.teccave.de/tecbeat/mailboxd/commit/08052e8d4ef7acdc5df19902abeefa34b94816a4))

### ⚙️ Miscellaneous Tasks

- *(deps)* Regenerate Cargo.lock after renovate consolidation - ([7fc235c](https://git.teccave.de/tecbeat/mailboxd/commit/7fc235ccf163711fce623fe29e73f74a650e74a4))
- *(renovate)* Ignore reqwest >=0.13 and native_model >=0.5 - ([c7040c8](https://git.teccave.de/tecbeat/mailboxd/commit/c7040c82b11baf584e2a5114abd4574873588c0c))

## [1.0.3](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.0.3) - 2026-08-04

### 🐛 Bug Fixes

- *(deps)* Update node.js to d32cdf6 - ([45140b2](https://git.teccave.de/tecbeat/mailboxd/commit/45140b2c923b89c112710214d80fbf20ee08dbc4))

## [1.0.2](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.0.2) - 2026-08-01

### 🐛 Bug Fixes

- *(deps)* Keep native_model at 0.4.20 (native_db 0.8.2 requires it) - ([3ad3d8b](https://git.teccave.de/tecbeat/mailboxd/commit/3ad3d8ba932b20212c23973b4a9fbe57644a4ce0))
- *(deps)* Update dependency typescript to v7 - ([9ebd90f](https://git.teccave.de/tecbeat/mailboxd/commit/9ebd90ff50b7d4c6e7af69714283bb1fa687e6c9))
- *(deps)* Update all non-major dependencies - ([2d429be](https://git.teccave.de/tecbeat/mailboxd/commit/2d429bea18a806eae95a5793bf2a35584b65fe9a))
- *(deps)* Pin dependencies - ([45cf2f5](https://git.teccave.de/tecbeat/mailboxd/commit/45cf2f5b2ad593fffa2578ce99f7d39fb9a30974))
- *(deps)* Pin dependencies - ([ca04d68](https://git.teccave.de/tecbeat/mailboxd/commit/ca04d686697671f90e38eed7727fdbb9d2cbfa01))

### ⚙️ Miscellaneous Tasks

- *(deps)* Regenerate Cargo.lock after renovate consolidation - ([3ba1778](https://git.teccave.de/tecbeat/mailboxd/commit/3ba1778084d1a63237da44c4cafeb2323cf605b2))

## [1.0.1](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.0.1) - 2026-07-31

### 🐛 Bug Fixes

- *(dashboard)* Translate "no attachment data" empty state - ([7971e52](https://git.teccave.de/tecbeat/mailboxd/commit/7971e52371a0c3e0659d59f7a0e4844a8b3c569a))
- *(settings)* Add missing i18n keys for access panel empty state - ([6e17426](https://git.teccave.de/tecbeat/mailboxd/commit/6e17426671bfe8b062c669d578a2a47ba900464b))

### 📚 Documentation

- *(config)* Expand config.yml for full README rendering - ([a249689](https://git.teccave.de/tecbeat/mailboxd/commit/a24968908d5776dd7a43609d07bc717b0066836c))

### ⚙️ Miscellaneous Tasks

- Consolidate renovate updates + categorise config - ([13b62d2](https://git.teccave.de/tecbeat/mailboxd/commit/13b62d297a4f28cc5b2b124d75294780c4328a89))

## [1.0.0](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.0.0) - 2026-07-30

### ⛰️  Features

- *(account)* Prioritize dedicated name field for IMAP authentication username #28 - ([4549503](https://git.teccave.de/tecbeat/mailboxd/commit/454950374f0c6fd4258923131c0f8ce52f028e6a))
- *(admin)* Add interactive data migration tool - ([0abaa66](https://git.teccave.de/tecbeat/mailboxd/commit/0abaa66a408a04316ecf9c97a5ccbf00d0f56da4))
- *(api)* Add envelope endpoint and improve API documentation - ([70db81d](https://git.teccave.de/tecbeat/mailboxd/commit/70db81dc03d96a44a4858f8e30203b663e0e8b11))
- *(bichonctl)* Add support for decoding MIME-encoded X-Gmail-Labels in mbox #182 - ([2b10d20](https://git.teccave.de/tecbeat/mailboxd/commit/2b10d201ee2574770a1440113e168e01081d7b6a))
- *(blob)* Add FilePool read cache and fix BucketCache TOCTOU - ([11e4075](https://git.teccave.de/tecbeat/mailboxd/commit/11e40750fe493d043163b3ccefec1ced4e8de39d))
- *(blob)* Replace JSON metadata with bincode + CRC32 binary format - ([e442db4](https://git.teccave.de/tecbeat/mailboxd/commit/e442db4a2d4c25c18846fd0149760d88ee20d6e6))
- *(blob)* Add embedded KV storage engine for email archiving - ([0474545](https://git.teccave.de/tecbeat/mailboxd/commit/04745458bc0048129c705e5b6a303924a721243c))
- *(cli)* Add interactive email import tool for EML, MBOX, and Thunderbird - ([3f4b37b](https://git.teccave.de/tecbeat/mailboxd/commit/3f4b37be172a6358022d7a79e67aa06b3b92148c))
- *(cli)* Add an option to specify the encrypt password in a file - ([9b83d56](https://git.teccave.de/tecbeat/mailboxd/commit/9b83d5617ebc01632ed52ed84f66f1d0b86bc561))
- *(config)* Add built-in IMAP server config with shared SMTP/IMAP TLS paths - ([c6da79c](https://git.teccave.de/tecbeat/mailboxd/commit/c6da79cdb06c32e4adf0e3cd4818c3950eef288f))
- *(core)* Add OIDC single sign-on client - ([2cad0b2](https://git.teccave.de/tecbeat/mailboxd/commit/2cad0b25a54d7fd00f3fd0fe0d43dea085b9280c))
- *(core)* Wire up attachment text extraction in IMAP sync pipeline - ([f9c2fc7](https://git.teccave.de/tecbeat/mailboxd/commit/f9c2fc77ff0edfe2f3204ef671d187849bb6cdea))
- *(core)* Add ext module with EventBus and AttachmentTextExtractor traits - ([61430b7](https://git.teccave.de/tecbeat/mailboxd/commit/61430b72b0acba1ca36deac5702ae20ac20aa1c7))
- *(cors)* Remove default value for BICHON_CORS_ORIGINS and allow all origins when unset - ([c00ffe8](https://git.teccave.de/tecbeat/mailboxd/commit/c00ffe8d116d2a2369a10aadf4660df8c052ab22))
- *(dashboard)* Display system version and Git hash, link to release tag #19 - ([736270b](https://git.teccave.de/tecbeat/mailboxd/commit/736270b08c7bdf43f013c4c12683122f2f1d1045))
- *(i18n)* Implement internationalization for date distance - ([9ba56ca](https://git.teccave.de/tecbeat/mailboxd/commit/9ba56ca5bb097a337c50d7464e00b3a2a6282331))
- *(i18n)* Add language switcher in top-right corner for multi-language support #9 - ([ea81d02](https://git.teccave.de/tecbeat/mailboxd/commit/ea81d0298baaa1d6dc52ed558c0ae1266e9ec5c0))
- *(imap)* Handle UIDVALIDITY changes via Message-ID comparison instead of full rebuild - ([220aa26](https://git.teccave.de/tecbeat/mailboxd/commit/220aa268c193fc621d4dc652b6685c16f63dc66d))
- *(imap)* Add message size check before download - ([a2a51a2](https://git.teccave.de/tecbeat/mailboxd/commit/a2a51a203791a3c9b620cd6df0345daac16acc25))
- *(import)* Support X-Bichon-Metadata and optimize CLI progress reporting - ([e83a00f](https://git.teccave.de/tecbeat/mailboxd/commit/e83a00fe3955b14916c870b637a75fc8c6f74216))
- *(import)* Introduce /api/v1/import endpoint to support batch EML email import - ([6ce1420](https://git.teccave.de/tecbeat/mailboxd/commit/6ce14207143941a20341256619a625d455cd964b))
- *(mailbox)* Support mailbox cleanup #96 - ([e56fe5e](https://git.teccave.de/tecbeat/mailboxd/commit/e56fe5ebeabfdf71623aa536bd9e15a742425beb))
- *(search)* Filter and sort messages by server-side timestamps - ([b3afc52](https://git.teccave.de/tecbeat/mailboxd/commit/b3afc52a822cba423b4419c09e8ce47e31267ac7))
- *(search)* Add advanced attachment filters for extension, category and mime type - ([c19f397](https://git.teccave.de/tecbeat/mailboxd/commit/c19f3977bab97895d2bd01cd2650e6bfe67bb390))
- *(search)* Expand default search scope and support specific field filtering - ([884fdeb](https://git.teccave.de/tecbeat/mailboxd/commit/884fdeba1034985d3888af4f1215aae4da9f7917))
- *(search)* Integrate mailbox directory tree into search interface - ([af0f47c](https://git.teccave.de/tecbeat/mailboxd/commit/af0f47c0e36a3d3b4ffb6a1afed8a1a4824a1828))
- *(search-ui)* Optimize search UI - ([0d20a96](https://git.teccave.de/tecbeat/mailboxd/commit/0d20a9676abf0c92a089c5e4b70b8f4d131ab255))
- *(server)* Expose OIDC login, callback, handoff and public config - ([db7a319](https://git.teccave.de/tecbeat/mailboxd/commit/db7a319c0704f2b3d8513f63bf22b3c1a0643dad))
- *(settings)* Expose OIDC default_role_id and auto_redirect in system config - ([dae8e43](https://git.teccave.de/tecbeat/mailboxd/commit/dae8e431e2c5f33a4caf7b2131338e15778fdc7b))
- *(smtp)* Implement built-in SMTP server for mail ingestion - ([4b0d571](https://git.teccave.de/tecbeat/mailboxd/commit/4b0d571cf2d05d811d69ec4ac469c0426d8bc5c6))
- *(ui)* Persist account table sorting to localStorage - ([41c3b84](https://git.teccave.de/tecbeat/mailboxd/commit/41c3b84e6578cad5b1b358ec6e2f284e8b454593))
- *(ui)* Sync search filters with URL and add dashboard navigation - ([2228e98](https://git.teccave.de/tecbeat/mailboxd/commit/2228e984105b3c52947bdcfad0b3a5b90ec10511))
- *(ui)* Add clickable logo to redirect to homepage #95 - ([c69ada3](https://git.teccave.de/tecbeat/mailboxd/commit/c69ada32efc7b62c30eb122675c560295f58abe5))
- *(ui)* Add quick page navigation to the email list pagination #85 - ([75cae51](https://git.teccave.de/tecbeat/mailboxd/commit/75cae51be9f8e8170db210902ca7e045f5498463))
- *(ui)* Add i18n support for profile dropdown - ([e8a1569](https://git.teccave.de/tecbeat/mailboxd/commit/e8a15695d8b75babdb02acdbb97ee0ccbae71729))
- *(version)* Use teccave pipeline VERSION for the runtime version string - ([b49b087](https://git.teccave.de/tecbeat/mailboxd/commit/b49b0873c5ee89f2e959a31312954da55efcfa05))
- *(web)* Add OIDC sign-in flow and secure token handoff - ([eee513c](https://git.teccave.de/tecbeat/mailboxd/commit/eee513c94c28eb32256080d9772b0fa1406f0533))
- *(web)* Replace Bichon logo with tecbeat mailboxd icon - ([0e69821](https://git.teccave.de/tecbeat/mailboxd/commit/0e698219ce01977207c3b51ce86ae0c2cfe1c5ea))
- Add tag display and editing to email view - ([166ac21](https://git.teccave.de/tecbeat/mailboxd/commit/166ac215497a54648a69a5bee6d6fb2a360ff126))
- Improve account setup UI and add post-download email filtering - ([2da4213](https://git.teccave.de/tecbeat/mailboxd/commit/2da42134d026c198c39dc6b400f32f1c3533c0fa))
- Add searchable account selector with alphabetical sort in user dialog #289 - ([42ce36a](https://git.teccave.de/tecbeat/mailboxd/commit/42ce36a43983321f7e94c796e1c54d294839d224))
- Web upload supports PST, configurable MBOX/PST size limits - ([40ae2d4](https://git.teccave.de/tecbeat/mailboxd/commit/40ae2d49d4a19bf318f5b322b7f9c7894d7e895e))
- Fullscreen attachment gallery with image navigation - ([0d37825](https://git.teccave.de/tecbeat/mailboxd/commit/0d37825625244d6d05e8e1c9e006b58ae7248167))
- Add web upload for EML/MBOX files #260 - ([cdf27f2](https://git.teccave.de/tecbeat/mailboxd/commit/cdf27f2dd437f09a862fc985a0431a1311493e0c))
- Add in-browser attachment preview for images, PDFs, and text files #303 - ([db36272](https://git.teccave.de/tecbeat/mailboxd/commit/db36272bae1303a6b570247bf9a792a263138131))
- Search for an email address simultaneously in to, cc and bcc  #304 - ([cad4472](https://git.teccave.de/tecbeat/mailboxd/commit/cad447275f1494ad856bd2691b70d46737cc1149))
- Display Name / Alias for IMAP Accounts  #306 - ([8542ba0](https://git.teccave.de/tecbeat/mailboxd/commit/8542ba0d2881534379989dbfbcd5c7a3d0bd74c7))
- Add per-account FilterRule, ExtractionRules and ArchiveRules with regex validation - ([bba1ea5](https://git.teccave.de/tecbeat/mailboxd/commit/bba1ea5cc7b8f1bdd8e23eec6940b1df03571b9d))
- Add SSO/OIDC support to user model and settings - ([9e55026](https://git.teccave.de/tecbeat/mailboxd/commit/9e55026f12fd9acb52c7cbff4cca2eac5ac2f432))
- Remove folder limit - ([f30cd66](https://git.teccave.de/tecbeat/mailboxd/commit/f30cd66e00b62779d9dd7869b037885c80deb37a))
- Enhance autoconfig detection - ([c0a63a1](https://git.teccave.de/tecbeat/mailboxd/commit/c0a63a1e3cfff5caeb85b2ec2bd9c637c28d98ee))
- Added Cron scheduling for email downloads #211 - ([005b1c2](https://git.teccave.de/tecbeat/mailboxd/commit/005b1c21161c45bb85b627420220cc65a8e636ec))
- Strip remote data from emails when viewed #54 - ([a3cdc09](https://git.teccave.de/tecbeat/mailboxd/commit/a3cdc094e8fba1083f9887220de20a8a4ffdda3f))
- Add async index deduplication task - ([85d5490](https://git.teccave.de/tecbeat/mailboxd/commit/85d549083402675cb8ecba9ef84018401b7acd71))
- Cache mailbox list for 10 minutes and show progress on initial fetch - ([6b6f11e](https://git.teccave.de/tecbeat/mailboxd/commit/6b6f11e4c10bb278bc2b69d91b076be59f174069))
- Add multiple color themes to appearance settings - ([c07e394](https://git.teccave.de/tecbeat/mailboxd/commit/c07e39495a40c9736da24a67d6ac350be3c14dfa))
- Detect legacy tantivy data layout and abort startup with migration hint - ([66b5959](https://git.teccave.de/tecbeat/mailboxd/commit/66b595908cd38a0921ab580e2efd7ae73656a29d))
- Add manual download and cancel download for email accounts - ([174d56e](https://git.teccave.de/tecbeat/mailboxd/commit/174d56e7b453d44a4c48e5d3c8a82e146094fc5f))
- Use stemmer for multilingual token matching - ([f583c34](https://git.teccave.de/tecbeat/mailboxd/commit/f583c3413c2c0482c331b564a3bf59bc8370a81a))
- Support export account emails to a single mbox file - ([fa70437](https://git.teccave.de/tecbeat/mailboxd/commit/fa70437a62dfb7d3c01e47b51191207eb453f5de))
- Nested eml quick view - ([5b88412](https://git.teccave.de/tecbeat/mailboxd/commit/5b884125f7b2974f7d9314491dc30c614456e73f))
- Add attachment search view - ([7dd722f](https://git.teccave.de/tecbeat/mailboxd/commit/7dd722f9b85ea9064f1f2b2703f9643cd75277bf))
- Restructure IMAP mail download state and ui - ([82915e7](https://git.teccave.de/tecbeat/mailboxd/commit/82915e76ba99c56fb88df00d19359b44994c473d))
- Use fjall to store detached emails and attachments - ([bd10e15](https://git.teccave.de/tecbeat/mailboxd/commit/bd10e15c650179fd56da69b0074ffaecb3a67db1))
- Ability to add/remove tags from any list of messages #189 - ([3f11c5d](https://git.teccave.de/tecbeat/mailboxd/commit/3f11c5dbbfdab1c2b28ccd00b3cc42c51b4ab87a))
- Support nested EML attachment preview and download #150 - ([a8b3b24](https://git.teccave.de/tecbeat/mailboxd/commit/a8b3b24d59b73d03adfa9f13d3e9d1c5d60383f5))
- Allow to host under subpath #145 - ([40eca89](https://git.teccave.de/tecbeat/mailboxd/commit/40eca89a758673a82ad21789a99b04d5557d2747))
- Saving user's page size choices #171 - ([396383a](https://git.teccave.de/tecbeat/mailboxd/commit/396383aa970e22556ec1a601cf5084b9af0ac8bb))
- Set frontend request timeout to 1 minute - ([a440479](https://git.teccave.de/tecbeat/mailboxd/commit/a4404799461a9d397fe0e74134b5bbb7cc895c02))
- Limit concurrent mailbox downloads to 5 per account - ([ecb81ac](https://git.teccave.de/tecbeat/mailboxd/commit/ecb81ac34485a44379584dfee10519c054f92824))
- Reset the login password #126 - ([694e5ec](https://git.teccave.de/tecbeat/mailboxd/commit/694e5ecbec9db8cd4331550bd081d5147d002ace))
- Separate Docker config file location and email data storage location #81 - ([7240be8](https://git.teccave.de/tecbeat/mailboxd/commit/7240be8c313918a73cc51c927fa31fe5829458ea))
- Add support for Outlook PST file import #105 - ([3b040d0](https://git.teccave.de/tecbeat/mailboxd/commit/3b040d0cd6e2d97380d035810d4e02c324c6e380))
- Change default bind address to :: for dual‑stack support - ([bc3eba5](https://git.teccave.de/tecbeat/mailboxd/commit/bc3eba5bf7f4c8f55393fefd2adffd6a3d093d1d))
- Add IPv6 support for bichon_bind_ip configuration - ([4dc99b4](https://git.teccave.de/tecbeat/mailboxd/commit/4dc99b4a84f80f9067b05e0a03ec79ea3854e176))
- Support restoring single message to IMAP #77 - ([a6216c2](https://git.teccave.de/tecbeat/mailboxd/commit/a6216c2ce6fa53626a5d08ff04568e9663d8eb8a))
- Use email 'Date' header for statistics and search filtering #87 - ([7fb6575](https://git.teccave.de/tecbeat/mailboxd/commit/7fb6575f8d33018554ec3df8abbe7e47baf83d75))
- Support user appearance preferences with persisted theme and language #85 - ([455e6b1](https://git.teccave.de/tecbeat/mailboxd/commit/455e6b1a75aa28026080909b964e70db2163d68e))
- Increase password max to 256, fix i18n, and force re-login  #83 - ([62d956c](https://git.teccave.de/tecbeat/mailboxd/commit/62d956c7d6e65000d73b66cb0c9ec20d6664d8cd))
- Replace min/max byte inputs with size preset selection #39 - ([06a1264](https://git.teccave.de/tecbeat/mailboxd/commit/06a126461b596564dddc88a6c6d5ca484a8c3301))
- Search results display the account email and mailbox name. #39 - ([a02bb65](https://git.teccave.de/tecbeat/mailboxd/commit/a02bb65ca0d07f0c88c37ce784cfc695235ea163))
- Add sync_batch_size to allow users to customize the synchronization batch size, and introduce date_before to support semantics such as downloading emails from more than one year ago. #24 #58 - ([1f57f37](https://git.teccave.de/tecbeat/mailboxd/commit/1f57f372d329a15c9c76b3dc90d497511f14f52b))
- Use password file as primary source if provided - ([0f3ad83](https://git.teccave.de/tecbeat/mailboxd/commit/0f3ad830045959ad856ce0d91bc9d09c38cc5e07))
- Add multi-user support and role-based access control #31 - ([4af5176](https://git.teccave.de/tecbeat/mailboxd/commit/4af5176b65d769d6efb12f5761c9f890855c87ff))
- Add IMAP connection pool status logging and disable bb8 idle timeout - ([20970b4](https://git.teccave.de/tecbeat/mailboxd/commit/20970b4fb6325f5135e01e23ac2913e68a2241cb))
- Add option to trust any TLS certificate for IMAP connections - ([7c7e353](https://git.teccave.de/tecbeat/mailboxd/commit/7c7e3531145ddce4844713a934072f5b3b6cc8b0))
- Add folder sync selection and All Mail exclusion logic - ([1042fad](https://git.teccave.de/tecbeat/mailboxd/commit/1042fad6d04ee60aa9c9e5b6fdbae1d82463099f))

### 🐛 Bug Fixes

- *(account)* Update sync range and handle all-mode reset - ([ef891b2](https://git.teccave.de/tecbeat/mailboxd/commit/ef891b20c37bf20ef61029cbf9f076d2a8039343))
- *(account)* Resolve name clearing issue and update field labels - ([8f7244c](https://git.teccave.de/tecbeat/mailboxd/commit/8f7244ccb92c097e33aee6b724bef081aff02dd0))
- *(account)* Update "disabled" semantics - ([089b688](https://git.teccave.de/tecbeat/mailboxd/commit/089b6885a7889cfba05478560987904bc604cd08))
- *(account)* Prevent IMAP password from being overwritten when editing account #7 - ([5ebc739](https://git.teccave.de/tecbeat/mailboxd/commit/5ebc7394d0af988b6d854608522aca52d4dc7d9e))
- *(api)* Use poem_openapi::param::Path for OpenAPI documentation - ([6b18d73](https://git.teccave.de/tecbeat/mailboxd/commit/6b18d7371d1969d5616d26f1d76694eb06eb8656))
- *(api)* [**breaking**] Rename query parameter `id` to `message_id` for clarity - ([934e81c](https://git.teccave.de/tecbeat/mailboxd/commit/934e81c5f9fa06fd1a9a6374c5c056fcf4c77799))
- *(bichon-admin)* Reduce memory usage during data migration - ([b2a7564](https://git.teccave.de/tecbeat/mailboxd/commit/b2a75643da8c5c0f6c09ca8407c9d9d279ab8a99))
- *(blob)* Fsync meta before rename and hold write_mutex during GC - ([ff59d47](https://git.teccave.de/tecbeat/mailboxd/commit/ff59d47a0d1d107d004ae3e80f99504d0f5d022a))
- *(blob)* Invalidate FilePool after GC to prevent stale reads - ([cd98c05](https://git.teccave.de/tecbeat/mailboxd/commit/cd98c050b62a4f2e901d47ec040a1f08818eeca2))
- *(blob)* Address review issues - visibility, unused param - ([b037451](https://git.teccave.de/tecbeat/mailboxd/commit/b0374517e8f5d036240485d0ac7732e4712bc30c))
- *(blob)* Address code review issues - CRC guard, tests, clone - ([495c91b](https://git.teccave.de/tecbeat/mailboxd/commit/495c91b7ae793cf12a7a228fb72da908cc15743a))
- *(core)* Use email schema field for attachment hash lookup in cleanup_unused_content - ([36f3f19](https://git.teccave.de/tecbeat/mailboxd/commit/36f3f19cdc6feb86e85b66eca7f474eb93c58ca4))
- *(core)* Commit/reload barrier before dedup GC reference count - ([895ea54](https://git.teccave.de/tecbeat/mailboxd/commit/895ea543a96b9451c942e6fef8197c0e875fa67c))
- *(docker)* Export MAILBOXD_VERSION so every crate embeds the correct version - ([bc89eac](https://git.teccave.de/tecbeat/mailboxd/commit/bc89eac33f32866de9f8912d65cb9f5fe653abf5))
- *(docker)* Disable strict-dep-builds in pnpm install - ([92e7422](https://git.teccave.de/tecbeat/mailboxd/commit/92e7422cd7a5055a2e5b7aaa43deccedd0e723fb))
- *(docker)* Copy web/pnpm-workspace.yaml into web-builder stage - ([0246452](https://git.teccave.de/tecbeat/mailboxd/commit/0246452351c32d4e108229613ea021d6c7a7be81))
- *(i18n, dashboard)* Internationalize recent activity chart dates - ([0fc83b6](https://git.teccave.de/tecbeat/mailboxd/commit/0fc83b693e713ac790e310041b83bbe1e74282e0))
- *(search)* Expose new SortBy variants via #[oai(rename)] - ([6873841](https://git.teccave.de/tecbeat/mailboxd/commit/6873841ba4c20094087050eb2ac6a77cfc4fa40f))
- *(server)* Correct ascii banner (m a i l b o x d) - ([99a1636](https://git.teccave.de/tecbeat/mailboxd/commit/99a1636d057a7fa1d9c08fb548b9d288bcfcb868))
- *(smtp)* Reject journaling attempts to non-local accounts - ([2f5de48](https://git.teccave.de/tecbeat/mailboxd/commit/2f5de48c6a6412ab75345a0f7ac26da4d7e6caca))
- *(smtp)* Don't clobber the IMAP-owned INBOX uid_validity on journal ingest - ([ce3f894](https://git.teccave.de/tecbeat/mailboxd/commit/ce3f8944a34db4fa45bcf77fac510633a4b4347e))
- *(sync)* Error in account sync task "TooNarrow" #38 - ([75d859a](https://git.teccave.de/tecbeat/mailboxd/commit/75d859abdf5e4f4444466011873e0c1590d64b34))
- *(sync)* Missing initial sync start time after enabling a previously disabled account #32 - ([4b13bf1](https://git.teccave.de/tecbeat/mailboxd/commit/4b13bf14c40e762979808421a33c13b12e1c7bd7))
- *(ui)* Fix sync folder selection jump issue; add auto-select children/parents and expand/collapse all folders button #21 - ([82397ab](https://git.teccave.de/tecbeat/mailboxd/commit/82397ab0cd75d30b48c74b83c61f2a75ed8cd3e1))
- *(ui)* Handle IMAP connection failure gracefully during folder sync #23 - ([1cfc123](https://git.teccave.de/tecbeat/mailboxd/commit/1cfc12324fc6895c87c8d6b4bdfa3516c70f69e6))
- *(ui)* Mailbox email details sheet zoom and close actions not working #5 - ([1f77ed9](https://git.teccave.de/tecbeat/mailboxd/commit/1f77ed91a74592a575898958a45345c491ea7746))
- *(ui)* Add a scrollbar when selecting a mailbox, as the mailbox list can be very long. #4 - ([ad02c76](https://git.teccave.de/tecbeat/mailboxd/commit/ad02c768495454fc227ee00b481d0e045a6391ea))
- *(ui, config)* Ensure use_dangerous status is visible in ui - ([2f3acdd](https://git.teccave.de/tecbeat/mailboxd/commit/2f3acdd75966db03190341562fc68579e6196b73))
- *(web)* Avoid doubled 'v' prefix in dashboard version footer - ([6e1e185](https://git.teccave.de/tecbeat/mailboxd/commit/6e1e185bbb82cde49b87fb33131614f28a7330d5))
- *(web)* Pin pnpm to 11.4.0 via packageManager and corepack prepare - ([4d81311](https://git.teccave.de/tecbeat/mailboxd/commit/4d8131158e23b2bb648bc23359cdd222a2274e53))
- *(web)* Use undefined for empty use_proxy in emptyImap default - ([6f071ee](https://git.teccave.de/tecbeat/mailboxd/commit/6f071ee751d2a901983b6569f2a29cc952b61e25))
- Replace remaining visible Bichon and RustMailer strings - ([4ab028a](https://git.teccave.de/tecbeat/mailboxd/commit/4ab028a67a5cd589dd5190448d7ece386ca283cd))
- Can't set proxy for email account (IMAP)  #326 - ([664ac2f](https://git.teccave.de/tecbeat/mailboxd/commit/664ac2fe5576ea8fd3ef27df99391e0742cd07e3))
- Detect previewable attachments by file extension when MIME type is octet-stream #309 - ([5034760](https://git.teccave.de/tecbeat/mailboxd/commit/5034760517fc74f11768a6666c334f165632fa31))
- Fix：Truncate "To" column recipients in Search tab to prevent excessive row height  #308 - ([875eb2e](https://git.teccave.de/tecbeat/mailboxd/commit/875eb2e309cb344c07b20493c2b3f18654e16be7))
- Sliding token expiry and silent 401 redirect - ([cfb8172](https://git.teccave.de/tecbeat/mailboxd/commit/cfb8172607fd216aa3b27a62809ddf5665c97952))
- Proxy does not support formats from proxy providers  #307 - ([8641bb4](https://git.teccave.de/tecbeat/mailboxd/commit/8641bb4b5651224e71ff4862d11b5b5e5d12c96f))
- Purge DedupCache entries on account/mailbox/envelope removal - ([e3fd9d2](https://git.teccave.de/tecbeat/mailboxd/commit/e3fd9d2f29b59cc8312f41d01b5b80e30e857adc))
- Reconnect and retry IMAP batch on BrokenPipe/network errors - ([c3a7257](https://git.teccave.de/tecbeat/mailboxd/commit/c3a725770cbd723d0b89bbf207d498207d00da11))
- Account deletion times out  #291 - ([c736aff](https://git.teccave.de/tecbeat/mailboxd/commit/c736afffb03949c9d7ceeda0db5ac2189de580a9))
- Preserve non-stored search fields when updating envelope tags - ([aebb94e](https://git.teccave.de/tecbeat/mailboxd/commit/aebb94ee4ebbd12a001d98bc97087c8176f4e76d))
- Open NewIndexWriter once across all migration segments - ([427f724](https://git.teccave.de/tecbeat/mailboxd/commit/427f7248d2005416102566f439e924b93efdbcbc))
- Add in-memory dedup cache to prevent duplicate emails before indexing - ([257736a](https://git.teccave.de/tecbeat/mailboxd/commit/257736a47bfa10ecbbfb47091170d26809ed4039))
- Add fallback UIDVALIDITY support for non-compliant IMAP servers - ([e8469da](https://git.teccave.de/tecbeat/mailboxd/commit/e8469da3bc15d7040ae8c51ca2ee8a9cac78899a))
- "No body available" #262 - ([1346dd2](https://git.teccave.de/tecbeat/mailboxd/commit/1346dd216a7392278ad46750e7544db9a38623af))
- Inline attachment detection and account-scoped export - ([d40ba90](https://git.teccave.de/tecbeat/mailboxd/commit/d40ba90b54ddbcf3b8e24079ea365f614dc6a608))
- Cant migrate with version >= 1.4.0  #277 - ([048d5f3](https://git.teccave.de/tecbeat/mailboxd/commit/048d5f361c3827ebba754c01394778db8180e481))
- HTTP Error 500 Internal Server Error: Failed for 58344335-2e86-4009-979d-6da0331bff63 - Failed to export an email. Aborting process...  #275 - ([4a3c42c](https://git.teccave.de/tecbeat/mailboxd/commit/4a3c42c1ebdc2de8cfb3389e001ad140e53de0bf))
- Use valid IMAP UID SEARCH instead of BEFORE in UID FETCH for incremental sync - ([8e46c7a](https://git.teccave.de/tecbeat/mailboxd/commit/8e46c7a162951a32a1bec621373412ad64b74e62))
- Self-heal missing content blob in download-message - ([171a40d](https://git.teccave.de/tecbeat/mailboxd/commit/171a40d70fff504562b8dfe4cde2841101ff8162))
- Transparent menu on iPhone #253 - ([b22811f](https://git.teccave.de/tecbeat/mailboxd/commit/b22811f78c0977e9b1ff907406f4add7078cace1))
- Imported emails and UTF-8 folders missing #182 - ([fd61d01](https://git.teccave.de/tecbeat/mailboxd/commit/fd61d013a22728cea6a58d129914cdb213c2d0a5))
- Account name don't change when Update Account #248 - ([3a950e7](https://git.teccave.de/tecbeat/mailboxd/commit/3a950e75913357ce2d039e8ff556dcd4bdc70e04))
- Add missing attachment index cleanup logic - ([f17820b](https://git.teccave.de/tecbeat/mailboxd/commit/f17820bfa876958ae6bc476b7f6ba9dfb391f006))
- Fix：After deleting an email, its attachment remains visible/active in the application #245 - ([178b25d](https://git.teccave.de/tecbeat/mailboxd/commit/178b25d27ddd44b18119b066607c8db197517f33))
- Migration link doesn't exist #244 - ([d160ca7](https://git.teccave.de/tecbeat/mailboxd/commit/d160ca75f521df70c405e5ab2575f8ba0a00ad8c))
- Fix small typo in store.rs - ([79b9f07](https://git.teccave.de/tecbeat/mailboxd/commit/79b9f0788898eacec9b182dfc9e9340776c62dc2))
- Migration to v1.0 panics with index out of bounds: the len is 0 but the index is 0 #234 - ([ff64b66](https://git.teccave.de/tecbeat/mailboxd/commit/ff64b66f79f850cc949c7fc01f14beb4b47720a8))
- Bichon-cli OOMs on import #233 - ([66fd50b](https://git.teccave.de/tecbeat/mailboxd/commit/66fd50bc23043e6c07a30cb2970a37a47b3cdc81))
- Can't select folders &  scroll issue in Choose Mailboxes #222 #217 - ([469d254](https://git.teccave.de/tecbeat/mailboxd/commit/469d254e2b845eb8a531411f3d1edaa26dfb1497))
- Overviews are breaking out of their boxes on the dashboard (v1.0.0) #218 - ([d543508](https://git.teccave.de/tecbeat/mailboxd/commit/d543508a237cef29193cff6fffebbc183da296f9))
- Rename bichonctl to bichon-cli - ([1ee2ead](https://git.teccave.de/tecbeat/mailboxd/commit/1ee2eade3a624c754e51b2e9f38e708c71985570))
- Inconsistent permissions for /oauth2: Access restricted to Global Manager only #196 - ([d90943b](https://git.teccave.de/tecbeat/mailboxd/commit/d90943bbfe254c8a83c77d8a648b4de77e3fc8b1))
- Prevent deletion of roles that are currently in use #194 - ([18a0d52](https://git.teccave.de/tecbeat/mailboxd/commit/18a0d52c57e49cc11e440a51afc85ebb96f7dfd0))
- Make email/login_name immutable and add ui sortable account_name #195 - ([39d8168](https://git.teccave.de/tecbeat/mailboxd/commit/39d8168de5dbd02463bd291877b04717cfae9358))
- Set journal_compression to None - ([61161b5](https://git.teccave.de/tecbeat/mailboxd/commit/61161b5f3b3b2851c1cc952173d80f7d3f732db4))
- Bichonctl Thunderbird upload crashes #178 - ([638a93f](https://git.teccave.de/tecbeat/mailboxd/commit/638a93f184f7d5a2370db095a4adbf2de55d195c))
- Add placeholders for dashboard data to prevent 500 errors - ([f3c46f9](https://git.teccave.de/tecbeat/mailboxd/commit/f3c46f97b9308dc0f0ec9e3f7a5328409cbbd024))
- Fix eml ID conversion issue - ([a273b7f](https://git.teccave.de/tecbeat/mailboxd/commit/a273b7f5e1e827bf55c8607897149f0d4ff28e16))
- Memory usage keeps growing #167 - ([5e3d0f1](https://git.teccave.de/tecbeat/mailboxd/commit/5e3d0f1c06ab1fa14c4473bf3f039eea757c552d))
- Fix delete emails - ([a9a9b4a](https://git.teccave.de/tecbeat/mailboxd/commit/a9a9b4a85f916c2645959523c6cb469691db3d1c))
- Sync settings modal doesn't fit on smaller viewport #168 - ([fd35f4b](https://git.teccave.de/tecbeat/mailboxd/commit/fd35f4be8e799437ca79dd7e14c9edb07dc1ef39))
- Search before date picker: go back to selected date #148 - ([ef4ab34](https://git.teccave.de/tecbeat/mailboxd/commit/ef4ab3496e23ea2e53a86084e1369110df8f8ffd))
- Make pst recipient_table optional - ([57afa30](https://git.teccave.de/tecbeat/mailboxd/commit/57afa30b5b42e590a8fc5e7f8aac76b86f064aa2))
- Treat ID command as best-effort and ignore failures - ([673e593](https://git.teccave.de/tecbeat/mailboxd/commit/673e593c4f7bd44b4c9a2afe9c63c67cac24df8f))
- Batch size validation - ([d63b1e0](https://git.teccave.de/tecbeat/mailboxd/commit/d63b1e0d7cb01a45d59c514309e96c733b0133d4))
- Add tolerant HTML-to-text extraction (#141) - ([fed28c3](https://git.teccave.de/tecbeat/mailboxd/commit/fed28c3ecadffdfc5be38895872590b6b0cfdb20))
- Switch from PUID/PGID env vars to Docker --user for permissions - ([01dba4f](https://git.teccave.de/tecbeat/mailboxd/commit/01dba4f71b6b0d3476e44aa970c67abf54bb833b))
- Dashboard fails with error 500 #80 - ([e29e8d7](https://git.teccave.de/tecbeat/mailboxd/commit/e29e8d76b2384a53f2810ae10b9e717f202da546))
- PUID is taken in default ubuntu base image #132 - ([0fb79c9](https://git.teccave.de/tecbeat/mailboxd/commit/0fb79c9a8bbaf38646653f8b4416947b1dbbf1f1))
- Group add issue #131 - ([451b533](https://git.teccave.de/tecbeat/mailboxd/commit/451b5338f1d5013846f51970a3d2420a0768871f))
- Inbox closed when it is already openend #122 - ([feedb91](https://git.teccave.de/tecbeat/mailboxd/commit/feedb912255b8f289981e2401dcb3a1ab287f4b7))
- Account detail modal doesn't fit in viewport #123 - ([57a6e3c](https://git.teccave.de/tecbeat/mailboxd/commit/57a6e3c62e03febfdcf27375bd18bf8cfd12ccd8))
- Allow setting of PUID and PGID to prevent permission issues when using NFS mounts or shared volumes - ([f7fe1f3](https://git.teccave.de/tecbeat/mailboxd/commit/f7fe1f30725dccf943fef75684da8b387167f0e3))
- Storage dir creation logic and permissions issues ( #120, #121) - ([fcd19b1](https://git.teccave.de/tecbeat/mailboxd/commit/fcd19b1c9faae155c7ce2665320ec9730a56d066))
- "unknown" sender when importing PST #117 - ([9b49005](https://git.teccave.de/tecbeat/mailboxd/commit/9b490055220b4cb97b7cf8ccd74d7e0884538fc6))
- Skip invalid MBOX files during import - ([5d3c319](https://git.teccave.de/tecbeat/mailboxd/commit/5d3c319a67aa115921770e6a75941e1f396633ba))
- Missing permission 'user:manage' #102 - ([54a0a71](https://git.teccave.de/tecbeat/mailboxd/commit/54a0a71c44cb0cb072595d691b10e2a5153c26e6))
- Inline attachments are not counted as attachments and are not shown when searching for emails with attachments. - ([0c46432](https://git.teccave.de/tecbeat/mailboxd/commit/0c4643215035bcd5fe1e7694640aa3d36e3cb46d))
- Large empty space at the bottom of the screen #98 - ([1768c1a](https://git.teccave.de/tecbeat/mailboxd/commit/1768c1a590ecab1ccfc28344b6e95c1656fb919c))
- Folder limit cannot be empty #97 - ([55e9751](https://git.teccave.de/tecbeat/mailboxd/commit/55e97510c4da5e7a700fc02969528ffd95009443))
- #94 - ([09375ee](https://git.teccave.de/tecbeat/mailboxd/commit/09375ee11c1a1f02b9911882f2d831fcc530ab49))
- Skip default admin role validation when global_roles is None #93 - ([b2e43b0](https://git.teccave.de/tecbeat/mailboxd/commit/b2e43b0907f55ca65fcf48079d7a29815d293698))
- Modifying the admin user - ([ae91657](https://git.teccave.de/tecbeat/mailboxd/commit/ae916574de6596c39ca1ac277b3fd33d7afd75b0))
- Stitch adjacent RFC2047 words to prevent byte-split artifacts #79 - ([16578fb](https://git.teccave.de/tecbeat/mailboxd/commit/16578fb8e29548da3905f9b770ecbcffa1a89f44))
- Ensure unselected checkboxes are visible in dark mode #70 - ([1e2f526](https://git.teccave.de/tecbeat/mailboxd/commit/1e2f526a0783a2bcbe381db76e055fc472ea31da))
- Fix FAQ link in README.md - ([c90a2d5](https://git.teccave.de/tecbeat/mailboxd/commit/c90a2d552d2eaca1d09c639394f2f4e04969677d))
- Send IMAP ID command after successful authentication to ensure compatibility with 163 mail servers #25 - ([3a2b42f](https://git.teccave.de/tecbeat/mailboxd/commit/3a2b42f5c383239406da699ed48e3cb0daafa55a))

### 🚜 Refactor

- *(blob)* Add delete_batch, gc_if_needed, background flush, and fix bincode compat - ([4cdf3ee](https://git.teccave.de/tecbeat/mailboxd/commit/4cdf3ee5f1c2b49bea2e6e0c62b1627cb0bed637))
- *(blob)* Add NFS-safe file I/O layer - ([b0f2296](https://git.teccave.de/tecbeat/mailboxd/commit/b0f229618c16ee5bb20f75eb3e9239c68c5c67af))
- *(blob)* Replace global RwLock with per-account Arc<AccountHandle> - ([4e12ae5](https://git.teccave.de/tecbeat/mailboxd/commit/4e12ae5457cbe109955fd23adb72916df356da12))
- *(cargo)* Rename workspace crates bichon-* to mailboxd-* - ([008e2d0](https://git.teccave.de/tecbeat/mailboxd/commit/008e2d0cd74371c144457ddc07023f42bfaf13cf))
- *(deploy)* Rename dockerfile, systemd unit, env template and changelog config to mailboxd - ([8d6e1d7](https://git.teccave.de/tecbeat/mailboxd/commit/8d6e1d740f22091ed2f7433f770d67ea52780dfc))
- *(migrate)* Use searchable_segment_ids() instead of reader() for merge #261 - ([26c14fc](https://git.teccave.de/tecbeat/mailboxd/commit/26c14fcaaf5b0f7ecedd81d8c70a7e347d23e67d))
- *(search)* Search filtering and sorting - ([b490923](https://git.teccave.de/tecbeat/mailboxd/commit/b490923e17229ac90473a0cba7d7ba57b6a7df01))
- *(server)* Rebrand env vars, cli flags, symbols and data paths to mailboxd - ([5fe0665](https://git.teccave.de/tecbeat/mailboxd/commit/5fe06653ef928653bb2d25e5dd6cda1799d34a83))
- *(web)* Rebrand webui strings, i18n, headers and source link to mailboxd - ([e388a67](https://git.teccave.de/tecbeat/mailboxd/commit/e388a67794d79a4a6964ae92d2fabda6eaa59220))
- *(workspace)* Decompose project into multiple crates - ([0b866c8](https://git.teccave.de/tecbeat/mailboxd/commit/0b866c81ffbb05e1d6a93677febadadb75b0c0ef))
- Remove pro/enterprise feature-flag scaffolding - ([c82e394](https://git.teccave.de/tecbeat/mailboxd/commit/c82e394b671bcac816f5ff58b7a671750999035f))
- Replace autoconfig with native impl, remove openssl dependency - ([7afb1e2](https://git.teccave.de/tecbeat/mailboxd/commit/7afb1e29aa243294ddd22a4d0531bddba789acb9))
- Replace native_db with memdb and add tests - ([5406c43](https://git.teccave.de/tecbeat/mailboxd/commit/5406c4322c4556bae59b343589b1d06bb5d2b652))
- Decouple email body and attachment storage - ([a41b541](https://git.teccave.de/tecbeat/mailboxd/commit/a41b5417e3d9776d4610d0599d31bd2b1d5f6ddd))
- Use UUID for envelope id to prevent accidental deletion - ([d690f57](https://git.teccave.de/tecbeat/mailboxd/commit/d690f57290117304c9380c01b731a7f5cc7901bf))
- [**breaking**] Replace Tantivy search engine with DuckDB - ([d2936ed](https://git.teccave.de/tecbeat/mailboxd/commit/d2936ed4a71d9813a7032b450d3c1132e74b31e5))

### 📚 Documentation

- *(openapi)* Update attachment download param to content_hash  #314 - ([524201c](https://git.teccave.de/tecbeat/mailboxd/commit/524201c26de396cc7e05e2d527d64eab4df42240))
- Rewrite readme for mailboxd fork and add NOTICE with AGPL attribution - ([311b2ae](https://git.teccave.de/tecbeat/mailboxd/commit/311b2ae84aa489694da9f38f006b25d2308b17d6))

### ⚡ Performance

- Optimize IMAP account fetch flow - ([83dd9cd](https://git.teccave.de/tecbeat/mailboxd/commit/83dd9cdd6be029d7c079cf49373e991f013c5365))
- Reduce tokio worker thread blocking to improve responsiveness on low-core machines - ([0792bb5](https://git.teccave.de/tecbeat/mailboxd/commit/0792bb546d3cf648ae1e08dc09233eb6f9e0e77b))

### 🎨 Styling

- *(web)* Shrink sidebar logo by 15% (60->51 open, 40->34 collapsed) - ([75474b1](https://git.teccave.de/tecbeat/mailboxd/commit/75474b1de4848bf266b394bc76ef27817c0251ac))
- *(web)* Drop the icon above the sign-in card - ([bb96c59](https://git.teccave.de/tecbeat/mailboxd/commit/bb96c5973ddfcd908784fe02dcfd2a7d010cf33a))

### 🧪 Testing

- *(blob)* Add concurrent access and crash recovery integration tests - ([b6957c8](https://git.teccave.de/tecbeat/mailboxd/commit/b6957c8ceb6a3420d4bb9557023903612cad136d))

### ⚙️ Miscellaneous Tasks

- *(blob)* Add bincode dependency and new meta error variants - ([cc54063](https://git.teccave.de/tecbeat/mailboxd/commit/cc540636694bb9a098c86902ec75645890a5c9ed))
- *(deps)* Replace async-imap with custom fork for imap-proto update - ([0c2e540](https://git.teccave.de/tecbeat/mailboxd/commit/0c2e54083472c8a17f98601fb7a63fda9fd6c024))
- *(docker)* Add root Dockerfile and enable teccave build_image pipeline - ([aba59e8](https://git.teccave.de/tecbeat/mailboxd/commit/aba59e8360c62341c622ba1b78dd55f0706120ce))
- *(release)* Package bichonctl together with bichon binaries - ([0bf2003](https://git.teccave.de/tecbeat/mailboxd/commit/0bf2003670c6cd56aa2289cccddaebfd77b041fb))
- *(search ui)* Quick selection of year and month #39 - ([b35493e](https://git.teccave.de/tecbeat/mailboxd/commit/b35493e4e10de88b64040067338177665a362366))
- *(ui)* Add attachment file type icon - ([d334a23](https://git.teccave.de/tecbeat/mailboxd/commit/d334a23ca7f17243edbfbd3d1729b73ec7b6614d))
- *(ui)* Adjust width of account list for better visibility - ([000d144](https://git.teccave.de/tecbeat/mailboxd/commit/000d144e70555c4082432f5029c721c8e3015b3f))
- *(ui)* Move name field to step 2 #30 - ([b0ce767](https://git.teccave.de/tecbeat/mailboxd/commit/b0ce7671a85d37d309ca3c53f3a67ecbcd9e3510))
- Drop unused tool configs and consolidate docker layout - ([95bab80](https://git.teccave.de/tecbeat/mailboxd/commit/95bab808b50ba3b5f8d9e14b05e99ff8c4bd9706))
- Drop unused build/mailboxd.service systemd unit - ([d337fa0](https://git.teccave.de/tecbeat/mailboxd/commit/d337fa054903c93e9a295d2c8903d5196a7d0c22))
- Rebuild image (cleanup canceled to preserve tag) - ([d79f7cf](https://git.teccave.de/tecbeat/mailboxd/commit/d79f7cf3423c7503a58b4f32c5b756933ba95678))
- Merge teccave repository seed - ([33aac4a](https://git.teccave.de/tecbeat/mailboxd/commit/33aac4a0594b1fd8e304fdcd1791ba80b285c51c))
- Remove github-specific and rustmailer-legacy files - ([2615f0e](https://git.teccave.de/tecbeat/mailboxd/commit/2615f0e9f591601e52fe91750b8f591204a42461))
- Remove custom global allocator - ([4bd714a](https://git.teccave.de/tecbeat/mailboxd/commit/4bd714a67013f16bd26c104fd78436aba994b28f))
- Add trace logging for duplicate email diagnosis #214 - ([ec3e842](https://git.teccave.de/tecbeat/mailboxd/commit/ec3e842bbb6f819bd78993b5cfbe15a0d0471f16))
- Optimize CPU usage #159 - ([54ed2c3](https://git.teccave.de/tecbeat/mailboxd/commit/54ed2c3c0aa9ea1c8df7a7feb0d36d9d86849bac))
- Remove bb8 pool for IMAP; create a new session per operation to avoid stale connections - ([5798227](https://git.teccave.de/tecbeat/mailboxd/commit/579822762f0c655018b1230598de76fb12241cba))
- Adjust IMAP connection timeout configuration - ([bdbbc04](https://git.teccave.de/tecbeat/mailboxd/commit/bdbbc04832e75007b3a74b4358e0abdd3ce39f61))
- Docker: bundle bichonctl and bichon-admin into Docker image #136 - ([0fc99aa](https://git.teccave.de/tecbeat/mailboxd/commit/0fc99aa172cf31dbf139aadc41fbf8f561a5d98e))
- Docker: bundle bichonctl and bichon-admin into Docker image - ([852ae2b](https://git.teccave.de/tecbeat/mailboxd/commit/852ae2b782169682e94842684cfa0fca29fac418))
- Support bulk restore emails - ([e1f471b](https://git.teccave.de/tecbeat/mailboxd/commit/e1f471b8f4fb67ffd3d6d30d0017c268d0d3b36f))
- Set minimum username length to 3 #106 - ([f82b20e](https://git.teccave.de/tecbeat/mailboxd/commit/f82b20e2fdb421050253264f52bada1b50e5d6d8))
- Adjust dark mode brightness and light mode saturation #109 - ([9841038](https://git.teccave.de/tecbeat/mailboxd/commit/9841038acbb4ac9146ac5bbe0d5cf5227793856e))
- Default to binding 0.0.0.0 and support binding IPv6 addresses. - ([97c3db3](https://git.teccave.de/tecbeat/mailboxd/commit/97c3db3bd25ddaab5315f2e676716ad787426ffe))
- Add aarch64-unknown-linux-gnu build and multi-arch Docker support #2 - ([b05bb4f](https://git.teccave.de/tecbeat/mailboxd/commit/b05bb4f48e0b77c97f207feea443ee663d201608))

<!-- generated by teccave -->
