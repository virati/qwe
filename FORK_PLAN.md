# Fork plan: session-manager fixes

Two features requested for this zellij fork. This doc tracks design decisions and
exact status so work can resume on another machine without re-deriving context.

## Feature 1: kill current session + switch to next (DONE, needs build verification)

Goal: a keybindable action that kills the *current* session outright (not just
detaches) and switches the client straight into another running session (the
alphabetically-next one, excluding the current one). If no other session exists,
it falls back to a plain quit.

Explicitly NOT "detach and leave running" — user confirmed: kill it.
Explicitly a *new* action/keybind, not a repurposing of Ctrl+q.

### Design

- New zero-arg `Action::CloseSessionAndSwitchToNext`.
- Every `Action` has to survive a client→server IPC round trip, which is
  protobuf-encoded (`zellij-utils/src/client_server_contract/common_types.proto`,
  generated into `zellij-utils/assets/prost_ipc/client_server_contract.rs` via
  `cargo xtask build` — codegen runs automatically whenever the `.proto` mtime is
  newer than the generated output, `protoc` must be on PATH). So the new action
  needed a protobuf variant too — added as field `148` (highest existing was 147).
- Reused the *existing* `Action::SwitchSession` machinery end-to-end (client
  detach/reconnect via `ServerToClientMsg::SwitchSession` /
  `ClientInstruction::SwitchSession`) rather than inventing a new client-side
  reconnect path.
- The "kill" part had to happen atomically with the "switch" part, in a single
  server-side instruction, to avoid a race between our own detach and a
  separately-fired kill (two independent IPC calls from the client could
  interleave badly — e.g. killing our own in-flight switch). So: new
  `ServerInstruction::SwitchSessionAndKillCurrentSession`, handled entirely
  within session A's own single-threaded instruction loop:
  1. send the requesting client `ServerToClientMsg::SwitchSession` (same as
     normal `SwitchSession`) and remove it from this session's bookkeeping
  2. send every *other* still-attached client (if any) `ServerToClientMsg::Exit`
     (same as `ServerInstruction::KillSession` does)
  3. `break` the loop, ending this session's server process — same shutdown as
     `KillSession`.
- "Next session down" is resolved server-side in
  `zellij-server/src/route.rs`, in the new `Action::CloseSessionAndSwitchToNext`
  handler: `zellij_utils::sessions::get_sessions()`, filter out the current
  session name, sort alphabetically, take the first. This matches the ordering
  the session-manager plugin already uses (current session pinned first, others
  alphabetical) — so "next down" in the list corresponds to "alphabetically
  first other session".
- No default keybinding was added — nothing bound this action to a key. The user
  needs to bind it themselves, e.g. in their config:
  ```kdl
  bind "Ctrl e" { CloseSessionAndSwitchToNext; }
  ```
  (Should ask the user what key they want, or just pick something sensible and
  mention it, before calling this fully done from a UX standpoint.)

### Files touched

- `zellij-utils/src/input/actions.rs` — new `Action::CloseSessionAndSwitchToNext` variant.
- `zellij-utils/src/kdl/mod.rs` — KDL parse (both the shared zero-arg macro table
  and the outer dispatcher match) + `to_kdl` serializer.
- `zellij-utils/src/client_server_contract/common_types.proto` — new oneof field
  148 + `message CloseSessionAndSwitchToNextAction {}`.
- `zellij-utils/assets/prost_ipc/client_server_contract.rs` — regenerated (do not
  hand-edit again; re-run `cargo xtask build` after further `.proto` changes).
- `zellij-utils/src/ipc/protobuf_conversion.rs` — both-direction `Action <->
  ActionType` conversion arms + import list entry.
- `zellij-utils/src/plugin_api/action.rs` — appended to the "Unsupported action"
  catch-all (this action is not meant to be invocable from plugins, same
  treatment as `Action::SwitchSession`).
- `zellij-server/src/route.rs` — added `sessions::get_sessions` import and the
  `Action::CloseSessionAndSwitchToNext` handler.
