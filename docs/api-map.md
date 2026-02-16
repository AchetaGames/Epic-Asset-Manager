# Epic Games / Unreal Engine / Fab — API Reference

Captured from browser mitmproxy sessions (`web-flows.mitm`), Epic Games Launcher traffic
(`epic-flows.mitm`), and JS bundle static analysis on 2026-02-16.
Covers: unrealengine.com, epicgames.com, fab.com web flows + EGL launcher APIs.

---

## Authentication Mechanisms

### Cookie-Based Auth (unrealengine.com)

All UE web APIs use cookie-based session auth. The session is established via:

```
1. GET  epicgames.com/id/api/reputation       → sets XSRF-TOKEN cookie
2. POST epicgames.com/id/api/exchange          → sends exchangeCode + x-xsrf-token header
3. GET  epicgames.com/id/api/redirect?         → returns {sid, authorizationCode, redirectUrl}
4. GET  unrealengine.com/id/api/set-sid?sid=X  → 204, sets session cookies on .unrealengine.com
5. GET  unrealengine.com/api/cosmos/auth       → upgrades bearer token, sets EPIC_EG1 + EPIC_EG1_REFRESH
```

Step 4 (`set-sid`) sets base session cookies (`EPIC_BEARER_TOKEN`, `EPIC_SSO`, etc.) with
`Domain=.unrealengine.com; Path=/`. However, the Cosmos API requires **upgraded** tokens
(`EPIC_EG1`, `EPIC_EG1_REFRESH`) that are only issued when you call `/api/cosmos/auth` (step 5).
Without step 5, all `/api/cosmos/*` endpoints return `401 {"error":"Not logged in","isLoggedIn":false}`.

The cosmos/auth response includes `"upgradedBearerToken": true`, confirming the upgrade occurred.
It also sets `EPIC_CLIENT_SESSION` (path=/, httponly) for additional session tracking.

After step 5, the cookie jar contains:

| Cookie | Set By | Domain | Path | Purpose |
|--------|--------|--------|------|---------|
| `EPIC_BEARER_TOKEN` | set-sid | `.unrealengine.com` | `/` | Base auth token |
| `EPIC_SSO` | set-sid | `.unrealengine.com` | `/` | SSO session |
| `EPIC_SSO_RM` | set-sid | `.unrealengine.com` | `/` | "Remember me" SSO |
| `EPIC_DEVICE` | set-sid | `.unrealengine.com` | `/` | Device identifier |
| `EPIC_SESSION_AP` | set-sid | `.unrealengine.com` | `/id` | Session identifier |
| `XSRF-TOKEN` | set-sid | (request domain) | `/id` | CSRF protection |
| `EPIC_EG1` | cosmos/auth | `.unrealengine.com` | `/` | Upgraded JWT (24h expiry) |
| `EPIC_EG1_REFRESH` | cosmos/auth | `.unrealengine.com` | `/` | Refresh token (1yr expiry) |
| `EPIC_CLIENT_SESSION` | cosmos/auth | (request domain) | `/` | Client session (httponly) |

**Important**: `XSRF-TOKEN` and `EPIC_SESSION_AP` have `Path=/id`, so they're only sent to `/id/*` paths.
Account v2 endpoints require a separate `x-xsrf-token` header obtained via `POST /account/v2/refresh-csrf`.

### Auth Tiers

| Tier | Required | Used By |
|------|----------|---------|
| **Cosmos** | Upgraded session (EPIC_EG1 from `cosmos/auth` upgrade) | `/api/cosmos/*` |
| **Account v2** | Cosmos cookies + `x-xsrf-token` header | `/account/v2/*` |
| **Epic ID** | EPIC_SESSION_AP + XSRF-TOKEN | `epicgames.com/id/api/*` |
| **Fab Session** | Fab session cookie (from OAuth) + `X-CSRFToken` header | `fab.com/i/*` (authenticated) |
| **Public** | None | `fab.com/i/listings/search`, store browsing |

### Fab Auth Flow (OAuth2 Authorization Code Grant)

Fab uses standard OAuth2 authorization code grant with Epic Games as the identity provider.
**This is a browser-only flow** — it requires the user's `EPIC_SESSION_AP` cookie on `epicgames.com`,
which is set during a prior browser-based Epic login. There is no API-only way to complete this flow
without browser interaction.

**Why the exchange-code redirect doesn't work for Fab**: The exchange-code SSO flow
(`/id/exchange?exchangeCode=...&redirectUrl=...`) only works for domains in Epic's SSO domain list
(`unrealengine.com`, `unrealtournament.com`, `fortnite.com`, `epicgames.com`). `fab.com` is NOT in
that list — it uses standard OAuth2 instead.

#### Full redirect chain (captured from browser):

