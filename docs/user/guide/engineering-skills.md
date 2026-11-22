# Use the built-in engineering skills

English | [中文](engineering-skills.zh.md)

Every base-backed profile ships 27 engineering workflow skills, so a new Session has them without installing a plugin or copying files. The component is part of the base composition; its plugin metadata names it **Engineering workflow skills**, and the skill text comes from the upstream [mattpocock/skills](https://github.com/mattpocock/skills) project.

## What ships

All 27 appear in the composer's `/` menu. Sixteen of them are user-only, marked as such in the menu and invisible to the model; the other eleven are also model-invoked, so they enter the Session skill catalog and the model loads one when the task fits.

| Reachable by | Skills |
|---|---|
| Model and user | `eng-code-review`, `eng-codebase-design`, `eng-diagnosing-bugs`, `eng-domain-modeling`, `eng-grilling`, `eng-pr`, `eng-prototype`, `eng-research`, `eng-tdd`, `eng-wizard`, `eng-writing-for-agents` |
| User only | `eng-ask-matt`, `eng-grill-me`, `eng-grill-with-docs`, `eng-handoff`, `eng-implement`, `eng-implement-spec`, `eng-improve-codebase-architecture`, `eng-retro`, `eng-setup-matt-pocock-skills`, `eng-teach`, `eng-to-questionnaire`, `eng-to-spec`, `eng-to-tickets`, `eng-triage`, `eng-wait-what`, `eng-wayfinder` |

## Invoke a skill

Type `/` in the composer to list the skills and insert one by name. Entries group by where they came from, so these 27 sit under `Built-in · mattpocock/skills` next to headings such as `Project · .agents/skills` and `User · ~/.agents/skills` that supply other skills. `eng-ask-matt` is the router: it recommends the skill or flow for your situation, so start there when you are unsure.

For a model-invoked skill, describe the task and the model loads the one that fits; name the skill explicitly when you want that one.

## The main flow

The skills compose into a path from idea to shipped change: `eng-grill-with-docs` sharpens the idea and records the domain language, `eng-to-spec` and `eng-to-tickets` turn it into work, `eng-implement` drives `eng-tdd` and closes with `eng-code-review`, and `eng-pr` and `eng-retro` finish the change. `eng-triage`, `eng-diagnosing-bugs`, and `eng-wayfinder` are on-ramps for incoming issues, hard bugs, and work too large for one Session.

## Repository configuration

The engineering skills read `GLOSSARY.md`, ADRs, and an issue tracker. Run `/eng-setup-matt-pocock-skills` once per repository to record the tracker, triage labels, and documentation layout under `docs/agents/`.

## Override or remove

A skill with the same name in the project's `.dsh/skills` or `.agents/skills`, or in your user skill directories, replaces the shipped one. Because the shipped names carry the `eng-` prefix, a personal copy under the upstream name — `tdd`, say — sits beside `eng-tdd` instead of replacing it, and both stay reachable. To remove the whole component from a profile, disable its row in that profile's patch:

```yaml
- id: skill-engineering
  disabled: true
```
