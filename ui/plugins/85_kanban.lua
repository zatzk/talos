-- Talos kanban — the board, as a plugin.
--
-- The octomux workflow board, over talos's own facts. Its six columns are
-- NOT stored anywhere: every card's column is DERIVED, on every frame, from
-- what its session observably is — the status the agent reported, and the
-- shape of the work in its tree. A card moves because the world moved; there
-- is no second bookkeeping to drift.
--
--   backlog        tasks not yet dispatched (talos's todo list — the prompts)
--   planned        sessions with nothing to look at yet: parked, silent, clean
--   in_progress    working, or printing, or an agent holds the pane unreported
--   human_review   blocked — a permission prompt, a question — or done with an
--                  uncommitted tree nobody has reviewed
--   pr             done with committed work ahead of its base, not yet merged
--   done           merged: the branch's work landed
--
-- talos's TASK status is a closed enum (todo/in_progress/done, enforced by
-- the kernel) — it tracks the prompt, not the review, and the board does not
-- try to store workflow stages in it.
--
-- Reads come from the snapshot (`talos.sessions`, `talos.tasks`); writes
-- go out as `action`/`diff`/`task`/`dispatch` commands. Nothing privileged.
--
-- Install with:
--
--     talos-cli plugin install kanban
--
-- A SWITCH pane in the centre (slot_mode = "switch"): F6 brings it forward.

local theme = require("lib.theme")
local widgets = require("lib.widgets")
local chrome = require("lib.chrome")

local ORDER = { "backlog", "planned", "in_progress", "human_review", "pr", "done" }

local GLYPH = {
  backlog = "○",
  planned = "◌",
  in_progress = "◐",
  human_review = "◆",
  pr = "◇",
  done = "●",
}

-- Column header colour, from theme ROLES so this pane looks right under every
-- palette. human_review is where a human owes the board something, so it
-- wears the working colour; done is at rest, so it is muted.
local function column_style(name, focused_column)
  if focused_column then
    return { fg = theme.accent, bold = true }
  elseif name == "human_review" then
    return { fg = theme.warn }
  elseif name == "done" then
    return { fg = theme.muted }
  end
  return { fg = theme.text }
end

-- --- the facts -----------------------------------------------------------------

local function sessions()
  return (talos and talos.sessions) or {}
end

local function tasks()
  return (talos and talos.tasks) or {}
end

--- Where a session sits on the board. Priority order matters: a blocked
--- session is in human_review whatever its tree looks like — the prompt is
--- the fact; a done session with an unmerged commit is a PR; a done session
--- with a dirty tree is review work; everything else is planned.
local function column_of_session(session)
  if session.status == "blocked" then
    return "human_review"
  end
  if session.status == "working" or session.status == "running" then
    return "in_progress"
  end
  local git = session.git
  if session.status == "done" or session.status == "idle" then
    if git and git.merged then
      return "done"
    elseif git and git.dirty then
      return "human_review"
    elseif git and (git.ahead or 0) > 0 then
      return "pr"
    end
    return "planned"
  end
  return "planned" -- stopped, unreported, uncovered: parked, nothing to look at
end