```
1. GET  fab.com/library
   → 302 Location: /social/login/epic/?next=/library
   (User is not logged in, redirected to login)

2. GET  fab.com/social/login/epic/?next=/library
   → 302 Location: https://www.epicgames.com/id/authorize?
       client_id=xyza7891REBVsEqSJRRNXmlS7EQHM459
       &redirect_uri=https://www.fab.com/social/complete/epic/?redirect_state={state}
       &state={state}
       &response_type=code
       &scope=basic_profile offline_access
       &prompt=select_account
   → Sets fab_sessionid cookie (Domain=.fab.com, HttpOnly, 90d expiry)

3. GET  epicgames.com/id/authorize?...
   → 200 HTML (Epic's authorize page loads)
   → Requires EPIC_SESSION_AP cookie (from prior Epic login)
   → Browser JS calls:

   3a. GET /id/api/authenticate → 200 (verifies session, refreshes EPIC_SESSION_AP)
   3b. GET /id/api/client/xyza7891REBVsEqSJRRNXmlS7EQHM459?redirectUrl=...&responseType=code&scope=...
       → 200 {clientName: "Fab", verified: true, allowedScopes: ["basic_profile", "offline_access"],
              thirdParty: true, logo: "https://media-cdn.epicgames.com/...", ...}
   3c. GET /id/api/account → 200 {id, email, displayName, switchable: true, ...}

4. GET  epicgames.com/id/api/redirect?
       redirectUrl=...fab.com/social/complete/epic/...
       &state={state}
       &responseType=code
       &clientId=xyza7891REBVsEqSJRRNXmlS7EQHM459
       &scope=basic_profile offline_access
       &prompt=select_account                        ← NOTE: causes 400 on first attempt!

   FIRST attempt → 400 {
     errorCode: "errors.com.epicgames.accountportal.account_select_required",
     message: "Please confirm you want to proceed with the currently logged in account."
   }

   User clicks "Continue" on account switch page, browser retries WITHOUT prompt=select_account:

   SECOND attempt → 200 {
     warning: "Do not share this code...",
     redirectUrl: "https://www.fab.com/social/complete/epic/?redirect_state={state}&state={state}&code={authCode}",
     authorizationCode: "{authCode}",        ← This is an AUTHORIZATION CODE, not an exchange code
     exchangeCode: null,
     sid: null
   }

5. GET  fab.com/social/complete/epic/?redirect_state={state}&state={state}&code={authCode}
   → 302 Location: /library
   → Sets fab_csrftoken (1yr expiry, SameSite=None)
   → Sets fab_sessionid (new value, 90d expiry, HttpOnly)
   (Fab's backend exchanges the auth code for tokens server-side)

6. GET  fab.com/library → 200 (authenticated, uses fab_sessionid cookie)
```

#### Key details:
- **Fab OAuth client ID**: `xyza7891REBVsEqSJRRNXmlS7EQHM459`
- **Scopes**: `basic_profile`, `offline_access`
- **Response type**: `code` (authorization code grant)
- **`prompt=select_account`**: Fab always sends this, which forces account selection confirmation.
  The first `/id/api/redirect` call returns 400 with `account_select_required`. After the user
  confirms, the second attempt succeeds (without the `prompt` param).
- **Session cookies set by Fab**:
  - `fab_sessionid` — HttpOnly, 90-day expiry, Domain=.fab.com
  - `fab_csrftoken` — 1-year expiry, Domain=.fab.com (used as `X-CSRFToken` header value)
- **Logout**: `POST fab.com/logout` → 302 to `/`, clears `fab_sessionid`

#### For EAM:
Since EAM already has OAuth2 tokens via egs-api, the **browser OAuth flow is unnecessary** for API
access. EAM should use the launcher auth path (`/e/`, `/p/egl/` endpoints with bearer tokens).
The "Open Fab" sidebar button simply opens `fab.com/library` in the user's default browser and
lets the browser handle the OAuth redirect naturally.

---

## Cosmos APIs

Base: `https://www.unrealengine.com/api/cosmos/`
Auth: Cookie-based (Cosmos tier)

### GET /api/cosmos/auth
Check and upgrade session authentication. **Must be called after `set-sid` before any other
Cosmos endpoint** — it upgrades the bearer token and issues `EPIC_EG1` / `EPIC_EG1_REFRESH`
JWTs required by all other `/api/cosmos/*` endpoints.

Response when session is valid:
```json
{
  "bearerTokenValid": true,
  "clearedOffline": false,
  "upgradedBearerToken": true,
  "accountId": "8645b4947bbc4c0092a8b7236df169d1"
}
```

Response when not authenticated:
```json
{"error": "Not logged in", "isLoggedIn": false}
```

Side effects (Set-Cookie):
- `EPIC_EG1` — Encrypted JWT, `Domain=.unrealengine.com; Path=/`, 24h expiry
- `EPIC_EG1_REFRESH` — Encrypted JWT, `Domain=.unrealengine.com; Path=/`, 1yr expiry
- `EPIC_CLIENT_SESSION` — Session token, `Path=/; HttpOnly`

### GET /api/cosmos/account
Get account details for current session.

```json
{
  "country": "CZ",
  "displayName": "Acheta Games",
  "email": "m***n@stastnej.ch",
  "id": "8645b4947bbc4c0092a8b7236df169d1",
  "preferredLanguage": "en",
  "cabinedMode": false,
  "isLoggedIn": true
}
```

### GET /api/cosmos/eula/accept?eulaId={id}&locale=en
Check if a specific EULA has been accepted.

**Known EULA IDs**: `unreal_engine`, `unreal_engine2`, `realityscan`, `mhc`, `content`
(from `/routing-rules` and `/account/v2/eula/acceptance-history`)

> **Note**: The acceptance-history shows the key as `unreal_engine2` (version 3), but
> `GET /api/cosmos/eula/accept?eulaId=unreal_engine` also returns `{accepted: true}`.
> The web UI POSTs to `unreal_engine2`. Both IDs appear to work for the GET check.

```json
{"accepted": true}
```

