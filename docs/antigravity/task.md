# MyTune — Task Checklist

## Phase 0: Planning & Architecture
- [x] Create implementation plan and get user approval
- [x] Define folder structure for monorepo
- [x] Write architecture decision records (ADRs)

## Phase 1: Project Scaffold
- [x] Initialize monorepo structure (`/frontend`, `/supabase`, `/ai`, `/scripts`)
- [x] Set up `frontend/` with Vite + React + TailwindCSS + React Router
- [x] Configure TypeScript, ESLint, Prettier
- [x] Set up Supabase project config files

## Phase 2: Database & Auth (Backend)
- [x] Write Supabase SQL migrations for all tables:
  - Users, Quizzes, Roadmaps, Lessons, Progress
  - Feedback, LessonMetadata, UserInteractions
- [x] Enable pgvector extension (lesson embeddings)
- [x] Configure Row Level Security (RLS) policies
- [x] Set up Supabase Auth (OAuth + email/password)
- [x] Write Edge Functions: roadmap-generate, stripe-checkout, stripe-webhook, explain-persona

## Phase 3: Frontend — Core Pages
- [x] `LandingPage` — hero, CTA, features
- [x] `AuthPage` — login/signup via Supabase Auth
- [x] `OnboardingQuiz` — instrument, level, genres, goals
- [x] `RoadmapPage` — personalized roadmap display
- [x] `LessonPlayer` — YouTube embeds + hosted content + exercises
- [x] `Dashboard` — streaks, progress, badges
- [x] `SettingsPage` — profile, subscription

## Phase 4: AI Personalization System
- [x] User feature vector construction (from interactions)
- [x] K-means / DBSCAN persona clustering pipeline
- [x] Store persona + vectors in Supabase (pgvector)

## Phase 5: Recommendation Engine
- [x] Content-based filtering (lesson metadata similarity)
- [x] Collaborative filtering (user-to-user)
- [x] Hybrid scoring + ranking

## Phase 6: Reinforcement Learning
- [x] Reward function (+5 complete, +3 replay, -2 skip)
- [x] Multi-armed bandit (epsilon-greedy + UCB + Thompson)

## Phase 7: LLM Persona Explanations
- [x] Prompt template for persona summaries
- [x] API integration (OpenAI / Anthropic via Edge Function)
- [x] Display persona card in dashboard and roadmap

## Phase 8: Payments (Stripe)
- [x] Stripe products + price config setup instructions
- [x] Checkout session Edge Function
- [x] Webhook handler (subscription lifecycle)
- [x] Premium feature gating in frontend settings

## Phase 9: Analytics (PostHog)
- [x] PostHog SDK integration in frontend
- [x] Key event tracking (page views, lesson start/complete, quiz submit)

## Phase 10: Testing & QA
- [ ] Unit tests for AI functions
- [ ] Edge Function integration tests
- [ ] E2E tests with Playwright (auth + quiz + roadmap flows)
- [ ] Stripe webhook testing

## Phase 11: DevOps & Deployment
- [x] GitHub Actions CI pipeline
- [x] Vercel deployment for frontend
- [x] Supabase configuration setup
- [x] Environment secrets management (.env.example)
