# Commit Rules & Guidelines

To maintain a clean and professional commit history, follow these rules for every commit.

## 1. Use Conventional Commits
Every commit message must follow this structure:
`<type>(<scope>): <description>`

### Types:
- **feat**: A new feature (e.g., `feat(ui): add search bar`)
- **fix**: A bug fix (e.g., `fix(engine): resolve buffer underrun`)
- **docs**: Documentation changes (e.g., `docs(brain): update roadmap`)
- **style**: Changes that do not affect the meaning of the code (white-space, formatting, missing semi-colons, etc)
- **refactor**: A code change that neither fixes a bug nor adds a feature
- **perf**: A code change that improves performance
- **test**: Adding missing tests or correcting existing tests
- **chore**: Changes to the build process or auxiliary tools and libraries (e.g., `chore: update dependencies`)

### Scopes:
- `core`: Audio engine, mixer, decks
- `ai`: LLM integration, planner, heuristic logic
- `analysis`: BPM, Key, Section detection
- `storage`: Database, preferences, migrations
- `ui`: React components, styles, store
- `app`: Tauri commands, window management
- `brain`: Project documentation and task management

## 2. Write Descriptive Messages
- Summary line should be 50 characters or less.
- Start with a lowercase type/scope.
- Use imperative mood ("add feature" not "added feature").
- Separate the summary from the body with a blank line if more detail is needed.
- Use bullet points in the body to describe technical changes.

## 3. Clean and Atomic Commits
- Commit related changes together.
- Avoid "work in progress" commits. If active development is stalling, use branches or stashes.
- Ensure the code builds and tests pass before committing.
- Do not commit large binary files (like sample songs) unless explicitly requested.

## 4. Documentation Compliance
- When completing a phase, always update `docs/brain/task.md` in the same commit or a follow-up `docs(brain)` commit.
- Do not touch coding files unless explicitly requested.