### POST /api/cosmos/eula/accept?eulaId={id}&locale=en&version={ver}
Accept a EULA. The web UI sends `eulaId=unreal_engine2&locale=en&version=3`.

```json
{"accepted": true}
```

### GET /api/cosmos/policy/aodc
Age of Digital Consent policy check.

```json
{"failed": false}
```

### GET /api/cosmos/communication/opt-in?setting={setting}
Check communication opt-in status for a specific setting.

**Known settings**: `email:ue` (Unreal Engine email). Likely also: `email:fn` (Fortnite), other per-product email settings.

```json
{"settingValue": false}
```

### GET /api/cosmos/search?query={q}&slug={slug}&locale={locale}&filter={filter}
Site-wide search across unrealengine.com content. Found in JS bundle `7069-*.js`.

Parameters:
- `query` — search text
- `slug` — content section/slug filter
- `locale` — locale string (e.g. `en`)
- `filter` — content type filter

Response: search results with `items[]` and `total` count. UI shows "N result(s)" via
`epic.cosmos.result` / `epic.cosmos.results` i18n keys. Results can be grouped.

> **Note**: Not captured in mitmproxy — discovered via JS bundle static analysis only.
> Exact response shape needs live verification.

### Cosmos endpoint summary

The Cosmos API is a thin account/session management layer specific to unrealengine.com.
It handles auth session upgrade, account info, EULA, policy checks, communication preferences,
and site search. The heavy lifting (asset catalog, downloads, Fab marketplace) lives on entirely
different API surfaces (`fab.com/i/*`, `catalog-public-service`, `launcher-public-service`, etc.).

All confirmed Cosmos endpoints at a glance:

| Method | Endpoint | Purpose |
|--------|----------|---------|
| `GET` | `/api/cosmos/auth` | Session upgrade (EPIC_EG1 JWTs) — **mandatory after set-sid** |
| `GET` | `/api/cosmos/account` | Account info (displayName, email, country, etc.) |
| `GET` | `/api/cosmos/eula/accept?eulaId={id}&locale={l}` | Check EULA acceptance |
| `POST` | `/api/cosmos/eula/accept?eulaId={id}&locale={l}&version={v}` | Accept EULA |
| `GET` | `/api/cosmos/policy/aodc` | Age of Digital Consent check |
| `GET` | `/api/cosmos/communication/opt-in?setting={s}` | Communication preference check |
| `GET` | `/api/cosmos/search?query=&slug=&locale=&filter=` | Site search (from JS bundle) |

---

## Account v2 APIs

Base: `https://www.unrealengine.com/account/v2/`
Auth: Cosmos cookies + `x-xsrf-token` header (obtained from `POST /account/v2/refresh-csrf`)

### Session & Auth

| Endpoint | Method | Response |
|----------|--------|----------|
| `/account/v2/refresh-csrf` | POST | `{success: true}` — refreshes XSRF token |
| `/account/v2/ajaxCheckLogin` | GET | `{needLogin: bool}` |
| `/account/v2/ajaxCheckLOA?profile=AAL2_MED_TIMEOUT` | GET | `{loaIsValid: bool, loaExpiresIn: number}` |
| `/account/v2/ajaxCheckUserHasPassword` | GET | `{hasPassword: bool}` |

### Account Info

| Endpoint | Method | Response |
|----------|--------|----------|
| `/account/v2/company-info` | GET | Company name, VAT, address |
| `/account/v2/api/email/info` | GET | Email, verified status, canUpdateEmail |
| `/account/v2/security/settings/ajaxGet` | GET | 2FA methods (sms, authenticator, email), enabled state |
| `/account/v2/security/settings/ajaxCheckAccountVerification` | GET | `{isAccountVerified: bool}` |
| `/account/v2/connections/socialConnection/ajaxGet?lang=en-US` | GET | Linked platforms (GitHub, Twitch, PSN, Xbox, Steam, etc.) |
| `/account/v2/retrieve-info/status` | GET | Account data retrieval/export status |
| `/account/v2/api/creator-programs/creator` | GET | Creator program enrollment |
| `/account/v2/parental-controls/get?lang=en-US` | GET | Parental control settings |
| `/account/v2/parental-controls/third-parties` | GET | Third-party app permissions |

### EULA

| Endpoint | Method | Response |
|----------|--------|----------|
| `/account/v2/eula/acceptance-history` | GET | All accepted EULAs with keys, versions, timestamps |
| `/account/v2/eula/acceptance-check?eulaId={id}&locale=en-US` | GET | `{accepted: bool, termKey: string}` |

Example acceptance-history entry:
```json
{
  "key": "unreal_engine2",
  "title": "Unreal® Engine End User License Agreement",
  "locale": "en-US",
  "version": 3,
  "revision": 1,
  "accountId": "8645b4947bbc4c0092a8b7236df169d1",
  "accepted": true,
  "responseTimestamp": "2026-02-16T00:17:53.162Z",
  "url": "https://cdn1.epicgames.com/eulatracking-download/unreal_engine2/en/v3/r1/012716d6c008247d5c3e8d1ac1cd3489.pdf"
}
```

Known EULA keys from capture: `unreal_engine2`, `epicgames_privacy_policy_no_table`, `egstore`, `fab_eula`, `wallet-terms`

### Communication

| Endpoint | Method | Response |
|----------|--------|----------|
| `/account/v2/api/communication/messages?lang=en-US` | GET | Communication preferences by namespace |
| `/account/v2/api/communication/emails?formatType=json` | GET | Email notification settings |

