-- Talos Chat — The native conversational orchestrator interface (F1).
--
-- The primary operator view of Talos v3:
-- 1. Conversation threads per workspace / session.
-- 2. Message stream history with multi-calling & Jev Intent badges.
-- 3. Jev Intent routing (Auto persona, tier, backend: API vs Headless CLI).
-- 4. Multi-agent mentions (@architect, @dev, @qa, @spec-master, @sec).
-- 5. Model/Agent selector modal (F4 / Ctrl+O) supporting Headless OAuth CLIs.
-- 6. Quick action to approve specs and commit to code-documentation (Ctrl+A).

local chrome = require("lib.chrome")
local theme = require("lib.theme")
local widgets = require("lib.widgets")
local textinput = require("lib.textinput")

local NAME = "chat"

local SELECTOR_OPTIONS = {
  { label = "⚡ Auto: Jev Router (Inteligente)", desc = "Auto-routing: avalia intenção, persona e melhor modelo/CLI", kind = "auto", agent = nil, model = nil },
  { label = "🤖 Headless Claude Code", desc = "CLI: claude -p — OAuth / Subscrição Claude Pro/Max local", kind = "cli", agent = "claude", model = "claude-3.7-sonnet" },
  { label = "✨ Headless Antigravity (agy)", desc = "CLI: agy -p — Google OAuth / Gemini Pro & Flash", kind = "cli", agent = "antigravity", model = "gemini-2.5-pro" },
  { label = "⚡ Headless Codex CLI", desc = "CLI: codex exec — OpenAI OAuth / o3-mini & gpt-4o", kind = "cli", agent = "codex", model = "o3-mini" },
  { label = "🏛️ Architect API", desc = "API: claude-3.7-sonnet — RFC e Design de Arquitetura", kind = "api", agent = "architect", model = "claude-3.7-sonnet" },
  { label = "🛠️ Dev Worker API", desc = "API: claude-3.7-sonnet — Implementação de código autônoma", kind = "api", agent = "dev", model = "claude-3.7-sonnet" },
  { label = "🛡️ QA Engineer API", desc = "API: claude-3.7-sonnet — Testes e auto-healing QA", kind = "api", agent = "qa", model = "claude-3.7-sonnet" },
  { label = "📋 Spec Master API", desc = "API: claude-3.7-sonnet — Pipeline de PRD e Governança", kind = "api", agent = "spec-master", model = "claude-3.7-sonnet" },
  { label = "🌐 9Router (GLM-4 / Kimi)", desc = "API Router: Modelos GLM e Kimi de alto raciocínio", kind = "api", agent = "router", model = "glm-4-plus" },
  { label = "🌐 OpenRouter (Qwen / DeepSeek)", desc = "API Router: Qwen 2.5 72B & DeepSeek V3", kind = "api", agent = "openrouter", model = "qwen-2.5-72b" },
}

local local_state = nil

local function get_state()
  if not local_state then
    local_state = {
      active_workspace = state.active_workspace or "default",
      active_thread = state.active_thread or "th-main",
      thread_title = state.thread_title or "Chat 1",
      input_field = {
        value = state.input_value or "",
        cursor = state.input_cursor or 0,
      },
      messages = {
        {
          role = "system",
          sender = "⚡ Jev Engine",
          badge = "Talos v3 Control Plane",
          time = "Agora",
          content = "Talos v3 pronto. Selecione uma sessão à esquerda ou digite seu comando aqui.\nAtalhos: [F1] Chat, [F8] Shell, [F2] Board Kanban, [F4/Ctrl+O] Seletor de Modelo/CLI, [Ctrl+A] Aprovar Spec.",
        },
      },
      scroll_offset = state.scroll_offset or 0,
      streaming = false,
      has_spec_context = state.has_spec_context == true,
      selected_target = state.selected_target or "Auto (Jev Router)",
      target_kind = state.target_kind or "auto",
      target_agent = state.target_agent,
      target_model = state.target_model,
      selector_open = state.selector_open == true,
      selector_index = state.selector_index or 1,
    }
  end
  return local_state
end

local function save_state(s)
  local_state = s
  state.active_workspace = s.active_workspace
  state.active_thread = s.active_thread
  state.thread_title = s.thread_title
  state.input_value = s.input_field.value
  state.input_cursor = s.input_field.cursor
  state.scroll_offset = s.scroll_offset
  state.has_spec_context = s.has_spec_context
  state.selected_target = s.selected_target
  state.target_kind = s.target_kind
  state.target_agent = s.target_agent
  state.target_model = s.target_model
  state.selector_open = s.selector_open
  state.selector_index = s.selector_index
end

