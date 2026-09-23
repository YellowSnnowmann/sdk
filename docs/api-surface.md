# API Surface

The SDK surface is grounded in the deployed Swagger/OpenAPI contract at
<https://api.tinyhumans.ai/swagger.json>. The spec reports TinyHumans API
`1.0.0` with 161 paths and 182 operations. The Rust SDK exposes one typed
method per public operation — **197 operations across the 21 namespaces
below**.
The remaining 32 administrative and 12 webhook-receiver operations are
intentionally excluded, including legacy routes whose summaries explicitly say
they are admin-only.

"Administrative" means *platform* administration. It does not cover operations
gated by a role within a resource the caller belongs to: `PUT /teams/{teamId}`,
`DELETE /teams/{teamId}/members/{userId}`, and
`PUT /teams/{teamId}/members/{userId}/role` say "(admin only)" in the contract,
but that is the **team-admin role** — held by every user over their own personal
team — not platform administrator rights. They are exposed on the `teams`
namespace. Teams were folded into users on the backend; the membership and
invite operations are deprecated in the contract and answer `410 Gone`.

"Webhook" exclusion means webhook *receivers* — the endpoints providers call
into (Stripe, Telegram, Discord, GitHub, Composio, Coinbase, Sentry, Twilio,
and the tunnel ingress paths). They carry no `bearerAuth`, are authenticated by
provider signature, and an SDK caller invoking one would be forging provider
traffic. The user-owned webhook *tunnel* CRUD under `/webhooks/core` is
ordinary bearer-authenticated user-facing API and is exposed as the `webhooks`
namespace.

| Namespace | Base path | Auth | Examples |
| --- | --- | --- | --- |
| `health` | `/` | none | `check()` liveness |
| `auth` | `/auth` | bearer | email login, OAuth, `me()`, integration tokens |
| `inference` | `/openai` | bearer | `GET /v1/models`, chat completions, responses, transcription |
| `agentIntegrations` | `/agent-integrations` | bearer | Composio, Parallel, media generation, maps, Apify, Twilio, crypto, OpenRouter |
| `apiKeys` | `/api-keys` | bearer | create, list, and revoke user API keys |
| `budgets` | `/budgets` | bearer | team budgets and seat allocations |
| `openCompany` | `/opencompany` | bearer | company instances, lifecycle, and custom domains |
| `payments` | `/payments` | bearer | Stripe, Coinbase, credits, transactions, plans |
| `feedback` | `/feedback` | bearer | create, ingest, list, detail, vote, and comments |
| `teams` | `/teams` | bearer | personal-team detail, usage, and billing; membership/invite routes are retired (`410`) |
| `channels` | `/channels` | bearer | messages, reactions, typing, threads |
| `mascots` | `/mascots` | mixed | catalog, render streams, meetings, Rive assets |
| `announcements` | `/announcements` | bearer | latest active announcement |
| `coupons` | `/coupons` | bearer | redemption and coupon history |
| `invite` | `/invite` | mixed | invite status, redemption, and owned codes |
| `referral` | `/referral` | bearer | referral stats and claim |
| `rewards` | `/rewards` | bearer | reward snapshot and Discord unlink |
| `redirect` | `/r` | none | resolve short redirect codes |
| `webhooks` | `/webhooks` | bearer | webhook tunnel CRUD and bandwidth budget |

The checked-in namespace manifest and Rust `PUBLIC_ROUTES` registry are
generated deterministically:

```bash
node scripts/sync-openapi.mjs
node scripts/sync-openapi.mjs --check
```

Most JSON responses use the hosted-backend envelope:

```json
{
  "success": true,
  "data": {}
}
```

SDK request helpers unwrap this envelope by default. The raw helper can return the
full response body when callers need status metadata or non-standard payloads.

## OpenRouter media generation

`agent_integrations::openrouter` exposes the direct OpenRouter proxy under
`/agent-integrations/openrouter/*`, including image (`POST /images`) and video
(`POST /videos`, `GET /videos/{jobId}`, `GET /videos/{jobId}/content`)
generation with untyped `impl Serialize` request bodies (OpenRouter's own API
is the contract for this surface).

`agent_integrations::openrouter_media` adds a fully typed alternative for the
media routes only — `OpenRouterImageRequest`/`OpenRouterImageResponse`,
`OpenRouterVideoRequest`, `ContentPartImage`, `FrameImage` — with an `extra`
flattened map on each request struct so an upstream field this module does not
yet model is still forwarded. Both modules call the same routes; pick whichever
fits the caller (`openrouter_images`/`openrouter_videos`/
`openrouter_image_models`/`openrouter_video_models` for typed,
`openrouter_create_image`/`openrouter_create_video`/
`list_openrouter_image_models`/`list_openrouter_video_models` for
passthrough). `get_openrouter_video`, `openrouter_video_content`, and the typed
module's `openrouter_video_content_with_type` (which also surfaces the
upstream `content-type`, since the plain byte helper drops response headers)
are shared by both.

`OpenRouterMediaModel` (returned by both the typed and untyped model listings)
carries OpenRouter's capability descriptors verbatim when the backend's
catalog published them: `supported_parameters`/`architecture` for image
models, and `supported_resolutions`/`supported_aspect_ratios`/
`supported_durations`/`supported_sizes`/`supported_frame_images`/
`generate_audio`/`seed`/`allowed_passthrough_parameters` for video models —
so a caller can validate a request against a model's real capabilities before
submitting it.

The older GMI-backed `agent_integrations::media_generation` module
(`/agent-integrations/media-generation/*`) is deprecated in favor of the
OpenRouter surface above; its methods are `#[deprecated]` but remain
functional.