### Financial

| Endpoint | Method | Response |
|----------|--------|----------|
| `/account/v2/wallet/balance?locale=en-US` | GET | Wallet balance, currency, limits |
| `/account/v2/api/wallet/fortnite` | GET | V-Bucks balance |
| `/account/v2/payment/purchaseToken?locale=en-US` | POST | `{purchaseToken: string}` |
| `/account/v2/payment/ajaxGetOrderHistory?count=10&sortDir=DESC&sortBy=DATE&locale=en-US` | GET | Order history with items, amounts |
| `/account/v2/payment/ajaxGetCodeRedemptionHistory?page=0` | GET | Redeemed codes with descriptions |
| `/account/v2/payment/vbucks-card-code-redemption-history?locale=en-US&page=0` | GET | V-Bucks card history |
| `/account/v2/api/transactions/in-island?locale=en-US&sortBy=DATE&sortDir=DESC&limit=10` | GET | In-game transactions |
| `/account/v2/gift/sent-gifts?start=0&count=5&locale=en-US` | GET | Sent gifts (paginated) |
| `/account/v2/gift/received-gifts?start=0&count=5&locale=en-US` | GET | Received gifts (paginated) |
| `/account/v2/subscription/orders?start=0&count=10&locale=en-US` | GET | Subscription orders |
| `/account/v2/subscription/subscriptions?start=0&count=5&locale=en-US` | GET | Active subscriptions |
| `/account/v2/subscription/platform-status?namespace=fn&locale=en-US` | GET | Platform subscription status |
| `/account/v2/order/refund-reasons` | GET | Refund reason enum |
| `/account/v2/reward-account?currency=CZK&locale=en-US` | GET | Rewards balance |
| `/account/v2/reward-account/transactions?count=10&locale=en-US` | GET | Rewards history |

### Redemption

| Endpoint | Method | Response |
|----------|--------|----------|
| `/account/v2/ajax/redemption/validate-redemption-code` | POST | Code validation result |

---

## Epic ID APIs

Base: `https://www.epicgames.com/id/api/`
Auth: EPIC_SESSION_AP cookie + XSRF-TOKEN header (for authenticated endpoints)

### Public (no auth)

| Endpoint | Method | Response |
|----------|--------|----------|
| `/id/api/reputation` | GET | Sets XSRF-TOKEN cookie, anti-fraud signal |
| `/id/api/location` | GET | `{country, city, coordinates: {latitude, longitude, time_zone}}` |
| `/id/api/i18n?ns={namespace}` | GET | Localization strings (e.g. `ns=messages`, `ns=epic-consent-dialog`) |
| `/id/api/analytics` | GET | `{trackingUuid, accountId, loginId}` |

### Authenticated

| Endpoint | Method | Response |
|----------|--------|----------|
| `/id/api/authenticate` | GET | 204 if session valid (silent auth check). Also refreshes `EPIC_SESSION_AP` cookie. |
| `/id/api/client/{clientId}?redirectUrl=...&responseType=code&scope=...` | GET | OAuth client info: clientName, logo, allowedScopes, verified, thirdParty, eulas |
| `/id/api/login` | POST | Login with email+password+captcha. Returns 431 + MFA metadata if 2FA required |
| `/id/api/login/mfa` | POST | Submit MFA code (`{code, method, rememberDevice}`) |
| `/id/api/exchange` | POST | Exchange token for session (`{exchangeCode}` + x-xsrf-token) |
| `/id/api/redirect?redirectUrl=...` | GET | For SSO domains: returns `{sid}`. For OAuth clients (e.g. Fab): pass `responseType=code&clientId=...&scope=...` to get `{authorizationCode, redirectUrl}`. See Fab auth flow for `prompt=select_account` behavior. |
| `/id/api/account` | GET | Full account: id, email, displayName, country, MFA status, hasPassword, switchable |
| `/id/api/external` | GET | Linked external accounts (Facebook, PSN, Xbox, Steam, etc.) |
| `/id/api/user-info` | GET | `{previousLogin, numOutfits, numFriends}` |

### Login / Authorize Error Codes

| HTTP | errorCode | Meaning |
|------|-----------|---------|
| 431 | `errors.com.epicgames.common.two_factor_authentication.required` | MFA needed — check `metadata.twoFactorMethod` |
| 400 | `errors.com.epicgames.accountportal.account_review_details_required` | Account review needed before redirect |
| 400 | `errors.com.epicgames.accountportal.account_select_required` | Must confirm account selection (caused by `prompt=select_account`). Retry without `prompt` param after user confirms. |

---

## Fab APIs

Base: `https://www.fab.com/i/`
Auth: `fab_sessionid` cookie (from OAuth flow) + `X-CSRFToken` header (value from `fab_csrftoken` cookie or `GET /i/csrf`)

Many Fab endpoints work for both authenticated and unauthenticated users.
Auth adds personalized data (ownership, wishlist, wallet).

### Session & User

| Endpoint | Method | Auth | Response |
|----------|--------|------|----------|
| `/i/csrf` | GET | none | `{}` — sets `fab_csrftoken` cookie |
| `/i/browser-support` | GET | none | `{support: "regular"}` |
| `/i/users/context` | GET | csrf | Country, currency, feature flags (works both logged-in and logged-out) |
| `/i/users/me/wallet` | GET | session | `{balance, currency}` |
| `/i/users/me/consents` | GET | session | User consent records (returns `[]` if none) |
| `/i/cart` | GET | session | Shopping cart items with pricing |
| `POST /logout` | POST | session | 302 → `/`, clears `fab_sessionid` cookie |

