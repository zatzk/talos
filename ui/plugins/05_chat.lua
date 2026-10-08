-- Talos Chat — The native conversational orchestrator interface (F1).
--
-- The primary operator view of Talos v3:
-- 1. Conversation threads per workspace.
-- 2. Message stream history with multi-calling & Jev Intent badges.
-- 3. Jev Intent routing (Auto persona, tier, backend: API vs CLI).
-- 4. Multi-agent mentions (@architect, @dev, @qa, @spec-master, @sec).
-- 5. Model/Agent selector modal (F4 / Ctrl+O).
-- 6. Quick action to approve specs and commit to code-documentation (Ctrl+A).

local chrome = require("lib.chrome")
local theme = require("lib.theme")
local widgets = require("lib.widgets")
local textinput = require("lib.textinput")

local NAME = "chat"

local SELECTOR_OPTIONS = {
  { label = "⚡ Auto (Jev Intent Router)", desc = "Auto-routing: persona, tier e backend (API/CLI)", kind = "auto", agent = nil, model = nil },
  { label = "🏛️ Architect (@architect)", desc = "API: claude-3.7-sonnet — Design de RFC e Especificação", kind = "api", agent = "architect", model = "claude-3.7-sonnet" },
  { label = "🛠️ Dev Worker (@dev)", desc = "API: claude-3.7-sonnet — Implementação de código autônoma", kind = "api", agent = "dev", model = "claude-3.7-sonnet" },
  { label = "🤖 Dev Agent CLI (@dev)", desc = "CLI: agy headless — Sessão interativa em tmux", kind = "cli", agent = "dev", model = "agy" },
  { label = "🛡️ QA Engineer (@qa)", desc = "API: claude-3.7-sonnet — Testes e auto-healing QA", kind = "api", agent = "qa", model = "claude-3.7-sonnet" },
  { label = "📋 Spec Master (@spec-master)", desc = "API: claude-3.7-sonnet — Pipeline de PRDs e governança", kind = "api", agent = "spec-master", model = "claude-3.7-sonnet" },
  { label = "🔒 Security Reviewer (@sec)", desc = "API: claude-3.7-sonnet — Auditoria OWASP e segredos", kind = "api", agent = "sec", model = "claude-3.7-sonnet" },
}

local function chat_store()
  store.chat_state = store.chat_state or {
    active_workspace = "default",
    active_thread = "th-main",
    input_field = textinput.new(""),
    messages = {
      {
        role = "system",
        sender = "⚡ Jev Engine",
        badge = "Talos v3 Control Plane",
        time = "Agora",
        content = "Talos v3 pronto. Workspace ativo: principal.\nAtalhos: [F1] Chat, [F2] Board Kanban, [F3] Fleet, [F4/Ctrl+O] Seletor de Modelo, [Ctrl+A] Aprovar Spec.",
      },
    },
    streaming = false,
    has_spec_context = false,
    selected_target = "Auto (Jev)",
    target_kind = "auto",
    target_agent = nil,
    target_model = nil,
    selector_open = false,
    selector_index = 1,
  }
  if type(store.chat_state.input_field) ~= "table" or type(store.chat_state.input_field.value) ~= "string" then
    store.chat_state.input_field = textinput.new("")
  end
  return store.chat_state
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

local function format_message(msg, is_last, width)
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