- `zellij-server/src/lib.rs` — new `ServerInstruction::SwitchSessionAndKillCurrentSession`
  variant + `ServerContext` mapping arm + full handler (mirrors `SwitchSession`
  handler then folds in `KillSession`'s shutdown of remaining clients).
- `zellij-utils/src/errors.rs` — new `ServerContext::SwitchSessionAndKillCurrentSession` variant.
- `zellij-client/src/input_handler.rs` — `dispatch_action` branch for the new
  action, mirrors the existing `Action::SwitchSession` branch (send to server,
  `self.exit(ExitReason::NormalDetached)`).

### Remaining for Feature 1

- Never compiled yet — see "Build & verify" below.
- Decide/ask the user for a default keybind, or leave unbound and just document it.

## Feature 2: renamed session shows custom name as primary, auto-name as secondary

Goal: after `RenameSession`, the session manager should show the user's chosen
name **bold/primary**, and the original auto-generated name dim/secondary in
parens next to it — confirmed wording from the user: "the new user-determined
name should be bold, and the hyphenated autoname should be in
parentheses/small."

### Design

- Zellij session rename is a *real* rename (socket file renamed, session_name
  overwritten everywhere) — there was no existing concept of "the name this
  session was created with" anywhere in the codebase. Had to add one.
- New field `SessionInfo.created_name: String` (`zellij-utils/src/data.rs`),
  set once and never mutated by the rename handler.
- New `Screen` field `created_session_name: String`
  (`zellij-server/src/screen.rs`), set once in `Screen::new()` alongside
  `session_name`, and deliberately **not** touched by
  `ScreenInstruction::RenameSession` (~line 11199) — that handler only mutates
  `screen.session_name`.
- Plumbed `created_name` through every place `SessionInfo` crosses a boundary:
  - the two live struct-literal construction sites in `screen.rs` (~5335,
    ~11157) that build the outgoing `SessionInfo` snapshot,
  - the plugin protobuf schema (`SessionManifest` message in
    `zellij-utils/src/plugin_api/event.proto`, field 13) + both-direction
    conversions in `zellij-utils/src/plugin_api/event.rs`,
  - the on-disk cross-process session cache, which is a **hand-written KDL
    serializer** (not plain serde) in `zellij-utils/src/kdl/mod.rs`
    (`SessionInfo::to_string` / `SessionInfo::from_string`, used by
    `background_jobs.rs` to write `session-metadata.kdl` per session under
    `session_info_folder_for_session`). `from_string` falls back to `name` if
    `created_name` is missing, so old cache files on disk don't break parsing.
- Session-manager plugin: added `created_name` to `SessionUiInfo`
  (`default-plugins/session-manager/src/ui/mod.rs`), and in
  `default-plugins/session-manager/src/ui/components.rs::build_session_ui_line`
  render a second dim span `" (created_name)"` right after the bold session
  name span, only when `created_name` is non-empty and differs from the
  current `name`. Added `Colors::dim()` helper
  (`text_unselected.base`, non-bold) alongside the existing `Colors::bold`/
  `tab_count`/`pane_count` helpers for this.

### Files touched

- `zellij-utils/src/data.rs` — `SessionInfo.created_name` field;
  `SessionInfo::new()` sets it equal to `name`.
- `zellij-server/src/screen.rs` — new `Screen.created_session_name` field, set
  once in the constructor, read (not written) by both `SessionInfo`
  construction sites.
- `zellij-utils/src/kdl/mod.rs` — `SessionInfo::to_string`/`from_string`
  read/write `created_name` (with back-compat fallback); also fixed the two
  hand-built `SessionInfo { .. }` test/example literals in this file
  (~6103-ish decode result, ~6754 test fixture).
- `zellij-utils/src/plugin_api/event.proto` — `SessionManifest.created_name`
  field 13.
- `zellij-utils/assets/prost/api.event.rs` — regenerated.
- `zellij-utils/src/plugin_api/event.rs` — both-direction conversions +
  two test fixtures (~3007, ~3039) updated.
- `default-plugins/session-manager/src/ui/mod.rs` — `SessionUiInfo.created_name`.
- `default-plugins/session-manager/src/ui/components.rs` —
  `build_session_ui_line` dim parenthetical span; `Colors::dim()` helper.

### Remaining for Feature 2 — NOT DONE YET

The session-manager plugin has **two separate rendering paths** and only one
is updated:

1. `ui/components.rs::build_session_ui_line` (DONE) — used by the
   `ActiveScreen::AttachToSession` multi-screen view (`SessionList` /
   `session_list.rs`).
2. **`single_screen.rs` / the "unified results" table — NOT DONE.** This is
   the *default* view (`main.rs` sets `active_screen =
   ActiveScreen::SingleScreen` whenever the `multi_screen` plugin config is
   not explicitly `"true"`, which is the common case), so this is the one the
   user will actually see day to day. It has its own data model:
   - `single_screen.rs::UnifiedSearchResult::ActiveSession` (~line 24) has no
     `created_name` field yet.
   - It's constructed from `SessionUiInfo` around `single_screen.rs:242` and
     `:257` (and a test helper at `:711`) — need to thread `created_name`
     through here too.
   - Rendering happens in `ui/components.rs::UnifiedResultsRenderCache::rebuild`
     (~line 66-163) which builds `CachedRowData` with a plain `session_name:
     String` used at a fixed `name_width` in a table column
     (`render_unified_results`, ~line 828). Splicing in a dim secondary name
     here is more invasive than the multi-screen case: either (a) add a
     separate `created_name` field to `CachedRowData` and render it as a
     second styled sub-span within the name column (adjusting `name_width`
     truncation math accordingly), or (b) bake the dim parenthetical directly
     into a pre-styled `session_name` string and track a separate *display
     width* for truncation (the existing code already separates "displayed
     string" from "width" in a few places, e.g. `StringAndLength`, so this
     pattern is available to copy).
   - Also check `UnifiedSearchResult::as_delete_target` / `session_name()` /
     `cmp_by_type_then_recency` (~line 52-103) — these don't need
     `created_name` but confirm nothing else assumes the struct shape without
     a wildcard `..` that would break on a new field.

Do this next, following the same "bold primary, dim secondary in parens" rule
as the multi-screen path.

## Build & verify — NOT DONE

Nothing in this branch has been compiled yet. Do this next, in order:

1. `cargo xtask build --no-plugins` (or `--no-web` to skip web-client
   features and speed things up) from the repo root — this re-runs proto
   codegen (only regenerates if `.proto` mtime > generated `.rs` mtime; safe
   to run repeatedly) and then `cargo build`s `zellij-utils`, `zellij-server`,
   `zellij-client`, and the `src` CLI binary.
2. Expect exhaustive-match compile errors from the two new enum variants
   (`Action::CloseSessionAndSwitchToNext`, and anywhere `SessionInfo { .. }`
   is constructed without `..Default::default()`) — fix each as the compiler
   points them out rather than trying to pre-grep every call site.
3. Once native crates build clean, build the plugins too (`cargo xtask build`
   without `--no-plugins`, needs `wasm32-wasip1` target) since
   `default-plugins/session-manager` was edited and is compiled to wasm
   separately.
4. Manually exercise both features in a running zellij (open two sessions,
   rename one, confirm the session manager shows `**custom** (auto-name)`;
   bind and trigger `CloseSessionAndSwitchToNext` and confirm it kills the
   session and lands you in the other one, and confirm the killed session no
   longer appears in `zellij list-sessions`).

## Open questions for the user

- What key should `CloseSessionAndSwitchToNext` be bound to by default (if
  any), or is "documented but unbound, user binds it themselves" fine?
- For Feature 2, "next down" was implemented as "alphabetically next session,
  excluding current" to match the session manager's own sort order — confirm
  this matches what "next session down" meant, since there's no on-screen list
  visible at the moment the keybind fires (it's a global action, not scoped to
  the session-manager plugin UI).
