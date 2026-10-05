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
   3. Open question: plain text matching first, or send the session
      summaries to an LLM for ranking (needs a key and sends screen
      contents off the machine, so it should be opt-in).