### Store Browsing

| Endpoint | Method | Response |
|----------|--------|----------|
| `/i/banners/ongoing/store` | GET | Active store banners |
| `/i/channels/{slug}` | GET | Channel info (e.g. `unreal-engine`: uid, name, icon, description, cover image) |
| `/i/layouts/{slug}` | GET | Channel layout with blade sections and listing tiles |
| `/i/blades/free_content_blade` | GET | Free content section with listings |

### Listing Search & Details

| Endpoint | Key Params | Response |
|----------|------------|----------|
| `/i/listings/search` | `channels`, `listing_types`, `categories`, `sort_by`, `count`, `cursor`, `aggregate_on`, `in=wishlist`, `is_discounted` | Paginated results with aggregations |
| `/i/listings/prices-infos?offer_ids=...` | Multiple offer_ids | Bulk pricing: currencyCode, price, discount, VAT |
| `/i/listings/{uid}` | — | Full listing detail (formats, rating, category, seller, images, description) |
| `/i/listings/{uid}/asset-formats/unreal-engine` | — | UE-specific: distribution method, tech details, platforms |
| `/i/listings/{uid}/prices-infos` | — | Per-listing pricing |
| `/i/listings/{uid}/ownership` | — | License types the user owns |
| `/i/store/listings/{uid}/reviews?ratings=...&sort_by=...` | — | Reviews with pagination |

#### Search Aggregation Types
- `category_per_listing_type` — category counts grouped by listing type
- `channel` — counts per channel
- `listing_type` — counts per type

#### Sort Options
- `-relevance` (default), `-createdAt`, `createdAt`, `-price`, `price`

#### Listing Types
`3d-model`, `material`, `audio`, `code-plugin`, `environment`, `game-template`, `hdri`, `decal`, `blueprint`, `music`

### Taxonomy & Metadata

| Endpoint | Response |
|----------|----------|
| `/i/taxonomy/licenses` | All license types (UE Marketplace License, Standard, Personal, Professional, CC-BY, etc.) |
| `/i/taxonomy/asset-format-groups` | Format groups: MetaHuman, Unreal Engine, Unity, USDZ, FBX, etc. |
| `/i/tags/groups` | Tag categories: rendering style, theme, genre, etc. |
| `/i/tags/slug/{slug}` | Single tag lookup (name, uid) |
| `/i/unreal-engine/versions` | All UE versions: `["UE_4.0", ..., "UE_5.5"]` |
| `/i/sellers/name/{name}/trader-info` | Seller verification: trader status, business info, contact |

### Library (Authenticated)

These endpoints return the user's owned assets.

| Endpoint | Key Params | Response |
|----------|------------|----------|
| `/i/library/entitlements/search` | `sort_by`, `cursor`, `listing_types`, `categories`, `tags`, `licenses`, `asset_formats`, `added_since`, `source=acquired`, `aggregate_on`, `count=0` (aggregation only) | Paginated entitlements |
| `/i/library/assets/{uid}/asset-formats` | — | Download format details for owned asset |
| `/i/users/me/listings-states/{uid}` | — | `{acquired, entitlementId, wishlisted, ownership, review}` |
| `/i/users/me/listings-states?listing_ids=...` | Multiple IDs | Bulk state check |

#### Entitlement Result Shape
```json
{
  "capabilities": {"addByVerse": false, "requestDownloadUrl": true},
  "createdAt": "2025-05-13T15:40:23.894243+00:00",
  "licenses": [{"name": "Standard License", "slug": "standard", "url": "/eula"}],
  "listing": {
    "assetFormats": [{
      "assetFormatType": {"code": "unreal-engine", "icon": "unreal-engine", "name": "Unreal Engine"},
      "technicalSpecs": {
        "technicalDetails": "<p>HTML tech details...</p>",
        "unrealEngineEngineVersions": ["UE_4.14", "UE_4.18", ...],
        "unrealEngineTargetPlatforms": ["Windows", "Mac", ...],
        "unrealEngineDistributionMethod": "asset_pack"
      }
    }],
    "isMature": false,
    "lastUpdatedAt": "...",
    "title": "...",
    "uid": "..."
  }
}
```

#### Asset Format Files (for download)
```json
[{
  "assetFormatType": {"code": "unreal-engine", "extensions": ["uasset", "uproject"]},
  "files": [{
    "fileSize": null,
    "name": "1.0.0",
    "uid": "ba701ff2-7399-49b3-80b9-4e4fd3ab9e4a",
    "assetType": null,
    "engineVersions": null
  }]
}]
```

#### Pagination
Fab uses cursor-based pagination:
```json
{
  "cursors": {"next": "cD0yMDI0LTExLTAz...", "previous": "cj0xJnA9MjAyNS0w..."},
  "next": "https://www.fab.com/i/library/entitlements/search?cursor=...&sort_by=-createdAt",
  "previous": "https://www.fab.com/i/library/entitlements/search?cursor=...&sort_by=-createdAt"
}
```

---

## Payment APIs

Base: `https://payment-website-pci.ol.epicgames.com/v2/`
Auth: EPIC_BEARER_TOKEN + EPIC_SSO cookies

