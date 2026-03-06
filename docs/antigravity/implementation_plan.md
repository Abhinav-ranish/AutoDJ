# MyTune — Implementation Plan

Build a production-ready, AI-driven music learning platform from scratch in the `c:\Users\sidds\OneDrive\Documents\GitHub\MyTune` monorepo. The workspace is currently empty (only a README). We will scaffold the full system in parallel across frontend, backend, AI, and DevOps layers.

## User Review Required

> [!IMPORTANT]
> This is a greenfield build. All code will be created from scratch. Please confirm:
> 1. Do you have a Supabase project already, or should we scaffold the config with placeholder credentials?
> 2. Do you have Stripe keys, or should placeholders be used?
> 3. Which AI/LLM provider should the persona explanation system use (OpenAI, Anthropic, or Google Gemini)?
> 4. Should the embedding/vector pipeline use **pgvector** (Supabase built-in) or an external service (Pinecone/FAISS)?

> [!NOTE]
> The plan below assumes: **pgvector** for embeddings, **OpenAI** for LLM explanations, **placeholder credentials** for Supabase/Stripe. These can be swapped at config time.

---

## Proposed Changes

### Monorepo Root

#### [NEW] Project root files
- `package.json` — workspace root with scripts
- `.env.example` — all environment variables listed
- `.github/workflows/ci.yml` — GitHub Actions CI
- `README.md` — full project README

---

### Frontend (`/frontend`)

#### [NEW] Setup
- Vite + React + TypeScript project
- TailwindCSS v3 + `tailwind.config.ts`
- React Router v6
- Supabase JS client (`@supabase/supabase-js`)
- PostHog SDK (`posthog-js`)
- Stripe.js (`@stripe/stripe-js`)

#### [NEW] Pages
| File | Purpose |
|------|---------|
| `src/pages/LandingPage.tsx` | Hero, CTA, features showcase |
| `src/pages/AuthPage.tsx` | Login / signup (Supabase Auth) |
| `src/pages/OnboardingQuiz.tsx` | Instrument, level, genres, goals |
| `src/pages/RoadmapPage.tsx` | Personalized roadmap display |
| `src/pages/LessonPlayer.tsx` | YouTube embed + exercises |
| `src/pages/Dashboard.tsx` | Streaks, progress, badges |
| `src/pages/SettingsPage.tsx` | Profile + subscription management |

#### [NEW] Core Components
| File | Purpose |
|------|---------|
| `src/components/Navbar.tsx` | Top nav |
| `src/components/RoadmapCard.tsx` | Module card in roadmap |
| `src/components/LessonCard.tsx` | Lesson item display |
| `src/components/PersonaCard.tsx` | AI-generated persona pill |
| `src/components/ProgressBar.tsx` | Lesson completion bar |
| `src/components/StreakBadge.tsx` | Practice streak display |
| `src/components/PremiumGate.tsx` | Blur/CTA for paywall |

#### [NEW] State & API
- `src/lib/supabase.ts` — Supabase client init
- `src/lib/posthog.ts` — PostHog init
- `src/hooks/useAuth.ts` — auth context hook
- `src/hooks/useRoadmap.ts` — fetch roadmap hook
- `src/hooks/useProgress.ts` — track/update progress

---

### Supabase (`/supabase`)

#### [NEW] Database Migrations (`/supabase/migrations/`)

```sql
-- Key tables:
users           -- profile, persona_cluster, embedding (vector)
quizzes         -- instrument, skill_level, genres[], goals[]
roadmaps        -- user_id, modules JSONB
lessons         -- title, youtube_url, content, metadata
lesson_metadata -- genre, skill, difficulty, topic
progress        -- user_id, lesson_id, status, watch_time
user_interactions -- user_id, lesson_id, event_type, reward
feedback        -- user_id, lesson_id, rating, comment
```

RLS policies: all tables scoped to `auth.uid()`.
pgvector extension enabled on `users.embedding` and `lessons.embedding`.

#### [NEW] Edge Functions (`/supabase/functions/`)
| Function | Purpose |
|----------|---------|
| `save-quiz` | Store onboarding answers, create initial persona |
| `generate-roadmap` | Call AI recommendation engine, return roadmap |
| `update-progress` | Log completion/skip/replay + compute reward |
| `get-recommendations` | Hybrid engine: content + collab + embedding |
| `stripe-checkout` | Create Stripe checkout session |
| `stripe-webhook` | Handle subscription lifecycle events |
| `explain-persona` | Call LLM to generate readable persona summary |

---

### AI System (`/ai`)

Python-based pipeline scripts (runnable locally or as scheduled jobs).

#### [NEW] Feature Vectors (`/ai/feature_vectors.py`)
- Pull `user_interactions` from Supabase
- Encode as vectors: lesson genre, difficulty, event types, watch time

#### [NEW] Persona Clustering (`/ai/clustering.py`)
- K-means clustering on user vectors
- Assign `persona_cluster` label back to `users` table

#### [NEW] Recommendation Engine (`/ai/recommendation.py`)
- Content-based: cosine similarity on lesson embeddings
- Collaborative: user-to-user via interaction matrix
- Embedding search: pgvector `<->` distance queries

#### [NEW] Reinforcement Learning (`/ai/rl_bandit.py`)
- Multi-armed bandit: epsilon-greedy + UCB + Thompson sampling
- Reward: +5 complete, +3 replay, −2 skip
- Update model weights from `user_interactions`

#### [NEW] LLM Persona (`/ai/persona_llm.py`)
- Build prompt from user's cluster, history, goals
- Call OpenAI GPT-4o (or Anthropic Claude)
- Return markdown summary (e.g., "Blues Improviser")

---

### Stripe Payments

Handled entirely via Supabase Edge Functions + Stripe SDK.

- Products: Free tier, Pro ($9.99/mo), Lifetime ($99)
- Webhook events: `checkout.session.completed`, `customer.subscription.deleted`
- `users.subscription_status` column updated on events
- `PremiumGate` component checks status before rendering premium content

---

### Analytics (PostHog)

Events tracked:
| Event | Properties |
|-------|-----------|
| `page_view` | path, referrer |
| `quiz_completed` | instrument, level, goals |
| `roadmap_viewed` | roadmap_id, module_count |
| `lesson_started` | lesson_id, source |
| `lesson_completed` | lesson_id, watch_time |
| `lesson_skipped` | lesson_id |
| `subscription_started` | plan, amount |

---

## Verification Plan

### Automated Tests

**Frontend unit tests (Vitest):**
```bash
cd frontend
npm run test
```
Tests: `useAuth`, `useRoadmap`, `PersonaCard`, `PremiumGate` component renders.

**Edge Function tests (Deno):**
```bash
cd supabase/functions
deno test --allow-net --allow-env
```

**AI pipeline tests (pytest):**
```bash
cd ai
pip install -r requirements.txt
pytest tests/
```
Tests: feature vector shape, clustering label types, RL reward calculations, LLM mock output parsing.

### E2E Browser Tests (Playwright)
```bash
cd frontend
npm run test:e2e
```
Flows tested:
1. Land → Sign Up → Quiz → Roadmap renders
2. Open lesson → mark complete → progress updates
3. Click upgrade → Stripe checkout opens

### Manual Verification
1. Start dev server: `cd frontend && npm run dev` → visit `http://localhost:5173`
2. Sign up with email → complete onboarding quiz → verify roadmap appears
3. Click a lesson → YouTube embed loads → mark complete → check dashboard streak increments
4. Visit Settings → click Upgrade → Stripe test mode checkout opens