local function active_session_name()
  local id = store.selected
  if not id then return "principal" end
  for _, sess in ipairs(talos and talos.sessions or {}) do
    if sess.id == id then
      return sess.name or id
    end
  end
  return id
end

local function active_thread_title(s)
  if s.thread_title and s.thread_title ~= "" then
    return s.thread_title
  end
  local active_th = s.active_thread
  for _, th in ipairs(talos and talos.threads or {}) do
    if th.id == active_th then
      return th.title or th.id
    end
  end
  return "Chat 1"
end

local function get_sender_style(role, sender)
  sender = sender or ""
  if role == "user" then
    return theme.info or theme.accent
  elseif role == "system" then
    return theme.warn or theme.accent
  end
  if sender:find("architect") then
    return theme.accent
  elseif sender:find("qa") then
    return theme.warn
  elseif sender:find("sec") then
    return theme.warn
  elseif sender:find("dev") then
    return theme.info or theme.text
  end
  return theme.accent
end

local function format_message(msg)
  local sender_color = get_sender_style(msg.role, msg.sender)
  local header_spans = {
    { text = "● ", style = { fg = sender_color, bold = true } },
    { text = (msg.sender or msg.role) .. " ", style = { fg = sender_color, bold = true } },
  }

  if msg.badge and msg.badge ~= "" then
    header_spans[#header_spans + 1] = {
      text = "[" .. msg.badge .. "] ",
      style = { fg = theme.muted },
    }
  end

  if msg.time and msg.time ~= "" then
    header_spans[#header_spans + 1] = {
      text = "(" .. msg.time .. ")",
      style = { fg = theme.muted },
    }
  end

  local lines = {
    { spans = header_spans },
  }

  local content = msg.content or ""
  for line in content:gmatch("[^\r\n]+") do
    lines[#lines + 1] = { spans = { { text = "  " .. line, style = { fg = theme.text } } } }
  end
  lines[#lines + 1] = { spans = { { text = "", style = {} } } }

  return lines
end

local function collect_messages(s)
  local msgs = {}
  if talos and talos.chat_messages and #talos.chat_messages > 0 then
    local active_th = s.active_thread or "th-main"
    for _, m in ipairs(talos.chat_messages) do
      if not m.thread_id or m.thread_id == active_th or active_th == "th-main" then
        local sender_name = m.agent
        if not sender_name or sender_name == "" then
          if m.role == "user" then
            sender_name = "Operador"
          elseif m.role == "system" then
            sender_name = "📡 Control Plane"
          else
            sender_name = "Talos Agent"
          end
        end
        local badge = nil
        if m.backend and m.model then
          badge = m.backend .. ": " .. m.model
        elseif m.backend then
          badge = m.backend
        end
        local time_str = "Agora"
        if m.created_at and type(m.created_at) == "number" and m.created_at > 0 then
          time_str = os.date("%H:%M", math.floor(m.created_at / 1000))
        end
        msgs[#msgs + 1] = {
          role = m.role,
          sender = sender_name,
          badge = badge,
          time = time_str,
          content = m.content or "",
        }
      end
    end
  end

  if #msgs == 0 then
    return s.messages
  end
  return msgs
end

return {
  name = NAME,
  slot = "center",
  slot_mode = "switch",
  order = 15,
  focusable = true,

  pills = {
    { action = "chat.open", label = "chat", priority = 20 },
    { action = "chat.selector_toggle", label = "selector", priority = 10 },
  },

  keys = {
    -- F1 primary and Alt+1 alternate per RFC v2.0
    { key = "f1", action = "chat.open", desc = "chat view", scope = "global", group = "Talos" },
    { key = "alt+1", action = "chat.open", desc = "chat view", scope = "global", group = "Talos" },
    { key = "f4", action = "chat.selector_toggle", desc = "model selector", scope = "global", group = "Talos" },
    { key = "enter", action = "chat.send", desc = "send message", group = "Chat" },
    { key = "ctrl+a", action = "chat.approve_spec", desc = "approve spec & commit", group = "Chat" },
    { key = "esc", action = "chat.cancel", desc = "cancel selector", group = "Chat" },
  },

  render = function(ctx)
    local s = get_state()
    local width, height = ctx.width or 80, ctx.height or 24
    local level = chrome.level(ctx.focused)
    local border = chrome.border_style(level)

    -- If selector is open, render model/agent selector modal overlay
    if s.selector_open then
      local rows = {}
      rows[#rows + 1] = {
        spans = {
          { text = " Escolha o Alvo / Modelo / Agente CLI Headless (OAuth):", style = { fg = theme.accent, bold = true } },
        },
      }
      rows[#rows + 1] = { spans = { { text = "", style = {} } } }

      for i, opt in ipairs(SELECTOR_OPTIONS) do
        local is_sel = (i == s.selector_index)
        local cursor = is_sel and " ▶ " or "   "
        local style_label = is_sel and { fg = theme.accent, bold = true } or { fg = theme.text }
        local style_desc = is_sel and { fg = theme.text } or { fg = theme.muted }

        rows[#rows + 1] = {
          spans = {
            { text = cursor, style = { fg = theme.accent, bold = true } },
            { text = opt.label .. "  ", style = style_label },
            { text = opt.desc, style = style_desc },
          },
        }
      end

      local sel_height = math.max(1, height - 5)
      local selector_widget = widgets.list({
        rows = rows,
        selected = s.selector_index + 2,
        height = sel_height,
        fill = 1,
      })

      local footer_hints = {
        { "↑/↓", "navegar" },
        { "enter", "selecionar" },
        { "esc", "cancelar" },
      }

      return {
        type = "box",
        frame = chrome.frame("Talos — Seletor de Modelo / Agente CLI (F4)", level),
        children = {
          selector_widget,
          widgets.divider(width - 2),
          widgets.hints(footer_hints),
        },
      }
    end

    local display_messages = collect_messages(s)
    local message_rows = {}
    for _, msg in ipairs(display_messages) do
      local formatted = format_message(msg)
      for _, line in ipairs(formatted) do
        message_rows[#message_rows + 1] = line
      end
    end

    local list_height = math.max(1, height - 7)
    local messages_widget = widgets.list({
      rows = message_rows,
      selected = #message_rows > 0 and #message_rows or 1,
      height = list_height,
      fill = 1,
    })

    local input_box = {
      type = "box",
      axis = "horizontal",
      len = 1,
      children = {
        {
          type = "text",
          len = 4,
          text = " > ",
          style = { fg = theme.accent, bold = true },
        },
        {
          type = "input",
          fill = 1,
          value = s.input_field.value or "",
          cursor = s.input_field.cursor or 0,
          placeholder = "Mensagem para o Jev / Talos (ex: @spec-master gere o PRD, @dev implemente)...",
          focused = ctx.focused == true,
          style = { fg = theme.text },
        },
      },
    }

    local footer_hints = {
      { "enter", "send" },
      { "f4", "model selector" },
      { "ctrl+t", "new chat" },
      { "ctrl+r", "rename chat" },
      { "ctrl+a", "approve spec" },
      { "f2", "board" },
      { "f8", "shell" },
    }

    local active_ws = store.selected or s.active_workspace or (talos and talos.active_workspace) or "default"
    local active_th = s.active_thread or (talos and talos.active_thread) or "th-main"
    local s_name = active_session_name()
    local th_title = active_thread_title(s)
    local has_spec = s.has_spec_context or (talos and talos.has_spec_context)

    local strip, _ = chrome.central_tab_strip(width, border, "chat", chrome.rule(level))
    local right_title = string.format(" %s › %s (%s) ", s_name, th_title, s.selected_target)
    if has_spec then
      right_title = right_title .. "[Ctrl+A Spec Pronta] "
    end
    local frame = chrome.central_border_frame(right_title, level, border, strip)

    return {
      type = "box",
      frame = frame,
      children = {
        messages_widget,
        widgets.divider(width - 2),
        input_box,
        widgets.divider(width - 2),
        widgets.hints(footer_hints),
      },
    }
  end,

  on_action = function(action)
    local s = get_state()

    if action == "chat.open" then
      command("focus", { text = NAME, toggle = true })
      return true
    end

    if action == "chat.selector_toggle" or action == "chat.select_model" then
      s.selector_open = not s.selector_open
      save_state(s)
      return true
    end

    if action == "chat.cancel" then
      if s.selector_open then
        s.selector_open = false
        save_state(s)
        return true
      end
      return false
    end

    if action == "chat.send" then
      if s.selector_open then
        local opt = SELECTOR_OPTIONS[s.selector_index]
        if opt then
          s.target_kind = opt.kind
          s.target_agent = opt.agent
          s.target_model = opt.model
          s.selected_target = opt.label
          s.selector_open = false
          save_state(s)
          command("message", { text = "Alvo selecionado: " .. opt.label, level = "info" })
        end
        return true
      end

      local val = s.input_field.value or ""
      if val:match("%S") then
        local active_ws = store.selected or s.active_workspace or (talos and talos.active_workspace) or "default"
        local active_th = s.active_thread or (talos and talos.active_thread) or "th-main"

        command("chat_send", {
          text = val,
          workspace = active_ws,
          thread = active_th,
          value = s.target_kind or "auto",
          agent = s.target_agent,
          model = s.target_model,
        })

        -- Local optimistic append
        s.messages[#s.messages + 1] = {
          role = "user",
          sender = "Operador",
          time = os.date("%H:%M"),
          content = val,
        }
        textinput.clear(s.input_field)
        s.scroll_offset = 0

        local lower = val:lower()
        if lower:find("prd") or lower:find("rfc") or lower:find("especifica") then
          s.has_spec_context = true
        end
        save_state(s)
      end
      return true
    end

    if action == "chat.approve_spec" then
      -- If there is no pending spec, fall through to textinput so Ctrl+A functions as move cursor to beginning of line
      if not s.has_spec_context and not (talos and talos.has_spec_context) then
        return false
      end

      local active_ws = store.selected or s.active_workspace or (talos and talos.active_workspace) or "default"
      local active_th = s.active_thread or (talos and talos.active_thread) or "th-main"

      command("chat_approve_spec", {
        workspace = active_ws,
        thread = active_th,
      })
      s.has_spec_context = false
      s.messages[#s.messages + 1] = {
        role = "system",
        sender = "📡 Control Plane",
        badge = "git commit",
        time = os.date("%H:%M"),
        content = "Especificação aprovada com sucesso! Tarefas desmembradas no Workspace Board [F2] e commitadas em code-documentation.",
      }
      save_state(s)
      return true
    end

    return false
  end,

  on_key = function(key)
    local s = get_state()
    local name = key.key or ""

    if s.selector_open then
      if name == "up" or name == "k" or key.char == "k" then
        s.selector_index = math.max(1, s.selector_index - 1)
        save_state(s)
        return true
      elseif name == "down" or name == "j" or key.char == "j" then
        s.selector_index = math.min(#SELECTOR_OPTIONS, s.selector_index + 1)
        save_state(s)
        return true
      elseif name == "enter" then
        local opt = SELECTOR_OPTIONS[s.selector_index]
        if opt then
          s.target_kind = opt.kind
          s.target_agent = opt.agent
          s.target_model = opt.model
          s.selected_target = opt.label
          s.selector_open = false
          save_state(s)
          command("message", { text = "Alvo selecionado: " .. opt.label, level = "info" })
        end
        return true
      elseif name == "esc" or name == "escape" then
        s.selector_open = false
        save_state(s)
        return true
      end
      return true
    end

    -- Model selector toggle (Ctrl+O or F4)
    if (key.ctrl and (name == "o" or key.char == "o")) or name == "f4" then
      s.selector_open = not s.selector_open
      save_state(s)
      return true
    end

    -- New chat in active session (Ctrl+T or Ctrl+N inside Chat)
    if key.ctrl and (name == "t" or key.char == "t" or name == "n" or key.char == "n") then
      local active_ws = store.selected or s.active_workspace or "default"
      local s_name = active_session_name()
      local count = 1
      for _, th in ipairs(talos and talos.threads or {}) do
        if th.workspace_id == active_ws then
          count = count + 1
        end
      end
      local new_id = "th-" .. tostring(os.time())
      s.active_thread = new_id
      s.thread_title = "Chat " .. tostring(count)
      s.messages = {
        {
          role = "system",
          sender = "⚡ Jev Engine",
          badge = "Nova Conversa",
          time = os.date("%H:%M"),
          content = "Novo chat iniciado (" .. s.thread_title .. ") na sessão " .. s_name .. ". Digite seu objetivo.",
        },
      }
      s.has_spec_context = false
      s.scroll_offset = 0
      save_state(s)
      command("message", { text = "Novo chat: " .. s.thread_title .. " em " .. s_name, level = "info" })
      return true
    end

    -- Rename active chat (Ctrl+R inside Chat)
    if key.ctrl and (name == "r" or key.char == "r") then
      local current = active_thread_title(s)
      local val = s.input_field.value or ""
      local new_title = (val:match("%S") and val) or (current .. " (ativo)")
      s.thread_title = new_title
      textinput.clear(s.input_field)
      save_state(s)
      command("chat_rename_thread", { thread = s.active_thread, text = new_title })
      command("message", { text = "Chat renomeado para: " .. new_title, level = "info" })
      return true
    end

    -- Scroll through messages
    if name == "pageup" or (key.ctrl and name == "u") then
      s.scroll_offset = (s.scroll_offset or 0) + 5
      save_state(s)
      return true
    elseif name == "pagedown" or (key.ctrl and name == "d") then
      s.scroll_offset = math.max(0, (s.scroll_offset or 0) - 5)
      save_state(s)
      return true
    end

    local consumed = textinput.key(s.input_field, key)
    if consumed then
      save_state(s)
      return true
    end

    return false
  end,
}