| Endpoint | Method | Response |
|----------|--------|----------|
| `/v2/payment-management?purchaseToken=...` | GET | Payment management page (HTML) |
| `/v2/purchase/payment-methods` | GET | Saved payment methods (PayPal, credit card, etc.) |
| `/v2/purchase/get-payment-client-token` | POST | Braintree client token for payment processing |

---

## Implementation Plan: egs-api vs EAM

### Belongs in `egs-api` (generic, reusable API client)

**High priority** (blocks EAM features):

| Endpoint | Proposed Method | Notes |
|----------|----------------|-------|
| `GET /api/cosmos/auth` | `cosmos_auth_upgrade()` | **Mandatory** after set-sid — upgrades bearer token, sets EPIC_EG1 JWTs |
| `GET/POST /api/cosmos/eula/accept` | `validate_eula(id)` / `accept_eula(id, version)` | Eliminates EAM's raw EpicWeb EULA code |
| `/i/listings/search` | `fab_search(filters)` | With filters, cursor pagination, aggregations |
| `/i/listings/{uid}` | `fab_listing(uid)` | Full listing detail |
| `/i/listings/{uid}/asset-formats/unreal-engine` | `fab_listing_formats(uid)` | UE-specific tech details |
| `/i/users/me/listings-states/{uid}` | `fab_listing_state(uid)` | Ownership/wishlist per listing |
| `/i/users/me/listings-states?listing_ids=...` | `fab_listing_states_bulk(ids)` | Bulk ownership check |
| `/e/accounts/{id}/ue/library` | `fab_egl_library(account_id)` | Launcher-style library (OAuth2 bearer, cursor pagination) |
| `/p/egl/listings/{uid}/asset-formats/{fmt}/files/{fid}/download-info` | `fab_egl_download_info(uid, fmt, fid)` | Get presigned download URL (OAuth2 bearer) |

**Medium priority** (cleaner architecture):

| Endpoint | Proposed Method | Notes |
|----------|----------------|-------|
| `GET /api/cosmos/account` | `cosmos_account()` | Account info via Cosmos |
| `GET /api/cosmos/communication/opt-in` | `cosmos_comm_opt_in(setting)` | Communication preference check |
| `GET /api/cosmos/policy/aodc` | `cosmos_policy_aodc()` | Age of Digital Consent check |
| `GET /api/cosmos/search` | `cosmos_search(query, slug, locale, filter)` | Site search (low value for EAM, but completes the API surface) |
| `/i/listings/prices-infos?offer_ids=...` | `fab_bulk_prices(offer_ids)` | Bulk pricing |
| `/i/listings/{uid}/prices-infos` | `fab_listing_prices(uid)` | Per-listing pricing |
| `/i/taxonomy/licenses` | `fab_licenses()` | Static, cacheable |
| `/i/taxonomy/asset-format-groups` | `fab_format_groups()` | Static, cacheable |
| `/i/tags/groups` | `fab_tags()` | Static, cacheable |
| `/i/unreal-engine/versions` | `fab_ue_versions()` | Static, cacheable |
| `/i/listings/{uid}/ownership` | `fab_listing_ownership(uid)` | License types user owns |
| `GET /api/blobs/{platform}` | `engine_versions(platform)` | Replaces EAM's direct blobs query |

**Low priority** (consolidation):

| Endpoint | Proposed Method | Notes |
|----------|----------------|-------|
| `/i/csrf` + `/i/users/context` | `fab_init_session()` | Fab session bootstrap |
| Full SID flow (XSRF → exchange → redirect → set-sid → cosmos/auth) | Encapsulate in `auth_sid()` | Currently split between egs-api and EAM's EpicWeb |

**End goal**: EAM's `EpicWeb` struct can be **eliminated entirely** — its three
responsibilities (session setup, EULA, engine versions) all move into egs-api.

### Stays in EAM (application-specific)

- **Account v2 settings/security** — 2FA, email, password, parental controls, linked platforms (account portal)
- **Account v2 financial** — wallet, order history, gifts, subscriptions, rewards, V-Bucks
- **Account v2 EULA acceptance-history** — Cosmos check/accept is sufficient
- **Account v2 communication prefs** — email opt-in/notification settings
- **Account v2 code redemption** — niche, low priority
- **Payment APIs** (`payment-website-pci.ol.epicgames.com`) — PCI-scoped, never in a generic client
- **Fab store layout** — `/i/channels/`, `/i/layouts/`, `/i/blades/`, `/i/banners/` (CMS/presentation)
- **Fab reviews** — `/i/store/listings/{uid}/reviews` (nice-to-have UI detail)
- **Fab cart/wallet/consents** — purchase flow, not relevant for asset management
- **Fab seller info** — `/i/sellers/name/{name}/trader-info` (store display detail)
- **Epic ID helpers** — `/id/api/location`, `/id/api/i18n`, `/id/api/analytics` (browser-oriented)
- **Tracking/Talon** — telemetry, never in a library
- **Publishing Portal** — separate product
- **Routing Rules** — internal infrastructure

---

## Launcher / EGL APIs (OAuth2 Bearer Token)

Captured from `epic-flows.mitm` — actual Epic Games Launcher traffic. These APIs use OAuth2 bearer
tokens (not browser cookies). The launcher authenticates via `client_credentials` or `refresh_token`
grants and passes `Authorization: bearer eg1~...` on all requests.

### Auth Context

