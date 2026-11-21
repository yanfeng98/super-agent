# Use the built-in engineering skills

English | [中文](engineering-skills.zh.md)

Every base-backed profile ships 27 engineering workflow skills, so a new Session has them without installing a plugin or copying files. The component is part of the base composition; its plugin metadata names it **Engineering workflow skills**, and the skill text comes from the upstream [mattpocock/skills](https://github.com/mattpocock/skills) project.

## What ships

Eleven skills are model-invoked and appear in the Session skill catalog; the model loads one when the task fits. Sixteen are user-invoked: type `/` in the composer and pick one.

| Invocation | Skills |
|---|---|
| Model-invoked | `code-review`, `codebase-design`, `diagnosing-bugs`, `domain-modeling`, `grilling`, `pr`, `prototype`, `research`, `tdd`, `wizard`, `writing-for-agents` |
| User-invoked | `ask-matt`, `grill-me`, `grill-with-docs`, `handoff`, `implement`, `implement-spec`, `improve-codebase-architecture`, `retro`, `setup-matt-pocock-skills`, `teach`, `to-questionnaire`, `to-spec`, `to-tickets`, `triage`, `wait-what`, `wayfinder` |

## Invoke a skill

Type `/` in the composer to list the user-invoked skills and insert one by name. Entries group by where they came from, so these 27 sit under `mattpocock/skills` next to headings for the project and user directories that supply other skills. `ask-matt` is the router: it recommends the skill or flow for your situation, so start there when you are unsure.

For a model-invoked skill, describe the task and the model loads the one that fits; name the skill explicitly when you want that one.

## The main flow

The skills compose into a path from idea to shipped change: `grill-with-docs` sharpens the idea and records the domain language, `to-spec` and `to-tickets` turn it into work, `implement` drives `tdd` and closes with `code-review`, and `pr` and `retro` finish the change. `triage`, `diagnosing-bugs`, and `wayfinder` are on-ramps for incoming issues, hard bugs, and work too large for one Session.

## Repository configuration

The engineering skills read `GLOSSARY.md`, ADRs, and an issue tracker. Run `/setup-matt-pocock-skills` once per repository to record the tracker, triage labels, and documentation layout under `docs/agents/`.

## Override or remove

A skill with the same name in the project's `.dsh/skills` or `.agents/skills`, or in your user skill directories, replaces the built-in one. To remove the whole component from a profile, disable its row in that profile's patch:

```yaml
- id: skill-engineering
  disabled: true
```
