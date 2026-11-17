# HANDOFF: `labu`

You are picking up a greenfield Rust project for RedPumpkin.AI. Nothing is built yet. The plan is approved; your job is to get **v0.1** into daily use.

## Read first

`docs/PLAN.md` (Indonesian) is the single source of truth. This file only adds what the plan lacks: order of work and when each step is done.

| Need | Section of `docs/PLAN.md` |
|---|---|
| What `labu` is, command list, architecture | 2 |
| Milestones and v0.1 scope | 3 |
| Design invariants and CI gates | 5 |
| What existing tools lack (why each feature exists) | 9 |

## Invariants

Four names from section 5 that every PR is checked against:

- **raw-bytes cassette**: responses are stored and replayed as the exact bytes received.
- **client-less replay**: the replay engine holds no outbound HTTP client.
- **header allowlist**: only known-safe headers reach disk.
- **canary secret**: a test plants a fake secret in a request and asserts it never appears on disk.

## Steps

Work in order. Each step ends on its criterion.

1. **Settle the open questions** with the human, one at a time:
   - Which provider does their agent call today? Working assumption: AWS Bedrock (their product runs on it), fallback Anthropic Messages API.
   - Comfort level with Rust?
   - Is the GitHub org `redpumpkin-ai` theirs? Licence: `MIT OR Apache-2.0`?
   - Which real agent will be the first user of `labu`?

   *Done when:* every answer is recorded in `docs/adr/0001-v0.1-decisions.md`.

2. **Scaffold** a cargo workspace with CI running fmt, clippy and tests.

   *Done when:* CI is green on a trivial test and `labu --help` prints.

3. **Bedrock spike**, timeboxed to 3 days (skip if step 1 chose another provider). Check the current Bedrock Runtime API reference and the current docs of any signing or event-stream crate before relying on memory.

   *Done when:* one `ConverseStream` call is recorded, then replayed byte-equal with no AWS credentials in the environment; **or** the timebox ends and the ADR records the fallback provider.

4. **Write the v0.1 implementation plan** covering only the v0.1 row of section 3.

   *Done when:* the human approves the plan.

5. **Implement v0.1 test-first.** Each invariant above gets its test before its code.

   *Done when:* the human's real agent test suite runs in CI in replay mode with `live calls: 0`, and the run's log shows it.

6. **Stop.** Write `docs/caught/v0.1.md` (what `labu` caught in real use) and wait. v0.2 starts only after two weeks of use.

## Ideas that come up mid-work

Append them to `LATER.md` and continue the current step.

## Suggested skills

- `superpowers:writing-plans` at step 4
- `superpowers:test-driven-development` (or `tdd`) at step 5
- `superpowers:using-git-worktrees` before step 5
- `superpowers:verification-before-completion` before claiming any step done
- `domain-modeling` when recording ADRs or a `CONTEXT.md` glossary
- `to-tickets` if the human wants v0.1 broken into tracker issues

## Context the plan does not carry

- The human wants something **usable first**; being first or best in the category is a non-goal.
- Long-term goal: public credibility in agentic AI. The `docs/caught/` notes are the raw material for that, so write them with real numbers.
- The human writes casual Indonesian; reply in kind.