local function collect_messages(state)
  local msgs = {}
  if talos and talos.chat_messages and #talos.chat_messages > 0 then
    local active_th = state.active_thread or "th-main"
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
    return state.messages
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
    { key = "ctrl+o", action = "chat.selector_toggle", desc = "model selector", group = "Chat" },
    { key = "enter", action = "chat.send", desc = "send message", group = "Chat" },
    { key = "ctrl+n", action = "chat.new_thread", desc = "new thread", group = "Chat" },
    { key = "ctrl+a", action = "chat.approve_spec", desc = "approve spec & commit", group = "Chat" },
    { key = "up", action = "chat.up", desc = "navigate up", group = "Chat" },
    { key = "down", action = "chat.down", desc = "navigate down", group = "Chat" },
    { key = "esc", action = "chat.cancel", desc = "cancel selector", group = "Chat" },
  },

  render = function(ctx)
    local state = chat_store()
    local width, height = ctx.width or 80, ctx.height or 24
    local level = chrome.level(ctx.focused)

    -- If selector is open, render model/agent selector modal overlay
    if state.selector_open then
      local rows = {}
      rows[#rows + 1] = {
        spans = {
          { text = " Escolha o Alvo / Persona / Modelo para o Chat:", style = { fg = theme.accent, bold = true } },
        },
      }
      rows[#rows + 1] = { spans = { { text = "", style = {} } } }

      for i, opt in ipairs(SELECTOR_OPTIONS) do
        local is_sel = (i == state.selector_index)
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
        selected = state.selector_index + 2,
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
        frame = chrome.frame("Talos — Seletor de Modelo / Agente (F4)", level),
        children = {
          selector_widget,
          widgets.divider(width - 2),
          widgets.hints(footer_hints),
        },
      }
    end

    local display_messages = collect_messages(state)
    local message_rows = {}
    for i, msg in ipairs(display_messages) do
      local formatted = format_message(msg, i == #display_messages, width)
      for _, line in ipairs(formatted) do
        message_rows[#message_rows + 1] = line
      end
    end

    local list_height = math.max(1, height - 7)
    local messages_widget = widgets.list({
      rows = message_rows,
      selected = #message_rows,
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
          text = { { { text = " > ", style = { fg = theme.accent, bold = true } } } },
        },
        {
          type = "input",
          fill = 1,
          value = state.input_field.value or "",
          cursor = state.input_field.cursor or 0,
          placeholder = "Mensagem para o Jev / Talos (ex: @architect @qa analise a RFC-001)...",
          focused = ctx.focused == true,
          style = { fg = theme.text },
        },
      },
    }

    local footer_hints = {
      { "enter", "send" },
      { "ctrl+a", "approve spec" },
      { "f4", "selector" },
      { "ctrl+n", "new thread" },
      { "f2", "board" },
      { "f3", "fleet" },
    }

    local active_ws = state.active_workspace or (talos and talos.active_workspace) or "default"
    local active_th = state.active_thread or (talos and talos.active_thread) or "th-main"
    local has_spec = state.has_spec_context or (talos and talos.has_spec_context)

    local header_info = "Thread: " .. active_th .. " │ Target: " .. state.selected_target
    if has_spec then
      header_info = header_info .. " │ [Ctrl+A] Spec pronta para aprovação!"
    end

    return {
      type = "box",
      frame = chrome.frame("Talos Chat — " .. header_info, level),
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
    local state = chat_store()

    if action == "chat.open" then
      command("focus", { text = NAME, toggle = true })
      return true
    end

    if action == "chat.selector_toggle" or action == "chat.select_model" then
      state.selector_open = not state.selector_open
      return true
    end

    if action == "chat.cancel" then
      if state.selector_open then
        state.selector_open = false
        return true
      end
      return false
    end

    if action == "chat.up" then
      if state.selector_open then
        state.selector_index = math.max(1, state.selector_index - 1)
        return true
      end
      return false
    end

    if action == "chat.down" then
      if state.selector_open then
        state.selector_index = math.min(#SELECTOR_OPTIONS, state.selector_index + 1)
        return true
      end
      return false
    end

    if action == "chat.send" then
      if state.selector_open then
        local opt = SELECTOR_OPTIONS[state.selector_index]
        if opt then
          state.target_kind = opt.kind
          state.target_agent = opt.agent
          state.target_model = opt.model
          state.selected_target = opt.label
          state.selector_open = false
          command("message", { text = "Alvo selecionado: " .. opt.label, level = "info" })
        end
        return true
      end

      local val = state.input_field.value or ""
      if val:match("%S") then
        local active_ws = state.active_workspace or (talos and talos.active_workspace) or "default"
        local active_th = state.active_thread or (talos and talos.active_thread) or "th-main"

        command("chat_send", {
          text = val,
          workspace = active_ws,
          thread = active_th,
          value = state.target_kind or "auto",
          agent = state.target_agent,
          model = state.target_model,
        })

        -- Local optimistic append
        state.messages[#state.messages + 1] = {
          role = "user",
          sender = "Operador",
          time = os.date("%H:%M"),
          content = val,
        }
        textinput.clear(state.input_field)

        local lower = val:lower()
        if lower:find("prd") or lower:find("rfc") or lower:find("especifica") then
          state.has_spec_context = true
        end
      end
      return true
    end

    if action == "chat.new_thread" then
      local new_id = "th-" .. tostring(os.time())
      state.active_thread = new_id
      state.messages = {
        {
          role = "system",
          sender = "⚡ Jev Engine",
          badge = "Nova Thread",
          time = os.date("%H:%M"),
          content = "Nova conversa iniciada: " .. new_id .. ". Digite seu objetivo.",
        },
      }
      state.has_spec_context = false
      command("message", { text = "Nova conversa iniciada: " .. new_id, level = "info" })
      return true
    end

    if action == "chat.approve_spec" then
      local active_ws = state.active_workspace or (talos and talos.active_workspace) or "default"
      local active_th = state.active_thread or (talos and talos.active_thread) or "th-main"

      command("chat_approve_spec", {
        workspace = active_ws,
        thread = active_th,
      })
      state.has_spec_context = false
      state.messages[#state.messages + 1] = {
        role = "system",
        sender = "📡 Control Plane",
        badge = "git commit",
        time = os.date("%H:%M"),
        content = "Especificação aprovada com sucesso! Tarefas desmembradas no Workspace Board [F2] e commitadas em code-documentation.",
      }
      return true
    end

    return false
  end,

  on_key = function(key)
    local state = chat_store()
    if state.selector_open then
      if key == "up" or key == "k" then
        state.selector_index = math.max(1, state.selector_index - 1)
        return true
      elseif key == "down" or key == "j" then
        state.selector_index = math.min(#SELECTOR_OPTIONS, state.selector_index + 1)
        return true
      elseif key == "enter" then
        local opt = SELECTOR_OPTIONS[state.selector_index]
        if opt then
          state.target_kind = opt.kind
          state.target_agent = opt.agent
          state.target_model = opt.model
          state.selected_target = opt.label
          state.selector_open = false
          command("message", { text = "Alvo selecionado: " .. opt.label, level = "info" })
        end
        return true
      elseif key == "esc" or key == "escape" then
        state.selector_open = false
        return true
      end
      return true
    end
    return textinput.key(state.input_field, key)
  end,
}