| Field | Value |
|-------|-------|
| **Client ID** | `34a02cf8f4414e29b15921876da36f9a` |
| **Grant types** | `client_credentials` (app-only), `refresh_token` (user) |
| **Token type** | `eg1` (JWT) |
| **Access token TTL** | 14,400s (4h) for client_credentials, 129,600s (36h) for refresh_token |
| **Refresh token TTL** | 31,540,000s (~1yr) |
| **User-Agent** | `UELauncher/{version}+++Portal+Release-Live Windows/{os_version}` |

**Important**: The launcher **never uses** the cookie-based set-sid/cosmos flow. It uses pure OAuth2
bearer tokens for everything, including Fab library access. The cookie flow is browser-only.

### Account Service

Base: `https://account-public-service-prod03.ol.epicgames.com/account/api/`

| Endpoint | Method | Response |
|----------|--------|----------|
| `/oauth/token` | POST | OAuth2 token exchange. Body: `grant_type=client_credentials&token_type=eg1` or `grant_type=refresh_token&refresh_token=eg1~...`. Auth: Basic (base64 client_id:secret) |
| `/oauth/verify?includePerms=true` | GET | Verify current token, returns account info + permissions |
| `/public/account/{accountId}` | GET | Full account: id, displayName, name, email, failedLoginAttempts, lastLogin, numberOfDisplayNameChanges, etc. |
| `/public/account/{accountId}/externalAuths` | GET | Linked platforms: facebook, github, twitch, psn, xbl, steam, etc. with external IDs |
| `/public/account?accountId=...&accountId=...` | GET | Bulk account lookup (multiple accountId params) |
| `/epicdomains/ssodomains` | GET | `["unrealengine.com", "unrealtournament.com", "fortnite.com", "epicgames.com"]` |

### Launcher Service

Base: `https://launcher-public-service-prod06.ol.epicgames.com/launcher/api/public/`

| Endpoint | Method | Response |
|----------|--------|----------|
| `/assets/v2/platform/{platform}/launcher?label={label}&clientVersion={ver}&machineId={id}` | GET | Launcher self-update manifest. Returns `{elements: [{appName, buildVersion, hash, ...}]}` |
| `/assets/{platform}?label=Live` | GET | All available app assets for platform. Returns array of `{appName, buildVersion, catalogItemId, namespace, ...}` |
| `/payment/accounts/{accountId}/billingaccounts/default` | GET | `{billingAccountName, country, countrySource, currency}` |

### Catalog Service

Base: `https://catalog-public-service-prod06.ol.epicgames.com/catalog/api/shared/`

| Endpoint | Method | Response |
|----------|--------|----------|
| `/currencies?start=0&count=100` | GET | All supported currencies with symbols, decimals, price ranges |
| `/namespace/{ns}/items?status=SUNSET\|ACTIVE&sortBy=creationDate&country=US&locale=en&start=0&count=1000` | GET | Items in namespace. Namespaces: `poodle` (Twinmotion), `ecc92c9d...` (RealityScan), `ue` (UE) |
| `/namespace/{ns}/offers?status=ACTIVE&locale=en&start=0&count=100` | GET | Active offers in namespace |
| `/bulk/namespaces/items?country=US&locale=en` | POST | Bulk item lookup across namespaces. Returns `{"ns:itemId": {...}, ...}` |

### Entitlement Service

Base: `https://entitlement-public-service-prod08.ol.epicgames.com/entitlement/api/`

| Endpoint | Method | Response |
|----------|--------|----------|
| `/account/{accountId}/entitlements?start=0&count=1000` | GET | All entitlements: `[{id, entitlementName, namespace, catalogItemId, ...}]` |

### Library Service

Base: `https://library-service.live.use1a.on.epicgames.com/library/api/public/`

| Endpoint | Method | Response |
|----------|--------|----------|
| `/items?includeMetadata=true` | GET | User's library items with metadata. Cursor-based: `{responseMetadata: {nextCursor, stateToken}, records: [...]}` |
| `/stateToken/{token}/status` | GET | Check if a state token is still valid: `{valid: bool}` |

### Friends Service

Base: `https://friends-public-service-prod06.ol.epicgames.com/friends/api/public/`

| Endpoint | Method | Response |
|----------|--------|----------|
| `/friends/{accountId}?includePending=true` | GET | Friends list: `[{accountId, status, direction, created, favorite}]` |

### Price Engine

Base: `https://priceengine-public-service-ecomprod01.ol.epicgames.com/priceengine/api/shared/`

| Endpoint | Method | Response |
|----------|--------|----------|
| `/offers/price` | POST | Bulk price lookup with tax calculation |

### Order Processor

Base: `https://orderprocessor-public-service-ecomprod01.ol.epicgames.com/orderprocessor/api/shared/`

| Endpoint | Method | Response |
|----------|--------|----------|
| `/accounts/{accountId}/orders/quickPurchase?country={cc}&locale={l}` | POST | Quick purchase: `{quickPurchaseStatus: "CHECKOUT"}` |

### Presence Service

Base: `https://presence-public-service-prod.ol.epicgames.com/presence/api/v1/`

| Endpoint | Method | Response |
|----------|--------|----------|
| `/_/{accountId}/presence/{presenceId}` | PATCH | Update online status: `{own: {status: "online", perNs: [...]}}` |

### Lightswitch (Service Status)

Base: `https://lightswitch-public-service-prod06.ol.epicgames.com/lightswitch/api/`

| Endpoint | Method | Response |
|----------|--------|----------|
| `/service/bulk/status?serviceId=Jasper` | GET | `[{serviceInstanceId, status: "UP", message, allowedActions: ["PLAY","DOWNLOAD"]}]` |

