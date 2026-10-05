# Fork to-do (virati/qwe)

Changes this fork carries on top of zellij-org/zellij, and ones still to build.
GitHub issues are off on this repo, so the list lives here.

## Done

1. **Session preview in the session manager** — a panel right of the session
   list shows the selected session's active tab, so empty sessions can be
   spotted and killed without attaching. Branch `vineet/preview`, commit 8c273d1c5.

## To do

2. **Find and attach to a session by describing it** — ask zellij for "the
   session where I'm working on X" in plain language and attach to the best
   match, e.g. `zellij attach --find "EEG DMD per-electrode run"`, plus a
   matching search mode in the session manager.
   1. Match against what each session already publishes to the session info
      cache: session name, tab names, pane titles and commands, working
      directories, and the `session-preview.txt` capture from item 1.
   2. Rank sessions, show the top few with why each matched, attach on
      confirm (attach straight away only when one match is clearly ahead).
   3. Ranking is done by an LLM through **pi**, the way `virati/e_harness`
      does it: headless, no pi TUI, no tools (it only reads the summaries it
      is handed, so text on someone's screen has nothing to act on), and a
      small fast model by default (`claude-haiku-4-5`, overridable like
      `EH_FAST_MODEL`). e_harness drives pi through its Node SDK
      (`createAgentSession` in `src/agent.mjs`); from Rust the simpler route
      is to shell out to `pi -p --no-tools --no-session --mode json --model
      ...` with the session summaries in the prompt and ask for ranked
      session names plus a one-line reason each.
   4. Off by default, since it sends screen contents to the model provider;
      if `pi` is missing or fails, fall back to plain text matching and say so.