--- Tasks that are not yet anybody's session: the prompt pile.
local function backlog_tasks()
  local rows = {}
  for _, task in ipairs(tasks()) do
    if task.status == "todo" then
      rows[#rows + 1] = task
    end
  end
  return rows
end

--- The board: six columns of cards. A card is a session (with its derived
--- column) or, in backlog, an undispatched task.
local function board()
  local cols = {}
  for _, name in ipairs(ORDER) do
    cols[name] = {}
  end
  for _, task in ipairs(backlog_tasks()) do
    cols.backlog[#cols.backlog + 1] = { task = task }
  end
  for _, session in ipairs(sessions()) do
    local col = column_of_session(session)
    cols[col][#cols[col] + 1] = { session = session }
  end
  return cols
end

-- --- the cursor ------------------------------------------------------------------

--- Per-pane state. `store` survives a frame; `state` would persist across
--- restarts, which a cursor position has no business doing.
local function ui()
  store.kanban_ui = store.kanban_ui or { col = 1, row = 1 }
  return store.kanban_ui
end

local function selected()
  local cols = board()
  local list = cols[ORDER[ui().col]] or {}
  local at = widgets.clamp(ui().row, #list)
  return list[at], at, list
end

--- One card. Backlog cards are prompts; session cards are named work with the
--- branch and the git shape of what sits in the tree.
local function card_of(card, is_cursor)
  local style = is_cursor and { fg = theme.accent, bold = true }
    or column_style(card.column, false)
  local spans
  if card.task then
    local prefix = ""
    if card.task.canonical_id and card.task.canonical_id ~= "" then
      prefix = "[" .. card.task.canonical_id .. "] "
    end
    spans = {
      { text = GLYPH.backlog .. " ", style = style },
      { text = prefix .. (card.task.title or "(untitled)"), style = style },
    }
    if card.task.rfc_id and card.task.rfc_id ~= "" then
      spans[#spans + 1] = { text = " " .. card.task.rfc_id, style = { fg = theme.accent } }
    elseif card.task.source and card.task.source ~= "local" then
      spans[#spans + 1] = { text = " " .. card.task.source, style = { fg = theme.muted } }
    end
  else
    local session = card.session
    local status = theme.status(session.status)
    spans = {
      { text = status.glyph .. " ", style = { fg = status.color } },
      { text = session.name or "(unnamed)", style = style },
    }
    if session.branch then
      spans[#spans + 1] = { text = " " .. session.branch, style = { fg = theme.branch } }
    end
    local git = session.git
    if git and git.dirty then
      spans[#spans + 1] = {
        text = string.format(" +%d -%d", git.insertions or 0, git.deletions or 0),
        style = { fg = theme.warn },
      }
    end
  end
  return spans
end

--- The card under the cursor, tagged with its column for the key handlers.
local function cards_with_columns()
  local cols = board()
  local cards = {}
  for _, name in ipairs(ORDER) do
    for _, card in ipairs(cols[name]) do
      card.column = name
      cards[#cards + 1] = card
    end
  end
  return cards, cols
end

return {
  name = "kanban",
  slot = "center",
  slot_mode = "switch",
  order = 80,
  focusable = true,

  -- Offered in the action band so the pane is discoverable by click too, and
  -- ctrl+h / ctrl+l cycle it into view (the kernel's focus movement).
  pills = {
    { action = "kanban.open", label = "board", priority = 10 },
  },

  keys = {
    -- F2 primary and Alt+2 alternate for Workspace Board per RFC v2.0
    { key = "f2", action = "kanban.open", desc = "workspace board", scope = "global", group = "Talos" },
    { key = "alt+2", action = "kanban.open", desc = "workspace board", scope = "global", group = "Talos" },
    { key = "j", action = "kanban.down", desc = "next card", group = "Talos" },
    { key = "k", action = "kanban.up", desc = "previous card", group = "Talos" },
    { key = "h", action = "kanban.left", desc = "column left", group = "Talos" },
    { key = "l", action = "kanban.right", desc = "column right", group = "Talos" },
    { key = "enter", action = "kanban.open_card", desc = "open / dispatch", group = "Talos" },
    { key = "d", action = "kanban.diff", desc = "refresh diff", group = "Talos" },
    { key = "t", action = "kanban.cycle_task", desc = "cycle task todo→doing→done", group = "Talos" },
  },

  render = function(ctx)
    local state = ui()
    local _, cols = cards_with_columns()

    local columns = {}
    for index, name in ipairs(ORDER) do
      local list = cols[name]
      state.row = widgets.clamp(state.row, math.max(#list, 1))

      local rows = {}
      for at, card in ipairs(list) do
        rows[#rows + 1] = {
          spans = card_of(card, index == state.col and at == state.row and ctx.focused),
          id = card.task and tostring(card.task.id) or card.session.id,
        }
      end
      if #rows == 0 then
        rows[#rows + 1] = { spans = { theme.dim("  —") } }
      end

      local body = widgets.list({
        rows = rows,
        selected = (index == state.col) and state.row or nil,
        height = math.max(1, ctx.height - 6),
      })
      body.fill = 1

      columns[#columns + 1] = {
        type = "box",
        axis = "vertical",
        fill = 1,
        children = {
          {
            type = "text",
            len = 1,
            text = {
              {
                { text = name, style = column_style(name, index == state.col) },
                { text = " " .. #list, style = { fg = theme.muted } },
              },
            },
          },
          body,
        },
      }
    end

    local children = {
      { type = "box", axis = "horizontal", fill = 1, children = columns },
      widgets.divider(ctx.width - 2),
      widgets.hints({
        { "enter", "open / dispatch" },
        { "d", "diff" },
        { "t", "cycle task" },
        { "h/l", "column" },
      }),
    }

    local level = chrome.level(ctx.focused)
    local border = chrome.border_style(level)
    local width = ctx.width or 80
    local strip, _ = chrome.central_tab_strip(width, border, "board", chrome.rule(level))
    local frame = chrome.central_border_frame(" Workspace Board ", level, border, strip)

    return {
      type = "box",
      frame = frame,
      children = children,
    }
  end,

  on_action = function(action)
    local state = ui()
    local card = selected()

    if action == "kanban.open" then
      command("focus", { text = "kanban", toggle = true })
      return true
    end
    if action == "kanban.down" or action == "kanban.up" then
      local _, cols = cards_with_columns()
      local list = cols[ORDER[state.col]] or {}
      local step = action == "kanban.down" and 1 or -1
      state.row = widgets.clamp(state.row + step, math.max(#list, 1))
      return true
    end
    if action == "kanban.left" or action == "kanban.right" then
      local step = action == "kanban.right" and 1 or -1
      state.col = ((state.col - 1 + step) % #ORDER) + 1
      state.row = 1
      return true
    end
    if not card then
      return false
    end
    if action == "kanban.open_card" then
      -- A backlog card is a prompt: dispatch hands it to an agent, exactly as
      -- the tasks pane's `enter` does. A session card is work: open it where
      -- it lives — the prompt it is blocked on is answered in its pane.
      if card.task then
        local branch = nil
        if card.task.canonical_id and card.task.canonical_id ~= "" then
          branch = "feat/" .. card.task.canonical_id:lower()
        end
        command("dispatch", { number = card.task.id, branch = branch })
      else
        command("action", { text = "sessions.open", session = card.session.id })
      end
      return true
    end
    if action == "kanban.diff" then
      if card.session then
        command("diff", { session = card.session.id })
        command("message", { text = "diff requested for " .. card.session.name, level = "info" })
      end
      return true
    end
    if action == "kanban.cycle_task" then
      -- The one stored fact on the board: a prompt's own todo state, in the
      -- kernel's closed vocabulary.
      if card.task then
        local next = card.task.status == "todo" and "in_progress"
          or (card.task.status == "in_progress" and "done" or "todo")
        command("task", { number = card.task.id, status = next })
      end
      return true
    end
    return false
  end,
}
