# README Beautification Tool Decision Matrix (`bd-1w9.1`)

Goal: make `README.md` visually strong, clear, and trustworthy without AI-generated hero art.

## Selection criteria

- Improves comprehension, not decoration-only
- Reproducible from repo commands
- Low maintenance burden
- Works in GitHub README rendering
- Supports trust-first posture (proof over slogans)

## Tool matrix

| Tool | Use in Vifei README | Pros | Risks / limits | Decision |
|---|---|---|---|---|
| `shields.io` | concise status badges (tests, release, license) | fast scanability; standard OSS pattern | badge clutter and rainbow noise if overused | **Adopt with strict cap** (3-5 badges max) |
| GitHub Mermaid | architecture and flow diagrams | source-controlled text diagrams; no image editor required | can become busy if oversized | **Adopt** for one architecture block |
| `vhs` | scripted terminal GIF/video capture from `.tape` | polished output in some workflows | extra dependencies (`ttyd`, `ffmpeg`); visual style does not fit current project direction | **Do not adopt for current lane** |
| `asciinema` | terminal session recording and replay | lightweight, authentic terminal proof; straightforward CI/dev usage | playback integration choices vary by channel | **Adopt as the standard capture lane** |
| `markdownlint-cli2` | README style and structure consistency | catches readability drift; CI-friendly | can be noisy if over-configured | **Adopt with small focused ruleset** |

## Specific standards for this repo

1. Badges
- Cap at 5 total.
- Only trust-bearing badges: build, tests, release/version, license, docs verification status.
- Use one subdued visual style (no mixed badge styles).

2. Diagram policy
- One architecture Mermaid diagram near top-half of README.
- Keep labels short and technical.
- Do not duplicate constitutional threshold tables in diagram text.

3. Visual assets policy
- No AI-generated hero image.
- Prefer deterministic terminal-native captures generated from real commands.
- Keep assets as evidence-centric snippets, not decorative banners.

4. Emoji and visual tone policy
- Keep emoji sparse and intentional.
- Recommended: at most one emoji per major section header, or none.
- Avoid novelty/corny emoji and avoid visual noise.

## Recommended baseline palette/tone for README polish

- Primary accent: cool blue or slate
- Success accent: green
- Warning accent: amber
- Error accent: red
- Neutrals: gray ramp

Use these colors consistently in badges/diagrams only; do not oversaturate.

## Proposed execution for next beads

- `bd-1w9.2`: apply new information architecture and writing rules.
- `bd-1w9.3`: generate deterministic visual assets (Mermaid + terminal captures).
- `bd-1w9.4`: implement polished README with capped badges and restrained emoji.
- `bd-1w9.5`: run readability + command validity + link QA.

## Sources

- GitHub diagrams in Markdown (Mermaid support):  
  https://docs.github.com/en/enterprise-cloud%40latest/get-started/writing-on-github/working-with-advanced-formatting/creating-diagrams
- Shields.io docs:  
  https://shields.io/docs/  
  https://shields.io/docs/static-badges
- VHS repository and usage:  
  https://github.com/charmbracelet/vhs
- asciinema docs:  
  https://docs.asciinema.org/
- markdownlint-cli2:  
  https://github.com/DavidAnson/markdownlint-cli2