### Other Launcher Endpoints

| Endpoint | Method | Response |
|----------|--------|----------|
| `cdn.quixel.com/fab/manifest/prod/plugins_v1.json` | GET | Plugin manifest: Blender, Cinema 4D, 3ds Max, Revit versions + download URLs |
| `datarouter.ol.epicgames.com/datarouter/api/v1/public/data` | POST | Telemetry/analytics data submission |

---

## Fab EGL APIs (Launcher-Authenticated)

These are Fab endpoints called by the Epic Games Launcher with OAuth2 bearer tokens.
They use a different URL pattern (`/e/` and `/p/egl/`) than the browser Fab APIs (`/i/`).

### GET /e/accounts/{accountId}/ue/library

User's UE asset library — the launcher's primary library endpoint.

Auth: `Authorization: bearer eg1~...`
Params: `?count=100` (pagination), `?cursor=...` (next page)

```json
{
  "cursors": {"next": "ZW1wPTI3JmZhYj0yNyZ1ZXM9Mjc="},
  "results": [
    {
      "assetId": "28166226c38a4ff3aa28bbe87dcbbe5b",
      "assetNamespace": "89efe5924d3d467c839449ab6ab52e7f",
      "categories": [{"id": "b66dbb5d-...", "name": "Forest & Jungle"}],
      "description": "Open World Demo Collection",
      "listingType": "3D",
      "seller": "Epic Games",
      "distributionMethod": "ASSET_PACK",
      "images": [{"md5": null, "type": "Featured", "url": "https://media.fab.com/..."}]
    }
  ]
}
```

Cursor is base64-encoded offset marker. `cursors.next` = null when no more pages.

### GET /p/egl/listings/{uid}/asset-formats/{format}/files/{fileId}/download-info

Get presigned download URL for an asset file.

Auth: `Authorization: bearer eg1~...`

```json
{
  "downloadInfo": [
    {
      "assetFormat": "asset-format/3d-exchange/fbx",
      "downloadUrl": "https://emp-fastly-stitched.epicgamescdn.com/Builds/Org/.../file.zip?f_token=...",
      "expires": "2026-02-14T22:41:56.599Z",
      "type": "binary"
    }
  ]
}
```

The download URL uses `emp-fastly-stitched.epicgamescdn.com` with a signed `f_token` parameter.
Downloads support `Range` requests (HTTP 206).

### Two Auth Paths — Summary

| Path | Auth | Fab Prefix | Used By |
|------|------|------------|---------|
| **Browser** | Cookie session + X-CSRFToken | `/i/` | fab.com website |
| **Launcher/API** | OAuth2 bearer `eg1~` | `/e/` (library), `/p/egl/` (downloads) | EGL, EAM, egs-api |

EAM should use the launcher path (`/e/`, `/p/egl/`) since it already has OAuth2 tokens via egs-api.
The browser path (`/i/`) requires the full Fab OAuth2 redirect dance and is unnecessary for EAM.

---

## Other Endpoints

### Linux Engine Downloads

`GET https://www.unrealengine.com/en-US/linux` — HTML page with presigned S3 download links.

The page embeds presigned AWS S3 URLs pointing to `ucs-blob-store.s3-accelerate.amazonaws.com/blobs/{id}`.
URLs use AWS4-HMAC-SHA256 signing with a 1-hour expiry. The download links are generated server-side
when the page is rendered (requires Cosmos auth cookies).

Three categories of downloads per UE version:

| Category | Filename Pattern | Example |
|----------|-----------------|---------|
| **Unreal Engine** | `Linux_Unreal_Engine_{version}.zip` | `Linux_Unreal_Engine_5.7.3.zip` |
| **Fab Plugin** | `Linux_Fab_{ue_ver}_{fab_ver}.zip` | `Linux_Fab_5.7.0_0.0.7.zip` |
| **Bridge Plugin** | `Linux_Bridge_{ue_ver}_{bridge_ver}.zip` | `Linux_Bridge_5.7.0_2025.0.1.zip` |

Available versions as of 2026-02-16:
- **Engine**: 5.1.0, 5.1.1, 5.2.0, 5.2.1, 5.3.0–5.3.2, 5.4.0–5.4.4, 5.5.0–5.5.4, 5.6.0–5.6.1, 5.7.0–5.7.3
- **Fab**: 5.3.0–5.7.0 (multiple sub-versions per UE version)
- **Bridge**: 5.1.0–5.7.0

> **Note**: EAM currently fetches engine versions via `GET /api/blobs/linux` (a JSON API).
> This HTML page is an alternative source. The blobs API may return the same data in a
> machine-readable format — needs comparison.

### Routing Rules
`GET https://www.unrealengine.com/routing-rules` returns regex rules mapping URL paths to backend origins.
The `ue-cosmos-website` origin handles: `/eulacheck/*`, `/agreements/*`, and various news/blog pages.

### Tracking
- `tracking.epicgames.com/track.png?...` — pixel-based event tracking
- `tracking.unrealengine.com/track.png?...` — UE-specific tracking
- `talon-service-prod.ecosec.on.epicgames.com/v1/*` — anti-fraud/bot detection (Talon)

### Publishing Portal
`GET https://publish.unrealengine.com/` → 302 → `/v3` → 302 → `/publishing-portal` (HTML app)
Uses same cookie auth as unrealengine.com.
