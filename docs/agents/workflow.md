# Development loop

A feature goes through the steps below in order. The skills marked ⌨️ only trigger if the user types their name; the others are called by the agent.

| # | Step | Skill | Session | Output |
|---|---|---|---|---|
| 1 | Interview | ⌨️ `/grill-with-docs <idea>` | A | Shared decisions, `GLOSSARY.md`, `docs/adr/` |
| 2 | Spec | ⌨️ `/to-spec` | A (same chat) | GitHub issue labelled `ready-for-agent` |
| 3 | Tickets | ⌨️ `/to-tickets` | A (same chat) | Sub-issues of the spec, linked by their blockers |
| 4 | Build | ⌨️ `/implement <ticket>` or ⌨️ `/implement-spec <spec>` | fresh | Commits on a branch, tests first |
| 5 | Review | `code-review` | sub-agents | Standards and Spec reports, then fixes |
| 6 | PR | `pr` | that of the build | Description with Summary, Evidence, Merge Danger |
| 7 | Retro | ⌨️ `/retro` | that of the build, before `/clear` | Suggestions on the repository, the checks, the instructions |

## 1. Interview

`/grill-with-docs` asks the questions in numbered rounds, each with a recommended answer. No code before the frontier is empty and the user confirms the agreement.

- Answer of the agent impossible to understand: ⌨️ `/wait-what`.
- Question that cannot be settled by talking (look of a screen, behaviour of a state model): ⌨️ `/handoff`, then `prototype` in a fresh session, and the answer comes back into the interview.
- Piece of work too big for one session: ⌨️ `/wayfinder` before the interview, to split it into decision tickets.

## 2 and 3. Spec and tickets

Stay in the chat of the interview: `/to-spec` and `/to-tickets` synthesise the conversation and ask no substantive question again. `/to-spec` has the test seams validated; `/to-tickets` has the split validated.

Each ticket is a vertical slice that fits in a fresh session.

## 4. Build

Two modes:

- **Ticket by ticket**: `/implement <ticket>` on a feature branch, then `/clear`, then the next ticket. Each ticket starts in an empty context.
- **Whole spec**: `/implement-spec <spec>`. One sub-agent per ready ticket, each in its worktree, merged on an integration branch with a draft PR.

In both cases the code is written with `tdd`: a red test at the agreed seam, then the minimum of code to make it pass. Refactoring does not happen here but in review.

## 5 and 6. Review and PR

`/implement` and `/implement-spec` call `code-review` themselves. To run it by hand: `code-review` with `main` as the fixed point.

The PR description follows the `pr` skill.

## 7. Retro

Before emptying the chat, `/retro` rereads the session and proposes improvements. It only proposes: the user chooses which ones to apply.

- Error that can be detected mechanically: a check (lint, hook, CI).
- Question of judgement: a rule in `CODING_STANDARDS.md`, read in review.
- File hard to find: a one-line pointer in `CLAUDE.md`.

## End of cycle

Once the PR is merged, close the issue of the spec. The decisions now live in the code, the tests and the glossary.

## Outside the loop

- Bug or regression: `diagnosing-bugs`.
- Architecture debt: ⌨️ `/improve-codebase-architecture`.
