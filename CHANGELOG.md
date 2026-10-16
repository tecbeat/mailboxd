# Changelog

All notable changes to this project will be documented in this file.

## [1.9.0](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.9.0) - 2026-10-16

### ⛰️  Features

- *(ci)* Mirror release images and notes to GitHub - ([272886e](https://git.teccave.de/tecbeat/mailboxd/commit/272886ef5a0064d146e0e4f642c5ddec1755f1a1))

## [1.8.1](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.8.1) - 2026-10-16

### 🐛 Bug Fixes

- *(web)* Restore WebUI by dropping the vendor chunk size cap - ([c28d05b](https://git.teccave.de/tecbeat/mailboxd/commit/c28d05b4420ed70115ebd8fe656ca5d6d14ff4ff))

## [1.8.0](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.8.0) - 2026-10-16

## [1.7.14](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.7.14) - 2026-10-16

### ⛰️  Features

- *(deps)* Add jsonwebtoken 11.1.0 for OIDC ID token verification - ([245dc2e](https://git.teccave.de/tecbeat/mailboxd/commit/245dc2e0937c641af4d604b865694fd3083ffd5b))
- *(oidc)* Verify asymmetric ID tokens against the provider JWKS - ([5b85965](https://git.teccave.de/tecbeat/mailboxd/commit/5b85965bb380e4578866914adf638407f4db9d12))

### 🐛 Bug Fixes

- *(deps)* Update all non-major dependencies - ([223fc06](https://git.teccave.de/tecbeat/mailboxd/commit/223fc0667a771f27aa3ca3867dc16f10f3851199))

### 📚 Documentation

- *(oidc)* Document JWKS-based ID token verification - ([f5cde6e](https://git.teccave.de/tecbeat/mailboxd/commit/f5cde6e5b9e5d791f1d07cb8c01f55ab96b1c407))

## [1.7.13](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.7.13) - 2026-10-15

### 🐛 Bug Fixes

- *(deps)* Bump mdast-util-to-hast to 13.2.1 - ([fa0ce68](https://git.teccave.de/tecbeat/mailboxd/commit/fa0ce6851c434937c6a361eb8ec55146c62c3a05))
- *(deps)* Update debian:bookworm-slim docker digest to 7c7b2c9 - ([f36fb80](https://git.teccave.de/tecbeat/mailboxd/commit/f36fb80b34c992b886b5cacffc8a4b1c77a51d8d))
- *(deps)* Update rust crate zip to v9 - ([6b9af7d](https://git.teccave.de/tecbeat/mailboxd/commit/6b9af7d867e26d6d91bf1c9f6ca0a78f8f3ff617))
- *(deps)* Update node.js to 44fce69 - ([41a55ae](https://git.teccave.de/tecbeat/mailboxd/commit/41a55ae94b6d9b08def4ffb02409b9e8a42170b7))
- *(ext)* Adapt OOXML extraction to fallible zip 9 entry names - ([9c24278](https://git.teccave.de/tecbeat/mailboxd/commit/9c24278d0b9710af1ecb77859cc4b8f2b21b5470))
- *(server)* Drop deprecated std::u32 module import - ([b24eb0b](https://git.teccave.de/tecbeat/mailboxd/commit/b24eb0b9cc7756a7af05365e114d4e5d49098526))
- *(web)* Resolve vite config and chunk size build warnings - ([9497f1c](https://git.teccave.de/tecbeat/mailboxd/commit/9497f1c4b3c101a28f0fef781f64b2141afc4936))

### ⚙️ Miscellaneous Tasks

- *(deps)* Update gitlab-ci - ([254d3c4](https://git.teccave.de/tecbeat/mailboxd/commit/254d3c4b3e0d11b2d7dad738c62cc6e69ab0abea))

## [1.7.12](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.7.12) - 2026-10-15

### 🐛 Bug Fixes

- *(deps)* Update all non-major dependencies - ([78ec531](https://git.teccave.de/tecbeat/mailboxd/commit/78ec531b4d07fe7e99db248e242c6b5fb6c92e20))

## [1.7.11](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.7.11) - 2026-10-12

### 🐛 Bug Fixes

- *(deps)* Update rust:1.99.0-slim-bookworm docker digest to 2c3a22f - ([d172955](https://git.teccave.de/tecbeat/mailboxd/commit/d1729557d4e6280d90d71a119a47e05808c314c9))

## [1.7.10](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.7.10) - 2026-10-11

### 🐛 Bug Fixes

- *(deps)* Update all non-major dependencies - ([28a885c](https://git.teccave.de/tecbeat/mailboxd/commit/28a885c5145a48107068a9d89cfe15584fcd7765))

## [1.7.9](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.7.9) - 2026-10-08

### 🐛 Bug Fixes

- *(deps)* Update all non-major dependencies - ([3da373b](https://git.teccave.de/tecbeat/mailboxd/commit/3da373be028f76b978a1663c43f3d77381ccdc0b))

### ⚙️ Miscellaneous Tasks

- *(ui)* Update logo and favicon to new mailboxd icon - ([76f9ed8](https://git.teccave.de/tecbeat/mailboxd/commit/76f9ed8090ab4e414e7b9ccf8e252182ddcafbee))

## [1.7.8](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.7.8) - 2026-10-06

### 🐛 Bug Fixes

- *(deps)* Update all non-major dependencies - ([4895154](https://git.teccave.de/tecbeat/mailboxd/commit/4895154ab4caeabe8cf8554cff40b4006c3d4521))

## [1.7.7](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.7.7) - 2026-10-05

### 🐛 Bug Fixes

- *(deps)* Update all non-major dependencies - ([41a5231](https://git.teccave.de/tecbeat/mailboxd/commit/41a52311c107c230c608035149fdf772f4513580))

## [1.7.6](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.7.6) - 2026-10-03

### 🐛 Bug Fixes

- *(deps)* Update all non-major dependencies - ([3b71e57](https://git.teccave.de/tecbeat/mailboxd/commit/3b71e572b9a7843fbd83a1e1fe168f1e3cd21d2a))

## [1.7.5](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.7.5) - 2026-10-01

### 🐛 Bug Fixes

- *(deps)* Update dependency msw to v3 - ([7f68d20](https://git.teccave.de/tecbeat/mailboxd/commit/7f68d200d46be586fd40ba518692a20d6617e51c))
- *(deps)* Update all non-major dependencies - ([2e70955](https://git.teccave.de/tecbeat/mailboxd/commit/2e709551e2de80279dc3d4d63c25c3b897e92428))
- *(deps)* Update rust:1.98.1-slim-bookworm docker digest to ff52144 - ([0fad411](https://git.teccave.de/tecbeat/mailboxd/commit/0fad4115a6bf3b051141b0b650174b800dd77c4d))
- *(deps)* Update node.js to ebfe2f9 - ([d7c687f](https://git.teccave.de/tecbeat/mailboxd/commit/d7c687f2d0f8e1a1a6bc55febcf7661866f417e3))
- *(deps)* Update docker/dockerfile:1.27 docker digest to 4edf897 - ([a6e7623](https://git.teccave.de/tecbeat/mailboxd/commit/a6e7623fba32a84f45521f963ed27a3b5e7e23f1))
- *(deps)* Update debian:bookworm-slim docker digest to 3783cc0 - ([2fa400d](https://git.teccave.de/tecbeat/mailboxd/commit/2fa400da5cb2153c9dd49748ad7c6f1648823e86))

### 🧪 Testing

- *(web)* Make jsdom same-origin so msw v3 CORS doesn't break tests - ([998ade7](https://git.teccave.de/tecbeat/mailboxd/commit/998ade776be94b23660cc35163487a6959f9bc34))

## [1.7.4](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.7.4) - 2026-09-30

### 🐛 Bug Fixes

- *(deps)* Update all non-major dependencies - ([a92215c](https://git.teccave.de/tecbeat/mailboxd/commit/a92215c83a2cbd709967f0f2f27eff70d41f0e48))

### ⚙️ Miscellaneous Tasks

- *(deps)* Update gitlab-ci - ([060e1dc](https://git.teccave.de/tecbeat/mailboxd/commit/060e1dccf17b96006f48e44e5b87fd2973224f21))

## [1.7.3](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.7.3) - 2026-09-18

### 🐛 Bug Fixes

- *(deps)* Update node.js to 760e44b - ([583fc59](https://git.teccave.de/tecbeat/mailboxd/commit/583fc5937b133a443333c23bf135fabbdf1472c1))

### ⚙️ Miscellaneous Tasks

- *(deps)* Update Node.js to 760e44b - ([7c2994f](https://git.teccave.de/tecbeat/mailboxd/commit/7c2994f947c24d302f9ff25320e97a7cd1a904b5))

## [1.7.2](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.7.2) - 2026-09-12

### 🐛 Bug Fixes

- *(deps)* Update all non-major dependencies - ([38a3d91](https://git.teccave.de/tecbeat/mailboxd/commit/38a3d91e17abd96c8a98216728ec64b37fc40b7f))

## [1.7.1](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.7.1) - 2026-09-09

### 🐛 Bug Fixes

- *(deps)* Update node.js to 50c8e8c - ([5c62341](https://git.teccave.de/tecbeat/mailboxd/commit/5c623410df8cc56590929200c8f925cf076098a7))

### ⚙️ Miscellaneous Tasks

- *(deps)* Update Node.js to 50c8e8c - ([fb1cb17](https://git.teccave.de/tecbeat/mailboxd/commit/fb1cb178c0427522d3f9c7695dca5a845b6fcd87))

## [1.7.0](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.7.0) - 2026-09-09

### ⛰️  Features

- *(audit)* Add admin-only audit log page to the web UI - ([4a583cb](https://git.teccave.de/tecbeat/mailboxd/commit/4a583cbae9e5dde8144afa4fd1ad55c1678f3013))
- *(audit)* Add persistent audit log for security-relevant events - ([c8efb1e](https://git.teccave.de/tecbeat/mailboxd/commit/c8efb1e805895a20f8efcc41b2c3d5962dc0589f))
- *(backup)* Add admin backup settings page (Refs #42) - ([846c42a](https://git.teccave.de/tecbeat/mailboxd/commit/846c42aa8eadc57a514bdd3aa8c1b3172f54724c))
- *(backup)* Add admin backup REST API and audit events (Refs #42) - ([129f2a7](https://git.teccave.de/tecbeat/mailboxd/commit/129f2a7ab7bb60ba726ac214e71cd0a7c7cf7ccc))
- *(backup)* Add instance backup and staged restore core (Refs #42) - ([70efeaf](https://git.teccave.de/tecbeat/mailboxd/commit/70efeafe1379a45cd2c956f0c1d29201e6439f6b))
- *(search)* Ship default attachment text extractor (Refs #4) - ([4b79f71](https://git.teccave.de/tecbeat/mailboxd/commit/4b79f713389849c934928fd223e3f894948f80e0))

### 🐛 Bug Fixes

- *(build)* Regenerate route tree for router-plugin 1.168.36 - ([2a11b4b](https://git.teccave.de/tecbeat/mailboxd/commit/2a11b4baf0ccfa34fa83bb89352c38d974bb2edc))
- *(ci)* Skip unittest gate jobs on the default branch - ([ba4f8b9](https://git.teccave.de/tecbeat/mailboxd/commit/ba4f8b9d8a0d93b2ca69d139b1aead6d0164d026))
- *(deps)* Pin knip to 6.34.0 to satisfy supply-chain release-age policy - ([896b4d7](https://git.teccave.de/tecbeat/mailboxd/commit/896b4d7b51836aed87756ad7e692ec8b267f5367))
- *(deps)* Update all non-major dependencies - ([7bf139b](https://git.teccave.de/tecbeat/mailboxd/commit/7bf139b7da0390afd5f24e5b4288d0ce841b55b6))
- *(oidc)* Localize SSO strings, document HS256 requirement (Refs #5) - ([bb3ad26](https://git.teccave.de/tecbeat/mailboxd/commit/bb3ad265201ae00513d7d7f65bb095c74ec6b14d))

### 📚 Documentation

- *(backup)* Document backup env vars and add compose bind-mount (Refs #42) - ([754c540](https://git.teccave.de/tecbeat/mailboxd/commit/754c540ee1ba6e7228b69b775707b012bdd8add8))

## [1.6.0](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.6.0) - 2026-09-08

### ⛰️  Features

- *(layout)* Shared PageHeader with action slot + consistent nav chrome (#22, #23, #40) - ([b71c647](https://git.teccave.de/tecbeat/mailboxd/commit/b71c64763c39a4ee7b167cca43996d2fdb7d2e28))
- *(profile)* Inline SSO-managed-credentials note and testable user update (#17) - ([333f64a](https://git.teccave.de/tecbeat/mailboxd/commit/333f64a1e6b0717e29eccbbc964944b3623c194a))
- *(profile)* Hide password field for SSO users - ([0958932](https://git.teccave.de/tecbeat/mailboxd/commit/095893245446d7c09569d22ac9e17a26919941ba))
- *(ui)* Shared EmptyState component + unified card surfaces and empty-state heights (#26, #39) - ([5ffbb2e](https://git.teccave.de/tecbeat/mailboxd/commit/5ffbb2e675bd41e5ca9de44e0bc317c201b7cb28))
- *(web)* Add semantic success color token - ([41468c6](https://git.teccave.de/tecbeat/mailboxd/commit/41468c66719aa4b0378f0ef3fd39570f27ae5627))
- *(web)* Integrate search into the filter toolbar (#24) - ([eef4ce3](https://git.teccave.de/tecbeat/mailboxd/commit/eef4ce32f9cf79da47bfd6f51013110fc3c6247f))

### 🐛 Bug Fixes

- *(build)* Force pnpm copy import method in Docker web build - ([a89ff72](https://git.teccave.de/tecbeat/mailboxd/commit/a89ff72f14a1c6545ff0c7386bbfd15610d8e654))
- *(dashboard)* Render loading state inside page chrome, mirror grid (#28) - ([fbd55f1](https://git.teccave.de/tecbeat/mailboxd/commit/fbd55f10bb18f078ac0da7ebfba211074a89e2f7))
- *(data-table)* Move vertical scrollbar outside the bordered box - ([8fd506f](https://git.teccave.de/tecbeat/mailboxd/commit/8fd506f2da3f21a492ecf72a3dbad8f29dae5e86))
- *(deps)* Update @tanstack/router-plugin to 1.168.36 - ([620dbe7](https://git.teccave.de/tecbeat/mailboxd/commit/620dbe7a0d18254f5f028920b657efaafef661af))
- *(deps)* Sync Cargo.lock with zstd 0.14 bump - ([251af1e](https://git.teccave.de/tecbeat/mailboxd/commit/251af1e3c50535bd01518ee94eaa63611b964e53))
- *(deps)* Align corepack pnpm pins to v12.3.4 - ([d7a3534](https://git.teccave.de/tecbeat/mailboxd/commit/d7a35340688838efcd6fe62d0c524943b2e660e5))
- *(deps)* Update pnpm to v12 - ([7ff91fb](https://git.teccave.de/tecbeat/mailboxd/commit/7ff91fbd54c4ad1b9dc8e88adc23a32e0fac5e8b))
- *(deps)* Update all non-major dependencies - ([8b1735e](https://git.teccave.de/tecbeat/mailboxd/commit/8b1735e573d1cb81f986ac735c53ff6cd690802e))
- *(deps)* Update vitest monorepo to v5 - ([a3ac5e9](https://git.teccave.de/tecbeat/mailboxd/commit/a3ac5e9261d35b61cc2f90ae40bf403737ad3c8a))
- *(format)* Route remaining timestamps through shared date formatters (#31) - ([7eb96a7](https://git.teccave.de/tecbeat/mailboxd/commit/7eb96a7742457367ff7395666d76f6ce3694df89))
- *(layout)* Unify page header bottom margin to mb-4 - ([1c6cec0](https://git.teccave.de/tecbeat/mailboxd/commit/1c6cec045c5d93225e937889f247d6af9c8ec7fe))
- *(layout)* Flush secondary-nav content with primary-nav right edge - ([c3ec8d2](https://git.teccave.de/tecbeat/mailboxd/commit/c3ec8d2ffcd6ebedfadf034f8bd33bb5ba747d8d))
- *(layout)* Center shared container, keep dashboard full-width, align settings nav (#21) - ([7d20cbd](https://git.teccave.de/tecbeat/mailboxd/commit/7d20cbdf55138ea574f190ebc8e7d7c20e74d0a0))
- *(mail-list)* Align filter toolbar right edge with table scrollbar gutter - ([6adc9e6](https://git.teccave.de/tecbeat/mailboxd/commit/6adc9e6329ae81e0166e47fc2de4a0048a7331e0))
- *(nav)* Shared SidebarNav shows current section on mobile + correct placeholder key (#18, #41) - ([448025c](https://git.teccave.de/tecbeat/mailboxd/commit/448025cd0cc52d6826dc8529192fe4e1534298aa))
- *(search)* Animate table updates instead of reloading the whole page - ([ede0146](https://git.teccave.de/tecbeat/mailboxd/commit/ede0146df33ba9119d579d8ad192a6bbfbfbfc49))
- *(ui)* Route remaining status badges through shared color helpers (#30) - ([6132c80](https://git.teccave.de/tecbeat/mailboxd/commit/6132c808b2825dadbdb975e72f5db78b667b6acb))
- *(ui)* Unify page-size options across all tables (#32) - ([95dbb50](https://git.teccave.de/tecbeat/mailboxd/commit/95dbb50d868c3dc44dbf4cc4f7d1fad5763fcbd1))
- *(web)* Correct hook dependency arrays - ([c1c1beb](https://git.teccave.de/tecbeat/mailboxd/commit/c1c1beb8e4302a0df9f4a0344bea29e3f018a77f))
- *(web)* Stabilize sort setters and complete effect deps - ([a94a0ce](https://git.teccave.de/tecbeat/mailboxd/commit/a94a0ceb1a98a3f8147505aa588ce08fc2ad8c09))
- *(web)* Keep IMAP password input controlled - ([e18e2c0](https://git.teccave.de/tecbeat/mailboxd/commit/e18e2c06bc130f4545e13ea7b09ae02c17bdc314))
- *(web)* Send JSON body for start-download to avoid 415 - ([1721fc8](https://git.teccave.de/tecbeat/mailboxd/commit/1721fc8a9c632764fcc1ab801e9f6f11d88f119a))
- *(web)* Align badge and plain values on one right edge in system config - ([d111432](https://git.teccave.de/tecbeat/mailboxd/commit/d1114323e49dc58b57f8aad0cd1b0cf43b4a7d2a))
- *(web)* Label bulk-action toolbar icon buttons - ([ac03ad7](https://git.teccave.de/tecbeat/mailboxd/commit/ac03ad7ae35c5e1ad8e3cad296e13ad5a54bd0d4))
- *(web)* Avoid nested interactive element in filter selector fields - ([10dd196](https://git.teccave.de/tecbeat/mailboxd/commit/10dd196548d69af07247be7e742d6585a7324502))
- *(web)* Add missing search_mailbox i18n keys - ([8311e47](https://git.teccave.de/tecbeat/mailboxd/commit/8311e47e64e10acc045529ed13fab6657d570dd5))
- *(web)* Keep search input focused while live results load - ([cbfadb7](https://git.teccave.de/tecbeat/mailboxd/commit/cbfadb7f2b7291133fd5029c6aa269bc69b2bb41))
- *(web)* Use text-destructive for row-action delete items (#29) - ([63996e5](https://git.teccave.de/tecbeat/mailboxd/commit/63996e587bf8374358b99bd788c9a9a9a13f67b5))
- *(web)* Replace hardcoded UI strings with i18n keys (#36) - ([3598468](https://git.teccave.de/tecbeat/mailboxd/commit/3598468082fb057ba83ea79cacaffa551f7468d3))
- *(web)* Persist header theme switch to user profile (#20) - ([853cf38](https://git.teccave.de/tecbeat/mailboxd/commit/853cf38f47774ff1ab0db2f33e736ddfe5e6b48e))
- *(web)* Make blue-dark surfaces distinct from background (#19) - ([91d8def](https://git.teccave.de/tecbeat/mailboxd/commit/91d8defb861ae8a1897abb995a1d5a9fdc473fc8))
- *(web)* Prevent system configurations rows from overflowing - ([c91ed88](https://git.teccave.de/tecbeat/mailboxd/commit/c91ed88df3ffcfe64764e6f9eacf3759d63a8d01))
- *(web)* Align users section shell with settings for consistent headers - ([8491aef](https://git.teccave.de/tecbeat/mailboxd/commit/8491aef7a0e9bf2026cd2c9e67456a7ed5c44181))
- *(web)* Tighten dashboard spacing so it fits without page scroll - ([7cb3e4d](https://git.teccave.de/tecbeat/mailboxd/commit/7cb3e4d4c3efc28e5cad12cfd93d151bc1220b04))
- *(web)* Use theme tokens in mail views for dark theme support (#27) - ([9ee3ca4](https://git.teccave.de/tecbeat/mailboxd/commit/9ee3ca45c1ea3bfe6c52667c6fa9fe3dec19f9ad))
- *(web)* Replace hardcoded palette colors with theme tokens (#29) - ([cf290a2](https://git.teccave.de/tecbeat/mailboxd/commit/cf290a2859631e3bce856e1c4ec85ff60a1edfda))
- *(web)* Right-align form action buttons in profile and appearance (#25) - ([4c52ea8](https://git.teccave.de/tecbeat/mailboxd/commit/4c52ea87fbc6c83cc1f517adbd47002b9f3ad091))
- *(web)* Reserve the settings scrollbar gutter to prevent tab shift - ([6b1b78f](https://git.teccave.de/tecbeat/mailboxd/commit/6b1b78f7ec5f8af21423dc2fa4447dc5b3500ed9))
- *(web)* Standardize the system configurations settings page - ([4688602](https://git.teccave.de/tecbeat/mailboxd/commit/4688602bfcb89f107a66f824ff1b012b6b6b50ca))
- *(web)* Left-align API docs cards in a 3-column grid - ([0aa4545](https://git.teccave.de/tecbeat/mailboxd/commit/0aa4545c1b5b0ffb72bf643c55567294c3a7b70c))
- *(web)* Use the standard PageHeader on the API documentation page - ([5ec8109](https://git.teccave.de/tecbeat/mailboxd/commit/5ec81095a15864aa563520f499134b23e9b9adef))
- *(web)* Hug table content and move the horizontal scrollbar below the border - ([80b9438](https://git.teccave.de/tecbeat/mailboxd/commit/80b94381f52583c7d69f6247b54d3ae3fdf8be77))
- *(web)* Highlight the active settings nav item by resolved active href - ([40e478d](https://git.teccave.de/tecbeat/mailboxd/commit/40e478d0c5c11c5257a7c2fa5c5f41346de10122))
- *(web)* Center page content and widen the max-width cap to 1600px - ([bed07f8](https://git.teccave.de/tecbeat/mailboxd/commit/bed07f830e9fb93bc35a93a7a16713291be96d94))
- *(web)* Unify list-page scrolling with pinned toolbar and pagination - ([694f776](https://git.teccave.de/tecbeat/mailboxd/commit/694f776c39478bb2ef2d79054c292477d4b8cf25))
- *(web)* Let mail table flow into the shell scroll region (#29) - ([ea8cebc](https://git.teccave.de/tecbeat/mailboxd/commit/ea8cebc9cdbf3ab623792fea8c90fc3859fd42c1))
- *(web)* Establish single content scroll region in app shell (#21) - ([9caa206](https://git.teccave.de/tecbeat/mailboxd/commit/9caa20600a3b571c0fc1f2d24e4d5358e39d6512))

### 🚜 Refactor

- *(format)* Locale-aware formatDateTime and formatRelativeTime helpers, migrate all call sites (#31) - ([fab4f7b](https://git.teccave.de/tecbeat/mailboxd/commit/fab4f7bd9ff4bf37a7c52e230f1316a8839bde67))
- *(layout)* Unified page container and single padding contract (#21, #38) - ([1738f66](https://git.teccave.de/tecbeat/mailboxd/commit/1738f66c4a663d83f455d71073f724de8652876e))
- *(ui)* Default EmptyState logo icon and migrate remaining empty state (#26, #39) - ([084dcc8](https://git.teccave.de/tecbeat/mailboxd/commit/084dcc844c7bb356a3b003165ce6eb6cc0fcc5b4))
- *(ui)* Consolidate table pagination into one component (#32) - ([e7007ad](https://git.teccave.de/tecbeat/mailboxd/commit/e7007add11d5cae24be23df255d30f07a9fb6cf8))
- *(ui)* Shared Spinner and unified loading states (#33) - ([ae36cb4](https://git.teccave.de/tecbeat/mailboxd/commit/ae36cb4503818b68471c988bbff908f976b851bf))
- *(ui)* Single source of truth for status and file-type colors (#30) - ([490e49d](https://git.teccave.de/tecbeat/mailboxd/commit/490e49de85e5346baa233f9ba8d459b99e36875a))
- *(web)* Extract non-component exports from mail-list modules - ([a01dc82](https://git.teccave.de/tecbeat/mailboxd/commit/a01dc82060faef9cfd479dafb0a8dd9d02eb38b9))
- *(web)* Split context providers into component-only files - ([5d670ba](https://git.teccave.de/tecbeat/mailboxd/commit/5d670ba2b3df172e7119d6bd7418786b7ea7e7d7))
- *(web)* Sync account settings form via RHF values prop - ([e7302f4](https://git.teccave.de/tecbeat/mailboxd/commit/e7302f48294dd1358ace6aac527c94ad953465c3))
- *(web)* Read form fields via useWatch in dialogs - ([4db8e8f](https://git.teccave.de/tecbeat/mailboxd/commit/4db8e8f7f93c56b95fb337cc29961ec655d2ff2e))
- *(web)* Stop rest-spreading query results in hooks - ([7d78e12](https://git.teccave.de/tecbeat/mailboxd/commit/7d78e123b482b4e2a9458bb6dc39f3fb2ae3d86d))
- *(web)* Place the filter reset right after the search field - ([868713e](https://git.teccave.de/tecbeat/mailboxd/commit/868713ef3a557ebf8c7ecc760ff826586d238949))
- *(web)* Refine filter bar wrap, font and fields trigger (#24) - ([fc76752](https://git.teccave.de/tecbeat/mailboxd/commit/fc76752ba659c80ef45b2b8688fff6b7bb0cc493))
- *(web)* Polish unified filter bar layout and responsiveness (#24) - ([ccc1949](https://git.teccave.de/tecbeat/mailboxd/commit/ccc194901ab738d14f9d6b681e1579ad3883bb61))
- *(web)* Unify search and filters into a single toolbar (#24) - ([b21c76a](https://git.teccave.de/tecbeat/mailboxd/commit/b21c76a0b403418bc81e15921cf59f1552b4d56f))
- *(web)* Standardize dialog sizes, footers and naming (#34) - ([41e900b](https://git.teccave.de/tecbeat/mailboxd/commit/41e900b7e52a2fcde0b77145d759157f52ec62e7))
- *(web)* Standardize icon usage (#35) - ([3458a22](https://git.teccave.de/tecbeat/mailboxd/commit/3458a221077fe5a1f4246cf226554d80a54e1417))
- *(web)* Unify dashboard card typography (#37) - ([db5d585](https://git.teccave.de/tecbeat/mailboxd/commit/db5d585f5d68d65ac59825e96fe6116778996d1a))

### ⚙️ Miscellaneous Tasks

- *(cleanup)* Remove dead code and normalize new-file license headers (#41) - ([247968c](https://git.teccave.de/tecbeat/mailboxd/commit/247968c4b018cc61fe2097e38c5881216d344044))
- *(cleanup)* Standardize toast access, fix oauth2 toolbar width, remove dead code (#41) - ([5f6f224](https://git.teccave.de/tecbeat/mailboxd/commit/5f6f2245e2cdaf22a50320ac0781b9ee5ba6758f))
- *(git)* Ignore pnpm store and centralize frontend ignores - ([97a6e09](https://git.teccave.de/tecbeat/mailboxd/commit/97a6e092c7da51fc788a5645a24a067cb7c3a0fc))
- *(web)* Scope react-refresh rule to skip routes and test files - ([77d0a52](https://git.teccave.de/tecbeat/mailboxd/commit/77d0a52deb978dfa5c3826ec4c5ad9b1e2d4415c))

## [1.5.1](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.5.1) - 2026-09-02

### 🐛 Bug Fixes

- *(deps)* Drop user-event and react-refresh bumps blocked by release-age policy - ([335ac53](https://git.teccave.de/tecbeat/mailboxd/commit/335ac53cad8df08924d65921102c75ea0f7a3274))
- *(deps)* Update all non-major dependencies - ([f86c05e](https://git.teccave.de/tecbeat/mailboxd/commit/f86c05ebd170031ba03d76199e0734529fe853b8))

## [1.5.0](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.5.0) - 2026-09-02

### 🐛 Bug Fixes

- *(web)* Resolve all ESLint errors (#16) - ([5a01750](https://git.teccave.de/tecbeat/mailboxd/commit/5a017506d5a8a0760dff905506e9a73766344c95))
- *(web)* Guard proxy and oauth2 delete dialogs against double submit - ([229a4f1](https://git.teccave.de/tecbeat/mailboxd/commit/229a4f128e7996f62f60c17bf10d5360861f2b12))
- *(web)* Correct stale react-query invalidation keys - ([933d6e9](https://git.teccave.de/tecbeat/mailboxd/commit/933d6e97a58e37a548041ecc0c4794be6556958b))
- *(web)* Refresh both message views when deleting messages - ([6333261](https://git.teccave.de/tecbeat/mailboxd/commit/6333261e98e3bc95f41c08777c76c438077fc334))
- *(web)* Render delete-mailbox dialog in attachment view - ([4065118](https://git.teccave.de/tecbeat/mailboxd/commit/40651186250892011a9e72b78a5ac75843c3885f))
- *(web)* Read current upload limits when queuing import files - ([467cd31](https://git.teccave.de/tecbeat/mailboxd/commit/467cd318a3f9ae0859a45213f0e223faa242b07f))
- *(web)* Pin typescript to 6.0.3 to keep eslint compatible - ([bdc6a2e](https://git.teccave.de/tecbeat/mailboxd/commit/bdc6a2e916532ad9996c8332d2b45f54d89fc465))

### 🚜 Refactor

- *(api-tokens)* Drop redundant console.error in token dialog - ([6ffedb8](https://git.teccave.de/tecbeat/mailboxd/commit/6ffedb8ba2d30dd3086f3bfedcc58cbec30f6e9a))
- *(api-tokens)* Consolidate duplicate token UIs into shared module - ([d2c8df9](https://git.teccave.de/tecbeat/mailboxd/commit/d2c8df9c31abfa24ae1da4024406401c2d9ec2f6))
- *(web)* Standardize icons on lucide-react (#12) - ([b152828](https://git.teccave.de/tecbeat/mailboxd/commit/b152828c4f013ebcd4f11d4c520ea6ec2b78c38f))
- *(web)* Share generic DataTable scaffold across CRUD features (#15) - ([5ae9f14](https://git.teccave.de/tecbeat/mailboxd/commit/5ae9f148654eb399e8c9ac0d3f232c25d2cb1bc6))
- *(web)* Remove duplicated route, helpers, and Button primitive (#13) - ([cd98de5](https://git.teccave.de/tecbeat/mailboxd/commit/cd98de5a080b9723ab6855b363dbc1f06e82540d))
- *(web)* Unify search/attachment tables into shared MailListDataTable - ([a65d2d8](https://git.teccave.de/tecbeat/mailboxd/commit/a65d2d8d029cc1a7240244d30b1fe6bc2125e898))
- *(web)* Extract shared more-filters shell and size presets - ([bd82f88](https://git.teccave.de/tecbeat/mailboxd/commit/bd82f8884df3f2386d1c975cf5c697dbcacc0e64))
- *(web)* Unify mail-display-dialog via the envelope keystone - ([030e982](https://git.teccave.de/tecbeat/mailboxd/commit/030e982403db5bc35684b9192d445942b741e368))
- *(web)* Unify mail-message-view and thread-dialog - ([53acf6a](https://git.teccave.de/tecbeat/mailboxd/commit/53acf6a6049d3313f2b05d46490eef28c9accfd8))
- *(web)* Move attachment-preview into shared mail-list - ([23e698c](https://git.teccave.de/tecbeat/mailboxd/commit/23e698c46eb61ac4bfc2737745ee5ce93242aa72))
- *(web)* Unify nested-email-dialog on a props contract - ([26ea875](https://git.teccave.de/tecbeat/mailboxd/commit/26ea875ec3877d9f66b3a463d27ba7f43c77a415))
- *(web)* Unify account-popover, normalize to richer variant - ([0c150a8](https://git.teccave.de/tecbeat/mailboxd/commit/0c150a85c0543db0c46a8e3214f3e5b0eb35268b))
- *(web)* Unify text-search-input via a feature config prop - ([4730fc8](https://git.teccave.de/tecbeat/mailboxd/commit/4730fc8b911fa2b1f9425641bc680c991aa765e1))
- *(web)* Share restore-message dialog via useCurrentEnvelope - ([a646616](https://git.teccave.de/tecbeat/mailboxd/commit/a646616940633b399635f8bd0d3f8d5cb3f06ff1))
- *(web)* Merge behavior-identical mailbox dialog and popover - ([5773f71](https://git.teccave.de/tecbeat/mailboxd/commit/5773f71f9ab0ca46555d6b55801720ec8b891528))
- *(web)* Share identical list primitives via mail-list config - ([0419314](https://git.teccave.de/tecbeat/mailboxd/commit/041931426914223c3df0b6f068585a323b6bae75))
- *(web)* Unify search and attachment list contexts - ([b570a4f](https://git.teccave.de/tecbeat/mailboxd/commit/b570a4fccdaaaa9b73d12094aa5158148008457a))
- *(web)* Extract shared getFileConfig into mail-list module - ([ba7055d](https://git.teccave.de/tecbeat/mailboxd/commit/ba7055dc894381e895dd50b7e19048dc9cde35fa))
- *(web)* Extract shared mail-list table primitives - ([63e0b06](https://git.teccave.de/tecbeat/mailboxd/commit/63e0b06db5902dfd61ba44629bc8e8537e9e0ae6))
- *(web)* Extract attachment dialogs into one component - ([9ea3fb8](https://git.teccave.de/tecbeat/mailboxd/commit/9ea3fb8786d0665252619c653c3f525b7b767377))
- *(web)* Extract import file validation into pure module - ([2dda845](https://git.teccave.de/tecbeat/mailboxd/commit/2dda845f48ccadcea9654488d785b43da2efe447))

### ⚡ Performance

- *(web)* Stop remounting the mailbox tree and memoize selection counts - ([2add72d](https://git.teccave.de/tecbeat/mailboxd/commit/2add72de1616e9949d6975fd891ec1eaaf88ed28))
- *(web)* Resolve user roles via a map instead of nested scans - ([58b6ce2](https://git.teccave.de/tecbeat/mailboxd/commit/58b6ce2ae037f6305bd4e6a117508ae944c6f4a7))

### 🧪 Testing

- *(web)* Characterize shared formatBytes and formatTimestamp (#13) - ([49e4aec](https://git.teccave.de/tecbeat/mailboxd/commit/49e4aecb4de3fdbdeb75fc8ffb38e4d43ba697e2))

### ⚙️ Miscellaneous Tasks

- *(web)* Make eslint a hard gate (#16) - ([a5272f2](https://git.teccave.de/tecbeat/mailboxd/commit/a5272f22bd13a1eb8c300ab88e5cdac3de8fa21f))
- *(web)* Remove dead code and unused dependencies (#14) - ([4abb23c](https://git.teccave.de/tecbeat/mailboxd/commit/4abb23cde33384f2ac1f1e13e608b38a6accd28d))
- *(web)* Use tecbeat-only license header on new files - ([2322692](https://git.teccave.de/tecbeat/mailboxd/commit/2322692840d603fd527e11b3e6f8034f2b1dafb3))
- Gate frontend with typecheck, tests and lint - ([492a2c8](https://git.teccave.de/tecbeat/mailboxd/commit/492a2c8ab8d830c3e42c159c96c00e970dce58bf))

## [1.4.0](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.4.0) - 2026-08-31

### 🐛 Bug Fixes

- *(deps)* Update all non-major dependencies - ([3b3ad61](https://git.teccave.de/tecbeat/mailboxd/commit/3b3ad61a392168f1b13be09c4fe04efe1911c636))
- *(deps)* Update node.js to e67514e - ([018badf](https://git.teccave.de/tecbeat/mailboxd/commit/018badf951e8b0af00a3ad15d8a94a03962c925f))

### 🚜 Refactor

- *(migrate)* Run storage migration in the server, drop entrypoint script - ([83cfed0](https://git.teccave.de/tecbeat/mailboxd/commit/83cfed0725855d8e7d861eaad6b092686a18c85b))

## [1.3.4](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.3.4) - 2026-08-25

## [1.3.3](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.3.3) - 2026-08-25

## [1.3.2](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.3.2) - 2026-08-25

### 🐛 Bug Fixes

- *(deps)* Update rust:1.98.0-slim-bookworm docker digest to 1469a27 - ([271e3c2](https://git.teccave.de/tecbeat/mailboxd/commit/271e3c28d160f960003eacd9146bc3536fc70b72))
- *(deps)* Update debian:bookworm-slim docker digest to 8820086 - ([6fbddd0](https://git.teccave.de/tecbeat/mailboxd/commit/6fbddd07b28b8cc14a8e2c0974f45ce725840c2e))

### ⚙️ Miscellaneous Tasks

- *(deps)* Update rust:1.98.0-slim-bookworm Docker digest to 1469a27 - ([a301ecc](https://git.teccave.de/tecbeat/mailboxd/commit/a301ecca40f257b88d417c044bdc9f25e40fe004))

## [1.3.1](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.3.1) - 2026-08-23

### 🐛 Bug Fixes

- *(license)* Attribute tecbeat-authored OIDC files to tecbeat only - ([5b81e34](https://git.teccave.de/tecbeat/mailboxd/commit/5b81e3424007748cb63f111575c9d14455059c39))

## [1.3.0](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.3.0) - 2026-08-22

### 🐛 Bug Fixes

- *(deps)* Update dependency @tanstack/react-table to v9 - ([890c7c1](https://git.teccave.de/tecbeat/mailboxd/commit/890c7c18741d2316862a66edf28462abe867e916))
- *(deps)* Update dependency eslint to v10.9.0 - ([4dd4d4e](https://git.teccave.de/tecbeat/mailboxd/commit/4dd4d4e6f6fc996b36096763512fc9f57f5b4507))

### 🚜 Refactor

- *(web)* Migrate data tables to @tanstack/react-table v9 API - ([9b03d17](https://git.teccave.de/tecbeat/mailboxd/commit/9b03d17301ea6cf4923a82ecd9a15d6e604edea2))

### ⚙️ Miscellaneous Tasks

- *(license)* Attribute new data-table module to tecbeat only - ([23e23cd](https://git.teccave.de/tecbeat/mailboxd/commit/23e23cdd51c74d3c569a79956083de62b1205a65))

## [1.2.1](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.2.1) - 2026-08-21

### 🐛 Bug Fixes

- *(deps)* Update all non-major dependencies - ([bd3b92a](https://git.teccave.de/tecbeat/mailboxd/commit/bd3b92a4d6d36393476e1f22550537ec66c85cf0))
- Correct migration-guide wiki URL printed after v1 migration - ([2143f08](https://git.teccave.de/tecbeat/mailboxd/commit/2143f08661f26aed2dbfc03ed46f56b77c639e3e))

## [1.2.0](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.2.0) - 2026-08-21

### 🐛 Bug Fixes

- *(admin)* Auto-migrate legacy storage to v2 on startup - ([f20ae72](https://git.teccave.de/tecbeat/mailboxd/commit/f20ae72d3ef9ac1fd70a308fedc9e7aa0cad7d2d))
- *(docker)* Run storage auto-migration before starting server - ([4aeedde](https://git.teccave.de/tecbeat/mailboxd/commit/4aeedded0714393ad7a8dd01e1c108fe21bcdd23))

### 🚜 Refactor

- *(core)* Extract path-parameterized adopt_bichon_layout - ([26e5a58](https://git.teccave.de/tecbeat/mailboxd/commit/26e5a58471f1989f8e5ad27f316020f3951687f5))

### 📚 Documentation

- Correct v2 storage default paths in config.yml - ([b3565d6](https://git.teccave.de/tecbeat/mailboxd/commit/b3565d64d674d79a12b27078237e80d736a15f7f))
- Link the project wiki from the README - ([45ba560](https://git.teccave.de/tecbeat/mailboxd/commit/45ba560d6ec296c0e642ad0b9ddf2e056a84ff6d))
- Correct v2 blob storage description and document auto-migration - ([ad741de](https://git.teccave.de/tecbeat/mailboxd/commit/ad741de1ef120329c643ec204be1cecab913947f))

### ⚙️ Miscellaneous Tasks

- Add image-based integration and update tests - ([8adbb2d](https://git.teccave.de/tecbeat/mailboxd/commit/8adbb2d1fdd3f50b5914cb0bbd1305eacc60c451))
- Add dedicated migration test job to pipeline - ([9ea6918](https://git.teccave.de/tecbeat/mailboxd/commit/9ea6918efb05db488b704e5cb4ea886dab4bf355))

## [1.1.1](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.1.1) - 2026-08-20

### 🐛 Bug Fixes

- *(deps)* Pin rust crate tempfile to =3.27.0 - ([552f849](https://git.teccave.de/tecbeat/mailboxd/commit/552f8490e31d7d54c7967a08bd84a1685a32d617))

## [1.1.0](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.1.0) - 2026-08-20

### ⛰️  Features

- *(blob)* Port upstream redb-backed blob storage engine rewrite - ([9acfb0c](https://git.teccave.de/tecbeat/mailboxd/commit/9acfb0c9c3be02eff729195c511ac0d16c5ee587))
- *(import)* Port import module changes from upstream 2.0.1 - ([5a4b9c2](https://git.teccave.de/tecbeat/mailboxd/commit/5a4b9c203540e22ecd9ec621cfb335f40f04f7b8))
- *(migrate)* Adopt Bichon 2.x data volume on startup - ([976b520](https://git.teccave.de/tecbeat/mailboxd/commit/976b5209796b3b8da90b32772c061fe611a014e5))
- *(server)* Port REST API and build script from upstream 2.0.1 - ([64d0b59](https://git.teccave.de/tecbeat/mailboxd/commit/64d0b5953a2308523cf1fe7fc2cf66bce4a605e9))
- *(settings)* Add imap_timeout_seconds and archive dir from upstream 2.0.1 - ([74d2339](https://git.teccave.de/tecbeat/mailboxd/commit/74d2339461e6a5de7a4278c9dac95109e4c67b05))

### 🐛 Bug Fixes

- *(deps)* Update all non-major dependencies - ([8edafa8](https://git.teccave.de/tecbeat/mailboxd/commit/8edafa804ae4cf89c178e0111e81774df87b13f6))

### 🚜 Refactor

- *(admin)* Move migration engine from core to admin crate - ([5e164e9](https://git.teccave.de/tecbeat/mailboxd/commit/5e164e9ceb2171188d5c7f10c1a9c8861920765c))
- *(core)* Rename cache module to archive and overhaul IMAP sync - ([fd816bb](https://git.teccave.de/tecbeat/mailboxd/commit/fd816bb896bda39b82cb11e49dd90cc68762daf2))

### 📚 Documentation

- *(config)* Add Bichon-to-mailboxd migration guide to README FAQ - ([78885d3](https://git.teccave.de/tecbeat/mailboxd/commit/78885d34b0ebb83ce71f36ccbfc45c590f318c37))

### 🧪 Testing

- Mark network- and SMTP-integration tests as ignored - ([044ba4c](https://git.teccave.de/tecbeat/mailboxd/commit/044ba4c3c1398642318b2e6d6718743e34f28962))
- Fix rand 0.10 API and remove Bichon developer scratchpad tests - ([6bf0a23](https://git.teccave.de/tecbeat/mailboxd/commit/6bf0a23087b54e3504854fe0cbee17084a9d77ad))

### ⚙️ Miscellaneous Tasks

- *(deps)* Update Cargo.lock for 2.0.1 backend port - ([9f5f88a](https://git.teccave.de/tecbeat/mailboxd/commit/9f5f88aa5d64bc4361b51b5a5575df6dd1dd10bc))
- *(server)* Gate frontend build on embed-web feature so cargo_test runs without pnpm - ([2f3bd40](https://git.teccave.de/tecbeat/mailboxd/commit/2f3bd4090c4e255f6ac8c32c7a2803b184493f0f))
- Add cargo_test job to run workspace unit tests on every push - ([16d79ee](https://git.teccave.de/tecbeat/mailboxd/commit/16d79ee99cbf64c0217a17d6f0b2b520d8a7afa3))

## [1.0.8](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.0.8) - 2026-08-17

### 🐛 Bug Fixes

- *(deps)* Update rust:1.97.1-slim-bookworm docker digest to 2775a09 - ([be77534](https://git.teccave.de/tecbeat/mailboxd/commit/be7753412f6109ef68464d3fd98955b93689f15b))

## [1.0.7](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.0.7) - 2026-08-16

### 🐛 Bug Fixes

- *(deps)* Update all non-major dependencies - ([d86d9c3](https://git.teccave.de/tecbeat/mailboxd/commit/d86d9c30aa7b116e739e1b923969a5966fe812c1))
- *(deps)* Update rust:1.97.1-slim-bookworm docker digest to 158b745 - ([d03cb7e](https://git.teccave.de/tecbeat/mailboxd/commit/d03cb7e2ea8138d93eac636913ac28723c79eb22))

## [1.0.6](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.0.6) - 2026-08-09

### 🐛 Bug Fixes

- *(deps)* Update all non-major dependencies - ([6c61cfb](https://git.teccave.de/tecbeat/mailboxd/commit/6c61cfb5a056a0e1ae30625a5578b80024547dca))

## [1.0.5](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.0.5) - 2026-08-06

### 🐛 Bug Fixes

- *(deps)* Update rust:1.97.1-slim-bookworm docker digest to 96c0af8 - ([fc38ae8](https://git.teccave.de/tecbeat/mailboxd/commit/fc38ae881d504a4e095a9f6c5c9dc72c6f3666ad))
- *(deps)* Update debian:bookworm-slim docker digest to abd67ff - ([1ed0bf0](https://git.teccave.de/tecbeat/mailboxd/commit/1ed0bf081f4c8c354c81f63dbd56f4fa6ca381a1))

## [1.0.4](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.0.4) - 2026-08-04

### 🐛 Bug Fixes

- *(deps)* Drop reqwest 0.13, native_model 0.6, and too-new npm bumps - ([ffdf0da](https://git.teccave.de/tecbeat/mailboxd/commit/ffdf0daa60aa9c157cc778d32db9b6bf84456504))
- *(deps)* Update all non-major dependencies - ([336f3fb](https://git.teccave.de/tecbeat/mailboxd/commit/336f3fb5a1c3d4776d03895e21d61a7056ed3ef3))

### ⚙️ Miscellaneous Tasks

- *(deps)* Regenerate Cargo.lock after renovate consolidation - ([9c7af1c](https://git.teccave.de/tecbeat/mailboxd/commit/9c7af1c6fd6057cc2a7624328aab94b3d1bdd9e4))
- *(renovate)* Ignore reqwest >=0.13 and native_model >=0.5 - ([059389a](https://git.teccave.de/tecbeat/mailboxd/commit/059389a4debb24cb36ca369f636d9b71c6e0cf04))

## [1.0.3](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.0.3) - 2026-08-04

### 🐛 Bug Fixes

- *(deps)* Update node.js to d32cdf6 - ([5f62d42](https://git.teccave.de/tecbeat/mailboxd/commit/5f62d42b0d915555843f1ec4717bf84f4e7ac3a2))

## [1.0.2](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.0.2) - 2026-08-01

### 🐛 Bug Fixes

- *(deps)* Keep native_model at 0.4.20 (native_db 0.8.2 requires it) - ([8bb5f90](https://git.teccave.de/tecbeat/mailboxd/commit/8bb5f908afd8ab99a2d0889587de221c4b22a2eb))
- *(deps)* Update dependency typescript to v7 - ([b5c02b9](https://git.teccave.de/tecbeat/mailboxd/commit/b5c02b955fafc48720c4a94b801c2941b30500af))
- *(deps)* Update all non-major dependencies - ([06deef1](https://git.teccave.de/tecbeat/mailboxd/commit/06deef1af348f7df6d351a87663d4043ba9a6c2c))
- *(deps)* Pin dependencies - ([0153056](https://git.teccave.de/tecbeat/mailboxd/commit/015305637e361c3cbf8f486430521eb727d32229))
- *(deps)* Pin dependencies - ([181e89b](https://git.teccave.de/tecbeat/mailboxd/commit/181e89b4d5ea0d65ce058604eaccb86e7c2f11c1))

### ⚙️ Miscellaneous Tasks

- *(deps)* Regenerate Cargo.lock after renovate consolidation - ([2f0e3e9](https://git.teccave.de/tecbeat/mailboxd/commit/2f0e3e9d3a2609cd285c239f1ac36a43593d6f15))

## [1.0.1](https://git.teccave.de/tecbeat/mailboxd/-/releases/v1.0.1) - 2026-07-31

### 🐛 Bug Fixes

- *(dashboard)* Translate "no attachment data" empty state - ([21df9c1](https://git.teccave.de/tecbeat/mailboxd/commit/21df9c114260b6eff62d6261730b3acf1d464f91))
- *(settings)* Add missing i18n keys for access panel empty state - ([bf4a12a](https://git.teccave.de/tecbeat/mailboxd/commit/bf4a12a4b10674a40f1cacaf784c04c031533b7a))

### 📚 Documentation

- *(config)* Expand config.yml for full README rendering - ([5a4fae6](https://git.teccave.de/tecbeat/mailboxd/commit/5a4fae62f21569ea926c438336c8b38ed8b1d1cc))

### ⚙️ Miscellaneous Tasks

- Consolidate renovate updates + categorise config - ([19cf184](https://git.teccave.de/tecbeat/mailboxd/commit/19cf1840d9c2423af2adbabdab2b9fc22c71e1f2))

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
- *(core)* Add OIDC single sign-on client - ([98ad333](https://git.teccave.de/tecbeat/mailboxd/commit/98ad333ab907973d7d450e0696c98ec26ca64263))
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
- *(server)* Expose OIDC login, callback, handoff and public config - ([045a85d](https://git.teccave.de/tecbeat/mailboxd/commit/045a85d82cdfef38b7e2f578604598f53766ea78))
- *(settings)* Expose OIDC default_role_id and auto_redirect in system config - ([30290e9](https://git.teccave.de/tecbeat/mailboxd/commit/30290e94d21a7f3b3f8fba1e2eeefa2d7759d40c))
- *(smtp)* Implement built-in SMTP server for mail ingestion - ([4b0d571](https://git.teccave.de/tecbeat/mailboxd/commit/4b0d571cf2d05d811d69ec4ac469c0426d8bc5c6))
- *(ui)* Persist account table sorting to localStorage - ([41c3b84](https://git.teccave.de/tecbeat/mailboxd/commit/41c3b84e6578cad5b1b358ec6e2f284e8b454593))
- *(ui)* Sync search filters with URL and add dashboard navigation - ([2228e98](https://git.teccave.de/tecbeat/mailboxd/commit/2228e984105b3c52947bdcfad0b3a5b90ec10511))
- *(ui)* Add clickable logo to redirect to homepage #95 - ([c69ada3](https://git.teccave.de/tecbeat/mailboxd/commit/c69ada32efc7b62c30eb122675c560295f58abe5))
- *(ui)* Add quick page navigation to the email list pagination #85 - ([75cae51](https://git.teccave.de/tecbeat/mailboxd/commit/75cae51be9f8e8170db210902ca7e045f5498463))
- *(ui)* Add i18n support for profile dropdown - ([e8a1569](https://git.teccave.de/tecbeat/mailboxd/commit/e8a15695d8b75babdb02acdbb97ee0ccbae71729))
- *(version)* Use teccave pipeline VERSION for the runtime version string - ([9c3c897](https://git.teccave.de/tecbeat/mailboxd/commit/9c3c8977d696405fa40afbc7d5d56e17b6e17a71))
- *(web)* Add OIDC sign-in flow and secure token handoff - ([abcee5d](https://git.teccave.de/tecbeat/mailboxd/commit/abcee5dc15d637d33c508d0e9b708043c829a4ad))
- *(web)* Replace Bichon logo with tecbeat mailboxd icon - ([671519a](https://git.teccave.de/tecbeat/mailboxd/commit/671519a6abde4a6d2effcb7fed3565ef5b997b0c))
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
- *(docker)* Export MAILBOXD_VERSION so every crate embeds the correct version - ([3593895](https://git.teccave.de/tecbeat/mailboxd/commit/359389518a7f511a05c4b47675c988220969e9ef))
- *(docker)* Disable strict-dep-builds in pnpm install - ([a2ffbea](https://git.teccave.de/tecbeat/mailboxd/commit/a2ffbea538232cb4c5a3dbbda0a5b71d02703e45))
- *(docker)* Copy web/pnpm-workspace.yaml into web-builder stage - ([f03d068](https://git.teccave.de/tecbeat/mailboxd/commit/f03d068195a2319e8f0f2294a1ebef335d7dea2c))
- *(i18n, dashboard)* Internationalize recent activity chart dates - ([0fc83b6](https://git.teccave.de/tecbeat/mailboxd/commit/0fc83b693e713ac790e310041b83bbe1e74282e0))
- *(search)* Expose new SortBy variants via #[oai(rename)] - ([6873841](https://git.teccave.de/tecbeat/mailboxd/commit/6873841ba4c20094087050eb2ac6a77cfc4fa40f))
- *(server)* Correct ascii banner (m a i l b o x d) - ([db508d4](https://git.teccave.de/tecbeat/mailboxd/commit/db508d409ecd11b1f2566f585ab130cee95d2127))
- *(smtp)* Reject journaling attempts to non-local accounts - ([2f5de48](https://git.teccave.de/tecbeat/mailboxd/commit/2f5de48c6a6412ab75345a0f7ac26da4d7e6caca))
- *(smtp)* Don't clobber the IMAP-owned INBOX uid_validity on journal ingest - ([ce3f894](https://git.teccave.de/tecbeat/mailboxd/commit/ce3f8944a34db4fa45bcf77fac510633a4b4347e))
- *(sync)* Error in account sync task "TooNarrow" #38 - ([75d859a](https://git.teccave.de/tecbeat/mailboxd/commit/75d859abdf5e4f4444466011873e0c1590d64b34))
- *(sync)* Missing initial sync start time after enabling a previously disabled account #32 - ([4b13bf1](https://git.teccave.de/tecbeat/mailboxd/commit/4b13bf14c40e762979808421a33c13b12e1c7bd7))
- *(ui)* Fix sync folder selection jump issue; add auto-select children/parents and expand/collapse all folders button #21 - ([82397ab](https://git.teccave.de/tecbeat/mailboxd/commit/82397ab0cd75d30b48c74b83c61f2a75ed8cd3e1))
- *(ui)* Handle IMAP connection failure gracefully during folder sync #23 - ([1cfc123](https://git.teccave.de/tecbeat/mailboxd/commit/1cfc12324fc6895c87c8d6b4bdfa3516c70f69e6))
- *(ui)* Mailbox email details sheet zoom and close actions not working #5 - ([1f77ed9](https://git.teccave.de/tecbeat/mailboxd/commit/1f77ed91a74592a575898958a45345c491ea7746))
- *(ui)* Add a scrollbar when selecting a mailbox, as the mailbox list can be very long. #4 - ([ad02c76](https://git.teccave.de/tecbeat/mailboxd/commit/ad02c768495454fc227ee00b481d0e045a6391ea))
- *(ui, config)* Ensure use_dangerous status is visible in ui - ([2f3acdd](https://git.teccave.de/tecbeat/mailboxd/commit/2f3acdd75966db03190341562fc68579e6196b73))
- *(web)* Avoid doubled 'v' prefix in dashboard version footer - ([4f319fb](https://git.teccave.de/tecbeat/mailboxd/commit/4f319fb6d017c2192f9dfc0bcee97c95dbb44eb0))
- *(web)* Pin pnpm to 11.4.0 via packageManager and corepack prepare - ([b99eba8](https://git.teccave.de/tecbeat/mailboxd/commit/b99eba85b9e056aceecd137013232f63c4f7cc50))
- *(web)* Use undefined for empty use_proxy in emptyImap default - ([c01a0ed](https://git.teccave.de/tecbeat/mailboxd/commit/c01a0ed19e602c507f63b77f28780faa520d2b33))
- Replace remaining visible Bichon and RustMailer strings - ([6064faa](https://git.teccave.de/tecbeat/mailboxd/commit/6064faa46969e37b38b61b9b708b038a67bdb8f8))
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
- *(cargo)* Rename workspace crates bichon-* to mailboxd-* - ([1f9a77a](https://git.teccave.de/tecbeat/mailboxd/commit/1f9a77ae2422133e0d00f025f3f26cdc823a3b92))
- *(deploy)* Rename dockerfile, systemd unit, env template and changelog config to mailboxd - ([c3a3c74](https://git.teccave.de/tecbeat/mailboxd/commit/c3a3c74310b5c2ac941362ce8b61e67c5461fa29))
- *(migrate)* Use searchable_segment_ids() instead of reader() for merge #261 - ([26c14fc](https://git.teccave.de/tecbeat/mailboxd/commit/26c14fcaaf5b0f7ecedd81d8c70a7e347d23e67d))
- *(search)* Search filtering and sorting - ([b490923](https://git.teccave.de/tecbeat/mailboxd/commit/b490923e17229ac90473a0cba7d7ba57b6a7df01))
- *(server)* Rebrand env vars, cli flags, symbols and data paths to mailboxd - ([6a77ca5](https://git.teccave.de/tecbeat/mailboxd/commit/6a77ca597d2edef64178a688dcee652c9a91fdd7))
- *(web)* Rebrand webui strings, i18n, headers and source link to mailboxd - ([6055320](https://git.teccave.de/tecbeat/mailboxd/commit/60553201639be052f9bb86464d1a185be4ef7433))
- *(workspace)* Decompose project into multiple crates - ([0b866c8](https://git.teccave.de/tecbeat/mailboxd/commit/0b866c81ffbb05e1d6a93677febadadb75b0c0ef))
- Remove pro/enterprise feature-flag scaffolding - ([c5b1881](https://git.teccave.de/tecbeat/mailboxd/commit/c5b1881f53f363a7323e898ac5efa92cfc3edc8f))
- Replace autoconfig with native impl, remove openssl dependency - ([7afb1e2](https://git.teccave.de/tecbeat/mailboxd/commit/7afb1e29aa243294ddd22a4d0531bddba789acb9))
- Replace native_db with memdb and add tests - ([5406c43](https://git.teccave.de/tecbeat/mailboxd/commit/5406c4322c4556bae59b343589b1d06bb5d2b652))
- Decouple email body and attachment storage - ([a41b541](https://git.teccave.de/tecbeat/mailboxd/commit/a41b5417e3d9776d4610d0599d31bd2b1d5f6ddd))
- Use UUID for envelope id to prevent accidental deletion - ([d690f57](https://git.teccave.de/tecbeat/mailboxd/commit/d690f57290117304c9380c01b731a7f5cc7901bf))
- [**breaking**] Replace Tantivy search engine with DuckDB - ([d2936ed](https://git.teccave.de/tecbeat/mailboxd/commit/d2936ed4a71d9813a7032b450d3c1132e74b31e5))

### 📚 Documentation

- *(openapi)* Update attachment download param to content_hash  #314 - ([524201c](https://git.teccave.de/tecbeat/mailboxd/commit/524201c26de396cc7e05e2d527d64eab4df42240))
- Rewrite readme for mailboxd fork and add NOTICE with AGPL attribution - ([b401c73](https://git.teccave.de/tecbeat/mailboxd/commit/b401c732af81afc4551db62e399cde85ca69ead8))

### ⚡ Performance

- Optimize IMAP account fetch flow - ([83dd9cd](https://git.teccave.de/tecbeat/mailboxd/commit/83dd9cdd6be029d7c079cf49373e991f013c5365))
- Reduce tokio worker thread blocking to improve responsiveness on low-core machines - ([0792bb5](https://git.teccave.de/tecbeat/mailboxd/commit/0792bb546d3cf648ae1e08dc09233eb6f9e0e77b))

### 🎨 Styling

- *(web)* Shrink sidebar logo by 15% (60->51 open, 40->34 collapsed) - ([8ac706a](https://git.teccave.de/tecbeat/mailboxd/commit/8ac706acc4a53cc5730d4a5cf1611e02f6257f78))
- *(web)* Drop the icon above the sign-in card - ([4789910](https://git.teccave.de/tecbeat/mailboxd/commit/47899103cefccbffb9b8e07831fadc0209b2d559))

### 🧪 Testing

- *(blob)* Add concurrent access and crash recovery integration tests - ([b6957c8](https://git.teccave.de/tecbeat/mailboxd/commit/b6957c8ceb6a3420d4bb9557023903612cad136d))

### ⚙️ Miscellaneous Tasks

- *(blob)* Add bincode dependency and new meta error variants - ([cc54063](https://git.teccave.de/tecbeat/mailboxd/commit/cc540636694bb9a098c86902ec75645890a5c9ed))
- *(deps)* Replace async-imap with custom fork for imap-proto update - ([0c2e540](https://git.teccave.de/tecbeat/mailboxd/commit/0c2e54083472c8a17f98601fb7a63fda9fd6c024))
- *(docker)* Add root Dockerfile and enable teccave build_image pipeline - ([198f99b](https://git.teccave.de/tecbeat/mailboxd/commit/198f99b810a81e43b88bee9116d63e69a853e1f0))
- *(release)* Package bichonctl together with bichon binaries - ([0bf2003](https://git.teccave.de/tecbeat/mailboxd/commit/0bf2003670c6cd56aa2289cccddaebfd77b041fb))
- *(search ui)* Quick selection of year and month #39 - ([b35493e](https://git.teccave.de/tecbeat/mailboxd/commit/b35493e4e10de88b64040067338177665a362366))
- *(ui)* Add attachment file type icon - ([d334a23](https://git.teccave.de/tecbeat/mailboxd/commit/d334a23ca7f17243edbfbd3d1729b73ec7b6614d))
- *(ui)* Adjust width of account list for better visibility - ([000d144](https://git.teccave.de/tecbeat/mailboxd/commit/000d144e70555c4082432f5029c721c8e3015b3f))
- *(ui)* Move name field to step 2 #30 - ([b0ce767](https://git.teccave.de/tecbeat/mailboxd/commit/b0ce7671a85d37d309ca3c53f3a67ecbcd9e3510))
- Drop unused tool configs and consolidate docker layout - ([354273b](https://git.teccave.de/tecbeat/mailboxd/commit/354273b987da35efc36fe71fe5459ec7a7ea0315))
- Drop unused build/mailboxd.service systemd unit - ([c089252](https://git.teccave.de/tecbeat/mailboxd/commit/c08925242aa7005b30e6a04d483d03c0d8561c66))
- Rebuild image (cleanup canceled to preserve tag) - ([ef47d13](https://git.teccave.de/tecbeat/mailboxd/commit/ef47d13c95746715825ff3b26de0d2a0f96346d8))
- Merge teccave repository seed - ([fe51554](https://git.teccave.de/tecbeat/mailboxd/commit/fe51554b57883e7645b5f98b3b5239392205ca1c))
- Remove github-specific and rustmailer-legacy files - ([4923981](https://git.teccave.de/tecbeat/mailboxd/commit/49239810fab76621a25dadeb41d39b19929a5986))
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
