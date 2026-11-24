(() => {
  const state = {
    providers: [],
    providerModelsCache: {},
    config: null,
    sessions: [],
    activeSessionId: null,
    deviceWidth: 1080,
    deviceHeight: 2400,
    videoWs: null,
    audioWs: null,
    eventsWs: null,
    audioEnabled: false,
    audioCtx: null,
    audioNextTime: 0,
    audioSampleRate: 16000,
    audioChannels: 1,
    lastFrameUrl: null,
    liveTraces: [],
  };

  const els = {
    badgeAccessibility: document.getElementById("badge-accessibility"),
    badgeVideo: document.getElementById("badge-video"),
    btnAudioToggle: document.getElementById("btn-audio-toggle"),
    btnOpenSettings: document.getElementById("btn-open-settings"),
    btnLogout: document.getElementById("btn-logout"),
    btnCloseSettings: document.getElementById("btn-close-settings"),
    btnWarnSettings: document.getElementById("btn-warn-settings"),
    btnNewSession: document.getElementById("btn-new-session"),
    sessionList: document.getElementById("session-list"),
    sessionProvider: document.getElementById("session-provider"),
    sessionModelSelect: document.getElementById("session-model-select"),
    btnRefreshModels: document.getElementById("btn-refresh-models"),
    modelSourceHint: document.getElementById("model-source-hint"),
    keyWarning: document.getElementById("key-warning"),
    chatMessages: document.getElementById("chat-messages"),
    agentActivity: document.getElementById("agent-activity"),
    agentActivityText: document.getElementById("agent-activity-text"),
    btnStopAgent: document.getElementById("btn-stop-agent"),
    chatForm: document.getElementById("chat-form"),
    chatInput: document.getElementById("chat-input"),
    screenImg: document.getElementById("screen-img"),
    screenPlaceholder: document.getElementById("screen-placeholder"),
    deviceDims: document.getElementById("device-dims"),
    touchRipple: document.getElementById("touch-ripple"),
    btnRefreshFrame: document.getElementById("btn-refresh-frame"),
    btnInspectUi: document.getElementById("btn-inspect-ui"),
    quickTypeForm: document.getElementById("quick-type-form"),
    quickTypeInput: document.getElementById("quick-type-input"),
    settingsModal: document.getElementById("settings-modal"),
    addKeyForm: document.getElementById("add-key-form"),
    addKeyProvider: document.getElementById("add-key-provider"),
    addKeyInput: document.getElementById("add-key-input"),
    configuredKeysList: document.getElementById("configured-keys-list"),
    keysSaveMsg: document.getElementById("keys-save-msg"),
    tabPanelKeys: document.getElementById("tab-panel-keys"),
    tabPanelModels: document.getElementById("tab-panel-models"),
    tabPanelPrompt: document.getElementById("tab-panel-prompt"),
    tabPanelMemory: document.getElementById("tab-panel-memory"),
    appMemoryList: document.getElementById("app-memory-list"),
    appMemoryForm: document.getElementById("app-memory-form"),
    memPackage: document.getElementById("mem-package"),
    memLabel: document.getElementById("mem-label"),
    memNotes: document.getElementById("mem-notes"),
    memSaveMsg: document.getElementById("mem-save-msg"),
    tabPanelTelegram: document.getElementById("tab-panel-telegram"),
    tabPanelSecurity: document.getElementById("tab-panel-security"),
    tgStatusBadge: document.getElementById("tg-status-badge"),
    tgEnabled: document.getElementById("tg-enabled"),
    tgBotToken: document.getElementById("tg-bot-token"),
    tgTokenStatus: document.getElementById("tg-token-status"),
    tgAllowedIds: document.getElementById("tg-allowed-ids"),
    tgSaveMsg: document.getElementById("tg-save-msg"),
    secAuthBadge: document.getElementById("sec-auth-badge"),
    secMfaBadge: document.getElementById("sec-mfa-badge"),
    secCredentialsForm: document.getElementById("sec-credentials-form"),
    secUsername: document.getElementById("sec-username"),
    secPassword: document.getElementById("sec-password"),
    btnSaveCredentials: document.getElementById("btn-save-credentials"),
    btnDisableAuth: document.getElementById("btn-disable-auth"),
    secMfaCard: document.getElementById("sec-mfa-card"),
    btnSetupMfa: document.getElementById("btn-setup-mfa"),
    btnDisableMfa: document.getElementById("btn-disable-mfa"),
    secMfaSetupBox: document.getElementById("sec-mfa-setup-box"),
    secMfaQr: document.getElementById("sec-mfa-qr"),
    secMfaSecret: document.getElementById("sec-mfa-secret"),
    secMfaVerifyForm: document.getElementById("sec-mfa-verify-form"),
    secMfaCode: document.getElementById("sec-mfa-code"),
    secMsg: document.getElementById("sec-msg"),
    loginOverlay: document.getElementById("login-overlay"),
    loginForm: document.getElementById("login-form"),
    loginUsername: document.getElementById("login-username"),
    loginPassword: document.getElementById("login-password"),
    loginMfaField: document.getElementById("login-mfa-field"),
    loginTotp: document.getElementById("login-totp"),
    loginError: document.getElementById("login-error"),
    cfgDefaultProvider: document.getElementById("cfg-default-provider"),
    cfgDefaultModel: document.getElementById("cfg-default-model"),
    cfgModelSourceHint: document.getElementById("cfg-model-source-hint"),
    btnRefreshDefaultModels: document.getElementById("btn-refresh-default-models"),
    settingsSaveMsg: document.getElementById("settings-save-msg"),
    promptSaveMsg: document.getElementById("prompt-save-msg"),
    uitreeModal: document.getElementById("uitree-modal"),
    btnCloseUitree: document.getElementById("btn-close-uitree"),
    uitreePre: document.getElementById("uitree-pre"),
  };

  let appBooted = false;
  let statusInterval = null;

  async function api(path, opts = {}) {
    const res = await fetch(path, {
      credentials: "same-origin",
      headers: { "Content-Type": "application/json", ...(opts.headers || {}) },
      ...opts,
    });
    if (res.status === 401 && !path.startsWith("/api/auth/")) {
      showLoginScreen();
      throw new Error("Authentication required");
    }
    const data = await res.json().catch(() => ({}));
    if (!res.ok) {
      const err = new Error(data.error || `HTTP ${res.status}`);
      err.data = data;
      err.status = res.status;
      throw err;
    }
    return data;
  }

  function wsUrl(path) {
    const proto = location.protocol === "https:" ? "wss:" : "ws:";
    return `${proto}//${location.host}${path}`;
  }

  function getActiveSession() {
    return state.sessions.find((s) => s.id === state.activeSessionId) || state.sessions[0] || null;
  }

  function populateModels(providerId, selectedModel, modelsOverride = null, source = null) {
    const cached = state.providerModelsCache[providerId];
    const p = state.providers.find((x) => x.id === providerId);
    const models = modelsOverride || cached?.models || (p ? p.models : []);
    const effSource = source || cached?.source || "fallback";

    els.sessionModelSelect.innerHTML = "";
    let hasSelected = false;
    for (const m of models) {
      const opt = document.createElement("option");
      opt.value = m;
      opt.textContent = m;
      if (m === selectedModel) {
        opt.selected = true;
        hasSelected = true;
      }
      els.sessionModelSelect.appendChild(opt);
    }

    if (selectedModel && !hasSelected) {
      const opt = document.createElement("option");
      opt.value = selectedModel;
      opt.textContent = selectedModel;
      opt.selected = true;
      els.sessionModelSelect.prepend(opt);
    }

    if (effSource === "api") {
      els.modelSourceHint.textContent = `${models.length} models`;
      els.modelSourceHint.className = "model-source-hint live";
      els.modelSourceHint.title = "Fetched live from provider API";
    } else if (effSource === "loading") {
      els.modelSourceHint.textContent = "Loading...";
      els.modelSourceHint.className = "model-source-hint";
      els.modelSourceHint.title = "Fetching models from provider API";
    } else {
      els.modelSourceHint.textContent = "Add API key";
      els.modelSourceHint.className = "model-source-hint";
      els.modelSourceHint.title = cached?.error || "Configure API key to fetch live models";
    }
  }

  async function fetchModelsForProvider(providerId, force = false) {
    const session = getActiveSession();
    if (!force && state.providerModelsCache[providerId]?.source === "api") {
      populateModels(providerId, session?.model);
      return state.providerModelsCache[providerId];
    }

    populateModels(providerId, session?.model, null, "loading");
    try {
      const data = await api(`/api/providers/${encodeURIComponent(providerId)}/models`);
      state.providerModelsCache[providerId] = data;
      const active = getActiveSession();
      if (active && active.provider === providerId) {
        const models = data.models || [];
        let targetModel = active.model;
        if (models.length > 0 && !models.includes(targetModel)) {
          targetModel = models[0];
          await api(`/api/sessions/${active.id}`, {
            method: "PUT",
            body: JSON.stringify({ model: targetModel }),
          });
          active.model = targetModel;
          renderSessions();
        }
        populateModels(providerId, targetModel, models, data.source);
      }
      return data;
    } catch (err) {
      const fallback = { ok: false, source: "fallback", error: err.message, models: [] };
      state.providerModelsCache[providerId] = fallback;
      const active = getActiveSession();
      if (active && active.provider === providerId) {
        populateModels(providerId, active.model, null, "fallback");
      }
      return fallback;
    }
  }

  function updateKeyWarning() {
    const session = getActiveSession();
    if (!session || !state.config) return;
    const configured = state.config.api_keys_configured?.[session.provider];
    els.keyWarning.classList.toggle("hidden", !!configured);
  }

  function renderSessions() {
    els.sessionList.innerHTML = "";
    for (const s of state.sessions) {
      const isTelegram = s.source === "telegram" || !!s.telegram_chat_id;
      const tgBadge = isTelegram
        ? `<span class="session-source-badge" title="Telegram Bot Session">TG</span>`
        : "";
      const item = document.createElement("div");
      item.className = `session-item ${s.id === state.activeSessionId ? "active" : ""}`;
      item.innerHTML = `
        <div class="session-meta">
          <div class="session-title-row">${tgBadge}<span class="session-title">${escapeHtml(s.title)}</span></div>
          <div class="session-sub">${escapeHtml(s.provider)} · ${escapeHtml(s.model)}</div>
        </div>
        <button class="session-del" title="Delete session">&times;</button>
      `;
      item.querySelector(".session-meta").addEventListener("click", () => {
        state.activeSessionId = s.id;
        renderAll();
      });
      item.querySelector(".session-del").addEventListener("click", async (e) => {
        e.stopPropagation();
        await api(`/api/sessions/${s.id}`, { method: "DELETE" });
        await loadSessions();
      });
      els.sessionList.appendChild(item);
    }
  }

  function renderInlineMarkdown(text) {
    const codeTokens = [];
    let s = escapeHtml(text || "");

    // Protect inline code spans first
    s = s.replace(/`([^`\n]+)`/g, (_, code) => {
      const idx = codeTokens.length;
      codeTokens.push(`<code>${code}</code>`);
      return `\u0000CODE${idx}\u0000`;
    });

    // Images ![alt](/api/media/... or https://...)
    s = s.replace(
      /!\[([^\]]*)\]\(((?:\/api\/media\/|https?:\/\/)[^\s)]+)\)/g,
      (_, alt, url) => {
        const idx = codeTokens.length;
        codeTokens.push(
          `<a class="md-image-link" href="${url}" target="_blank" rel="noopener noreferrer"><img class="md-image" src="${url}" alt="${alt}" loading="lazy" /></a>`
        );
        return `\u0000CODE${idx}\u0000`;
      }
    );

    // Links [text](https://...)
    s = s.replace(/\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g, (_, label, url) => {
      return `<a href="${url}" target="_blank" rel="noopener noreferrer">${label}</a>`;
    });

    // Bold + Italic, Bold, Italic, Strikethrough
    s = s.replace(/\*\*\*([^*]+)\*\*\*/g, "<strong><em>$1</em></strong>");
    s = s.replace(/\*\*([^*\n]+)\*\*/g, "<strong>$1</strong>");
    s = s.replace(/__([^_\n]+)__/g, "<strong>$1</strong>");
    s = s.replace(/(^|[^\*])\*([^*\n]+)\*(?!\*)/g, "$1<em>$2</em>");
    s = s.replace(/(^|[^_])_([^_\n]+)_(?!_)/g, "$1<em>$2</em>");
    s = s.replace(/~~([^~\n]+)~~/g, "<del>$1</del>");

    // Restore inline code spans
    s = s.replace(/\u0000CODE(\d+)\u0000/g, (_, idx) => codeTokens[Number(idx)] || "");
    return s;
  }

  function renderMarkdown(md) {
    const lines = String(md || "").replace(/\r\n/g, "\n").split("\n");
    const out = [];
    let i = 0;

    while (i < lines.length) {
      const line = lines[i];

      // Fenced code block
      const fence = line.match(/^```(\w*)\s*$/);
      if (fence) {
        const lang = escapeHtml(fence[1] || "");
        const codeLines = [];
        i++;
        while (i < lines.length && !/^```\s*$/.test(lines[i])) {
          codeLines.push(escapeHtml(lines[i]));
          i++;
        }
        i++; // skip closing fence
        const langAttr = lang ? ` data-lang="${lang}"` : "";
        out.push(`<pre${langAttr}><code>${codeLines.join("\n")}</code></pre>`);
        continue;
      }

      // Blank line
      if (/^\s*$/.test(line)) {
        i++;
        continue;
      }

      // Horizontal rule
      if (/^\s*([-*_])\1{2,}\s*$/.test(line)) {
        out.push("<hr />");
        i++;
        continue;
      }

      // Heading
      const heading = line.match(/^(#{1,6})\s+(.+)$/);
      if (heading) {
        const level = heading[1].length;
        out.push(`<h${level}>${renderInlineMarkdown(heading[2].trim())}</h${level}>`);
        i++;
        continue;
      }

      // Blockquote
      if (/^\s*>\s?/.test(line)) {
        const qLines = [];
        while (i < lines.length && /^\s*>\s?/.test(lines[i])) {
          qLines.push(lines[i].replace(/^\s*>\s?/, ""));
          i++;
        }
        out.push(`<blockquote>${renderMarkdown(qLines.join("\n"))}</blockquote>`);
        continue;
      }

      // GFM Table
      if (
        line.includes("|") &&
        i + 1 < lines.length &&
        /^\s*\|?(\s*:?-+:?\s*\|)+\s*:?-+:?\s*\|?\s*$/.test(lines[i + 1])
      ) {
        const parseRow = (r) =>
          r
            .trim()
            .replace(/^\|/, "")
            .replace(/\|$/, "")
            .split("|")
            .map((c) => c.trim());
        const headers = parseRow(line);
        i += 2; // skip header & separator
        const rows = [];
        while (i < lines.length && lines[i].includes("|") && !/^\s*$/.test(lines[i])) {
          rows.push(parseRow(lines[i]));
          i++;
        }
        const thead = `<thead><tr>${headers.map((h) => `<th>${renderInlineMarkdown(h)}</th>`).join("")}</tr></thead>`;
        const tbody = `<tbody>${rows
          .map((r) => `<tr>${r.map((c) => `<td>${renderInlineMarkdown(c)}</td>`).join("")}</tr>`)
          .join("")}</tbody>`;
        out.push(`<div class="md-table-wrap"><table>${thead}${tbody}</table></div>`);
        continue;
      }

      // Unordered list
      if (/^\s*[-*+]\s+/.test(line)) {
        const items = [];
        while (i < lines.length && /^\s*[-*+]\s+/.test(lines[i])) {
          items.push(`<li>${renderInlineMarkdown(lines[i].replace(/^\s*[-*+]\s+/, ""))}</li>`);
          i++;
        }
        out.push(`<ul>${items.join("")}</ul>`);
        continue;
      }

      // Ordered list
      if (/^\s*\d+\.\s+/.test(line)) {
        const items = [];
        while (i < lines.length && /^\s*\d+\.\s+/.test(lines[i])) {
          items.push(`<li>${renderInlineMarkdown(lines[i].replace(/^\s*\d+\.\s+/, ""))}</li>`);
          i++;
        }
        out.push(`<ol>${items.join("")}</ol>`);
        continue;
      }

      // Paragraph
      const pLines = [];
      while (
        i < lines.length &&
        !/^\s*$/.test(lines[i]) &&
        !/^```/.test(lines[i]) &&
        !/^#{1,6}\s+/.test(lines[i]) &&
        !/^\s*[-*+]\s+/.test(lines[i]) &&
        !/^\s*\d+\.\s+/.test(lines[i]) &&
        !/^\s*>\s?/.test(lines[i]) &&
        !/^\s*([-*_])\1{2,}\s*$/.test(lines[i])
      ) {
        pLines.push(renderInlineMarkdown(lines[i]));
        i++;
      }
      if (pLines.length > 0) {
        out.push(`<p>${pLines.join("<br />")}</p>`);
      }
    }

    return out.join("");
  }

  function parseErrorDisplay(raw) {
    let text = String(raw || "").trim().replace(/^Error:\s*/i, "");
    let title = "Request failed";
    let message = text;
    let details = null;

    const jsonIdx = text.indexOf("{");
    if (jsonIdx !== -1) {
      const prefix = text.slice(0, jsonIdx).replace(/:\s*$/, "").trim();
      const jsonPart = text.slice(jsonIdx).trim();
      try {
        const parsed = JSON.parse(jsonPart);
        const extracted =
          parsed?.error?.message || parsed?.message || parsed?.detail || null;
        if (extracted) {
          if (prefix) title = prefix;
          message = String(extracted).trim();
          details = JSON.stringify(parsed, null, 2);
          return { title, message, details };
        }
      } catch {}
    }

    const colonIdx = text.indexOf(": ");
    if (colonIdx > 0 && colonIdx < 48) {
      title = text.slice(0, colonIdx).trim();
      message = text.slice(colonIdx + 2).trim();
    }
    return { title, message, details };
  }

  function renderMessages() {
    const session = getActiveSession();
    els.chatMessages.innerHTML = "";
    if (!session) return;

    if (session.messages.length === 0) {
      const empty = document.createElement("div");
      empty.className = "msg welcome";
      empty.innerHTML = `<img src="/logo.png" alt="Agentic Android" class="welcome-logo" /><h3>Agentic Android</h3><p>Describe what you want done on your device, or interact with the live screen on the right.</p>`;
      els.chatMessages.appendChild(empty);
      return;
    }

    for (const m of session.messages) {
      const isError = m.role === "error" || (m.role === "assistant" && /^Error:\s*/i.test(m.content || ""));
      const div = document.createElement("div");
      div.className = `msg ${isError ? "error" : m.role}`;

      if (isError) {
        const err = parseErrorDisplay(m.content);
        const detailsHtml = err.details
          ? `<details class="error-details"><summary>Raw details</summary><pre>${escapeHtml(err.details)}</pre></details>`
          : "";
        div.innerHTML = `
          <div class="error-card">
            <div class="error-head">
              <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
              <span>${escapeHtml(err.title)}</span>
            </div>
            <div class="error-body">${escapeHtml(err.message)}</div>
            ${detailsHtml}
          </div>`;
        els.chatMessages.appendChild(div);
        continue;
      }

      let tracesHtml = "";
      if (m.tool_traces && m.tool_traces.length > 0) {
        const items = m.tool_traces
          .map(
            (t) =>
              `<div class="trace-item"><strong>${escapeHtml(t.tool_name)}</strong>(${escapeHtml(
                JSON.stringify(t.arguments)
              )})<br/>→ ${escapeHtml(t.result)}</div>`
          )
          .join("");
        tracesHtml = `<details class="traces"><summary>${m.tool_traces.length} device action${m.tool_traces.length === 1 ? "" : "s"}</summary><div class="trace-list">${items}</div></details>`;
      }
      const bodyHtml =
        m.role === "assistant"
          ? `<div class="msg-body md-content">${renderMarkdown(m.content)}</div>`
          : `<div class="msg-body user-text">${escapeHtml(m.content)}</div>`;
      div.innerHTML = `${bodyHtml}${tracesHtml}`;
      els.chatMessages.appendChild(div);
    }
    els.chatMessages.scrollTop = els.chatMessages.scrollHeight;
  }

  function renderAll() {
    const session = getActiveSession();
    if (session) {
      state.activeSessionId = session.id;
      els.sessionProvider.value = session.provider;
      populateModels(session.provider, session.model);
      if (!state.providerModelsCache[session.provider]) {
        fetchModelsForProvider(session.provider);
      }
    }
    renderSessions();
    renderMessages();
    updateKeyWarning();
  }

  const PROVIDER_LABELS = {
    claude: "Claude (Anthropic)",
    gemini: "Gemini (Google)",
    grok: "Grok (xAI)",
    openai: "OpenAI",
  };

  function renderConfiguredKeys() {
    const masked = state.config?.api_keys_masked || {};
    const configured = ["claude", "gemini", "grok", "openai"].filter((id) => !!masked[id]);
    els.configuredKeysList.innerHTML = "";

    if (configured.length === 0) {
      const empty = document.createElement("div");
      empty.className = "keys-empty";
      empty.textContent = "No provider API keys added yet. Select a provider below to add one.";
      els.configuredKeysList.appendChild(empty);
      return;
    }

    for (const id of configured) {
      const row = document.createElement("div");
      row.className = "key-row";
      row.innerHTML = `
        <div class="key-row-info">
          <span class="key-row-name">${escapeHtml(PROVIDER_LABELS[id] || id)}</span>
          <span class="key-row-mask">${escapeHtml(masked[id])}</span>
        </div>
        <button type="button" class="pill-btn pill-btn-danger">Remove</button>
      `;
      row.querySelector("button").addEventListener("click", async () => {
        await api("/api/config", {
          method: "PUT",
          body: JSON.stringify({ api_keys: { [id]: "" } }),
        });
        state.providerModelsCache = {};
        await loadConfig();
        const active = getActiveSession();
        if (active && active.provider === id) {
          await fetchModelsForProvider(id, true);
        }
      });
      els.configuredKeysList.appendChild(row);
    }
  }

  function populateDefaultModels(providerId, selectedModel, modelsOverride = null, source = null) {
    const cached = state.providerModelsCache[providerId];
    const p = state.providers.find((x) => x.id === providerId);
    const models = modelsOverride || cached?.models || (p ? p.models : []);
    const effSource = source || cached?.source || "fallback";

    els.cfgDefaultModel.innerHTML = "";
    let hasSelected = false;
    for (const m of models) {
      const opt = document.createElement("option");
      opt.value = m;
      opt.textContent = m;
      if (m === selectedModel) {
        opt.selected = true;
        hasSelected = true;
      }
      els.cfgDefaultModel.appendChild(opt);
    }

    if (selectedModel && !hasSelected) {
      const opt = document.createElement("option");
      opt.value = selectedModel;
      opt.textContent = selectedModel;
      opt.selected = true;
      els.cfgDefaultModel.prepend(opt);
    }

    if (effSource === "api") {
      els.cfgModelSourceHint.textContent = `${models.length} models`;
      els.cfgModelSourceHint.className = "model-source-hint live";
    } else if (effSource === "loading") {
      els.cfgModelSourceHint.textContent = "Loading...";
      els.cfgModelSourceHint.className = "model-source-hint";
    } else {
      els.cfgModelSourceHint.textContent = "Add API key";
      els.cfgModelSourceHint.className = "model-source-hint";
    }
  }

  async function fetchDefaultModels(providerId, preferredModel = null, force = false) {
    if (!force && state.providerModelsCache[providerId]?.source === "api") {
      const models = state.providerModelsCache[providerId].models || [];
      const target =
        preferredModel && models.includes(preferredModel)
          ? preferredModel
          : models[0] || preferredModel || "";
      populateDefaultModels(providerId, target, models, "api");
      return;
    }

    populateDefaultModels(providerId, preferredModel, null, "loading");
    try {
      const data = await api(`/api/providers/${encodeURIComponent(providerId)}/models`);
      state.providerModelsCache[providerId] = data;
      if (els.cfgDefaultProvider.value === providerId) {
        const models = data.models || [];
        const target =
          preferredModel && models.includes(preferredModel)
            ? preferredModel
            : models[0] || preferredModel || "";
        populateDefaultModels(providerId, target, models, data.source);
      }
    } catch (err) {
      populateDefaultModels(providerId, preferredModel, null, "fallback");
    }
  }

  async function loadConfig() {
    state.config = await api("/api/config");
    renderConfiguredKeys();
    els.cfgDefaultProvider.value = state.config.default_provider;
    populateDefaultModels(state.config.default_provider, state.config.default_model);
    document.getElementById("cfg-max-steps").value = state.config.max_agent_steps;
    document.getElementById("cfg-step-delay").value = state.config.step_delay_ms;
    document.getElementById("cfg-include-screenshot").checked = !!state.config.include_screenshot;
    document.getElementById("cfg-gemini-search").checked = state.config.gemini_google_search !== false;
    document.getElementById("cfg-gemini-maps").checked = state.config.gemini_google_maps !== false;
    document.getElementById("cfg-system-prompt").value = state.config.system_prompt || "";
    const tg = state.config.telegram || {};
    els.tgEnabled.checked = !!tg.enabled;
    els.tgTokenStatus.textContent = tg.token_masked ? `Saved (${tg.token_masked})` : "Not set";
    els.tgAllowedIds.value = (tg.allowed_chat_ids || []).join(", ");
    const tgStatus = tg.status || "Disabled";
    const tgOk = tgStatus.startsWith("Connected");
    const tgWarn = tg.enabled && !tgOk;
    els.tgStatusBadge.className = `status-pill ${tgOk ? "ok" : tgWarn ? "warn" : ""}`;
    els.tgStatusBadge.querySelector(".label").textContent = tgStatus;
    updateKeyWarning();
  }

  async function loadSessions() {
    state.sessions = await api("/api/sessions");
    if (!state.activeSessionId || !state.sessions.some((s) => s.id === state.activeSessionId)) {
      state.activeSessionId = state.sessions[0]?.id || null;
    }
    renderAll();
  }

  async function refreshStatus() {
    try {
      const st = await api("/api/status");
      const accEnabled = !!st.device?.accessibility_enabled;
      els.badgeAccessibility.className = `status-pill ${accEnabled ? "ok" : "warn"}`;
      els.badgeAccessibility.querySelector(".label").textContent = accEnabled ? "Accessibility" : "Accessibility Off";
      if (st.device?.device_name) {
        document.title = `${st.device.device_name} (Agentic Android)`;
      }
      if (st.device?.screen_width && st.device?.screen_height) {
        state.deviceWidth = st.device.screen_width;
        state.deviceHeight = st.device.screen_height;
        els.deviceDims.textContent = `${state.deviceWidth} × ${state.deviceHeight}`;
      }
    } catch {
      els.badgeAccessibility.className = "status-pill warn";
      els.badgeAccessibility.querySelector(".label").textContent = "Disconnected";
    }
  }

  function connectVideoWs() {
    if (state.videoWs) state.videoWs.close();
    const ws = new WebSocket(wsUrl("/ws/video"));
    ws.binaryType = "blob";
    state.videoWs = ws;

    ws.onopen = () => {
      els.badgeVideo.className = "status-pill ok";
      els.badgeVideo.querySelector(".label").textContent = "Screen";
    };

    ws.onmessage = (ev) => {
      if (typeof ev.data === "string") {
        try {
          const meta = JSON.parse(ev.data);
          if (meta.width && meta.height) {
            state.deviceWidth = meta.width;
            state.deviceHeight = meta.height;
            els.deviceDims.textContent = `${meta.width} × ${meta.height}`;
          }
        } catch {}
      } else {
        const url = URL.createObjectURL(ev.data);
        if (state.lastFrameUrl) URL.revokeObjectURL(state.lastFrameUrl);
        state.lastFrameUrl = url;
        els.screenImg.src = url;
        els.screenImg.style.display = "block";
        els.screenPlaceholder.classList.add("hidden");
      }
    };

    ws.onclose = () => {
      els.badgeVideo.className = "status-pill warn";
      els.badgeVideo.querySelector(".label").textContent = "Reconnecting";
      setTimeout(connectVideoWs, 2000);
    };
  }

  function toggleAudio() {
    if (state.audioEnabled) {
      state.audioEnabled = false;
      if (state.audioWs) {
        state.audioWs.close();
        state.audioWs = null;
      }
      els.btnAudioToggle.textContent = "Audio Off";
      return;
    }

    state.audioEnabled = true;
    els.btnAudioToggle.textContent = "Audio On";
    if (!state.audioCtx) {
      state.audioCtx = new (window.AudioContext || window.webkitAudioContext)();
    }
    if (state.audioCtx.state === "suspended") {
      state.audioCtx.resume();
    }
    state.audioNextTime = state.audioCtx.currentTime;

    const ws = new WebSocket(wsUrl("/ws/audio"));
    ws.binaryType = "arraybuffer";
    state.audioWs = ws;

    ws.onmessage = (ev) => {
      if (typeof ev.data === "string") {
        try {
          const meta = JSON.parse(ev.data);
          if (meta.sample_rate) state.audioSampleRate = meta.sample_rate;
          if (meta.channels) state.audioChannels = meta.channels;
        } catch {}
        return;
      }
      if (!state.audioCtx || !state.audioEnabled) return;
      const pcm16 = new Int16Array(ev.data);
      if (pcm16.length === 0) return;
      const channels = state.audioChannels || 1;
      const frameCount = Math.floor(pcm16.length / channels);
      if (frameCount <= 0) return;
      const buf = state.audioCtx.createBuffer(channels, frameCount, state.audioSampleRate || 16000);
      for (let ch = 0; ch < channels; ch++) {
        const out = buf.getChannelData(ch);
        for (let i = 0; i < frameCount; i++) {
          out[i] = pcm16[i * channels + ch] / 32768.0;
        }
      }
      const src = state.audioCtx.createBufferSource();
      src.buffer = buf;
      src.connect(state.audioCtx.destination);
      const now = state.audioCtx.currentTime;
      if (state.audioNextTime < now) state.audioNextTime = now + 0.02;
      src.start(state.audioNextTime);
      state.audioNextTime += buf.duration;
    };

    ws.onclose = () => {
      if (state.audioEnabled) {
        setTimeout(() => {
          if (state.audioEnabled) {
            state.audioEnabled = false;
            toggleAudio();
          }
        }, 2000);
      }
    };
  }

  function connectEventsWs() {
    if (state.eventsWs) state.eventsWs.close();
    const ws = new WebSocket(wsUrl("/ws/events"));
    state.eventsWs = ws;

    ws.onmessage = async (ev) => {
      try {
        const evt = JSON.parse(ev.data);
        if (evt.type === "agent_status" && evt.session_id === state.activeSessionId) {
          els.agentActivity.classList.toggle("hidden", !evt.running);
          els.agentActivityText.textContent = evt.status || "Working...";
        } else if (evt.type === "tool_call" && evt.session_id === state.activeSessionId) {
          els.agentActivityText.textContent = `Executed ${evt.tool_name}`;
        } else if (
          evt.type === "session_updated" ||
          evt.type === "assistant_message" ||
          evt.type === "error"
        ) {
          await loadSessions();
        }
      } catch {}
    };

    ws.onclose = () => setTimeout(connectEventsWs, 2000);
  }

  // Interactive pointer tap & swipe on live screen
  let pointerStart = null;

  function toDeviceCoords(clientX, clientY) {
    const rect = els.screenImg.getBoundingClientRect();
    const rx = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
    const ry = Math.max(0, Math.min(1, (clientY - rect.top) / rect.height));
    return {
      x: Math.round(rx * state.deviceWidth),
      y: Math.round(ry * state.deviceHeight),
      stageX: clientX - els.screenImg.parentElement.getBoundingClientRect().left,
      stageY: clientY - els.screenImg.parentElement.getBoundingClientRect().top,
    };
  }

  function showRipple(stageX, stageY) {
    els.touchRipple.style.left = `${stageX}px`;
    els.touchRipple.style.top = `${stageY}px`;
    els.touchRipple.classList.remove("hidden");
    setTimeout(() => els.touchRipple.classList.add("hidden"), 280);
  }

  els.screenImg.addEventListener("pointerdown", (e) => {
    e.preventDefault();
    pointerStart = toDeviceCoords(e.clientX, e.clientY);
  });

  els.screenImg.addEventListener("pointerup", async (e) => {
    if (!pointerStart) return;
    const end = toDeviceCoords(e.clientX, e.clientY);
    const dx = end.x - pointerStart.x;
    const dy = end.y - pointerStart.y;
    const dist = Math.hypot(dx, dy);
    showRipple(end.stageX, end.stageY);

    if (dist < 20) {
      await api("/api/device/action", {
        method: "POST",
        body: JSON.stringify({ action: "tap", params: { x: end.x, y: end.y } }),
      });
    } else {
      await api("/api/device/action", {
        method: "POST",
        body: JSON.stringify({
          action: "swipe",
          params: {
            start_x: pointerStart.x,
            start_y: pointerStart.y,
            end_x: end.x,
            end_y: end.y,
            duration_ms: 300,
          },
        }),
      });
    }
    pointerStart = null;
  });

  // Session controls
  els.btnNewSession.addEventListener("click", async () => {
    const s = await api("/api/sessions", {
      method: "POST",
      body: JSON.stringify({}),
    });
    state.activeSessionId = s.id;
    await loadSessions();
  });

  els.sessionProvider.addEventListener("change", async () => {
    const session = getActiveSession();
    if (!session) return;
    const provider = els.sessionProvider.value;
    const p = state.providers.find((x) => x.id === provider);
    const cachedModels = state.providerModelsCache[provider]?.models;
    const model = (cachedModels && cachedModels[0]) || p?.models?.[0] || "";
    populateModels(provider, model);
    await api(`/api/sessions/${session.id}`, {
      method: "PUT",
      body: JSON.stringify({ provider, model }),
    });
    await loadSessions();
    await fetchModelsForProvider(provider);
  });

  els.sessionModelSelect.addEventListener("change", async () => {
    const session = getActiveSession();
    if (!session) return;
    const model = els.sessionModelSelect.value;
    await api(`/api/sessions/${session.id}`, {
      method: "PUT",
      body: JSON.stringify({ model }),
    });
    await loadSessions();
  });

  els.btnRefreshModels.addEventListener("click", async () => {
    const provider = els.sessionProvider.value;
    await fetchModelsForProvider(provider, true);
  });

  // Chat submit
  els.chatForm.addEventListener("submit", async (e) => {
    e.preventDefault();
    const session = getActiveSession();
    const content = els.chatInput.value.trim();
    if (!session || !content) return;
    els.chatInput.value = "";
    els.agentActivity.classList.remove("hidden");
    els.agentActivityText.textContent = "Starting agent...";
    await api(`/api/sessions/${session.id}/message`, {
      method: "POST",
      body: JSON.stringify({
        content,
        provider: els.sessionProvider.value,
        model: els.sessionModelSelect.value,
      }),
    });
  });

  els.chatInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      els.chatForm.requestSubmit();
    }
  });

  els.btnStopAgent.addEventListener("click", async () => {
    const session = getActiveSession();
    if (!session) return;
    await api(`/api/sessions/${session.id}/stop`, { method: "POST" });
  });

  // Device quick controls
  document.querySelectorAll("[data-global]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const action = btn.getAttribute("data-global");
      await api("/api/device/action", {
        method: "POST",
        body: JSON.stringify({ action: "global_action", params: { action } }),
      });
    });
  });

  els.btnRefreshFrame.addEventListener("click", async () => {
    await api("/api/device/action", {
      method: "POST",
      body: JSON.stringify({ action: "request_screenshot", params: {} }),
    });
  });

  els.btnInspectUi.addEventListener("click", async () => {
    els.uitreeModal.classList.remove("hidden");
    els.uitreePre.textContent = "Fetching accessibility UI tree...";
    try {
      const tree = await api("/api/device/ui-tree");
      els.uitreePre.textContent = JSON.stringify(tree, null, 2);
    } catch (err) {
      els.uitreePre.textContent = `Error: ${err.message}`;
    }
  });

  els.btnCloseUitree.addEventListener("click", () => {
    els.uitreeModal.classList.add("hidden");
  });

  els.quickTypeForm.addEventListener("submit", async (e) => {
    e.preventDefault();
    const text = els.quickTypeInput.value;
    if (!text) return;
    els.quickTypeInput.value = "";
    await api("/api/device/action", {
      method: "POST",
      body: JSON.stringify({ action: "type_text", params: { text, clear_first: false } }),
    });
  });

  els.btnAudioToggle.addEventListener("click", toggleAudio);

  // Settings modal & tabs
  const setSettingsTab = (tabName) => {
    document.querySelectorAll(".tab-btn").forEach((btn) => {
      const active = btn.getAttribute("data-tab") === tabName;
      btn.classList.toggle("active", active);
      btn.setAttribute("aria-selected", active ? "true" : "false");
    });
    els.tabPanelKeys.classList.toggle("hidden", tabName !== "keys");
    els.tabPanelModels.classList.toggle("hidden", tabName !== "models");
    els.tabPanelPrompt.classList.toggle("hidden", tabName !== "prompt");
    els.tabPanelMemory.classList.toggle("hidden", tabName !== "memory");
    els.tabPanelTelegram.classList.toggle("hidden", tabName !== "telegram");
    els.tabPanelSecurity.classList.toggle("hidden", tabName !== "security");
    if (tabName === "models") {
      fetchDefaultModels(els.cfgDefaultProvider.value, state.config?.default_model);
    } else if (tabName === "memory") {
      loadAppMemory();
    } else if (tabName === "telegram") {
      loadConfig();
    } else if (tabName === "security") {
      refreshAuthStatus();
    }
  };

  document.querySelectorAll(".tab-btn").forEach((btn) => {
    btn.addEventListener("click", () => setSettingsTab(btn.getAttribute("data-tab")));
  });

  const openSettings = () => {
    els.keysSaveMsg.textContent = "";
    els.settingsSaveMsg.textContent = "";
    els.promptSaveMsg.textContent = "";
    els.memSaveMsg.textContent = "";
    els.tgSaveMsg.textContent = "";
    els.secMsg.textContent = "";
    els.secMfaSetupBox.classList.add("hidden");
    setSettingsTab("keys");
    els.settingsModal.classList.remove("hidden");
  };
  els.btnOpenSettings.addEventListener("click", openSettings);
  els.btnWarnSettings.addEventListener("click", openSettings);
  els.btnCloseSettings.addEventListener("click", () => els.settingsModal.classList.add("hidden"));

  els.addKeyForm.addEventListener("submit", async (e) => {
    e.preventDefault();
    const provider = els.addKeyProvider.value;
    const key = els.addKeyInput.value.trim();
    if (!key) return;
    els.addKeyInput.value = "";

    await api("/api/config", {
      method: "PUT",
      body: JSON.stringify({ api_keys: { [provider]: key } }),
    });
    state.providerModelsCache = {};
    await loadConfig();
    const active = getActiveSession();
    if (active) {
      await fetchModelsForProvider(active.provider, true);
    }
    els.keysSaveMsg.textContent = `${PROVIDER_LABELS[provider] || provider} key saved!`;
  });

  els.cfgDefaultProvider.addEventListener("change", () => {
    const provider = els.cfgDefaultProvider.value;
    const preferred =
      state.config?.default_provider === provider ? state.config.default_model : null;
    fetchDefaultModels(provider, preferred);
  });

  els.btnRefreshDefaultModels.addEventListener("click", () => {
    fetchDefaultModels(els.cfgDefaultProvider.value, els.cfgDefaultModel.value, true);
  });

  els.tabPanelModels.addEventListener("submit", async (e) => {
    e.preventDefault();
    const payload = {
      default_provider: els.cfgDefaultProvider.value,
      default_model: els.cfgDefaultModel.value,
      max_agent_steps: parseInt(document.getElementById("cfg-max-steps").value, 10) || 15,
      step_delay_ms: parseInt(document.getElementById("cfg-step-delay").value, 10) || 600,
      include_screenshot: document.getElementById("cfg-include-screenshot").checked,
      gemini_google_search: document.getElementById("cfg-gemini-search").checked,
      gemini_google_maps: document.getElementById("cfg-gemini-maps").checked,
    };
    await api("/api/config", {
      method: "PUT",
      body: JSON.stringify(payload),
    });
    await loadConfig();
    els.settingsSaveMsg.textContent = "Model settings saved!";
  });

  els.tabPanelPrompt.addEventListener("submit", async (e) => {
    e.preventDefault();
    const systemPrompt = document.getElementById("cfg-system-prompt").value;
    await api("/api/config", {
      method: "PUT",
      body: JSON.stringify({ system_prompt: systemPrompt }),
    });
    await loadConfig();
    els.promptSaveMsg.textContent = "System prompt saved!";
  });

  async function loadAppMemory() {
    const list = await api("/api/app-memory");
    els.appMemoryList.innerHTML = "";
    if (!list || list.length === 0) {
      const empty = document.createElement("div");
      empty.className = "keys-empty";
      empty.textContent = "No app playbooks saved yet. The agent will record UI quirks automatically, or you can add one below.";
      els.appMemoryList.appendChild(empty);
      return;
    }

    for (const pb of list) {
      const card = document.createElement("div");
      card.className = "memory-card";
      const title = pb.app_label ? `${pb.app_label}` : pb.package_name;
      const ver = pb.app_version ? ` · v${pb.app_version}` : "";
      const notesHtml = (pb.notes || [])
        .map((n) => `<li>${escapeHtml(n)}</li>`)
        .join("");
      card.innerHTML = `
        <div class="memory-card-head">
          <div>
            <div class="key-row-name">${escapeHtml(title)}</div>
            <div class="memory-pkg">${escapeHtml(pb.package_name)}${escapeHtml(ver)}</div>
          </div>
          <div class="security-actions">
            <button type="button" class="pill-btn btn-edit-mem">Edit</button>
            <button type="button" class="pill-btn pill-btn-danger btn-del-mem">Delete</button>
          </div>
        </div>
        <ul class="memory-notes">${notesHtml}</ul>
      `;
      card.querySelector(".btn-edit-mem").addEventListener("click", () => {
        els.memPackage.value = pb.package_name;
        els.memLabel.value = pb.app_label || "";
        els.memNotes.value = (pb.notes || []).join("\n");
        els.memNotes.focus();
      });
      card.querySelector(".btn-del-mem").addEventListener("click", async () => {
        await api(`/api/app-memory/${encodeURIComponent(pb.package_name)}`, {
          method: "DELETE",
        });
        await loadAppMemory();
      });
      els.appMemoryList.appendChild(card);
    }
  }

  els.appMemoryForm.addEventListener("submit", async (e) => {
    e.preventDefault();
    els.memSaveMsg.textContent = "";
    const pkg = els.memPackage.value.trim();
    if (!pkg) return;
    const notes = els.memNotes.value
      .split("\n")
      .map((l) => l.trim())
      .filter(Boolean);
    await api(`/api/app-memory/${encodeURIComponent(pkg)}`, {
      method: "PUT",
      body: JSON.stringify({
        app_label: els.memLabel.value.trim(),
        notes,
      }),
    });
    els.memPackage.value = "";
    els.memLabel.value = "";
    els.memNotes.value = "";
    els.memSaveMsg.textContent = "Playbook saved!";
    await loadAppMemory();
  });

  els.tabPanelTelegram.addEventListener("submit", async (e) => {
    e.preventDefault();
    els.tgSaveMsg.textContent = "";
    const ids = els.tgAllowedIds.value
      .split(/[,\s]+/)
      .map((s) => s.trim())
      .filter((s) => /^-?\d+$/.test(s))
      .map((s) => Number(s));

    const tgPatch = {
      enabled: els.tgEnabled.checked,
      allowed_chat_ids: ids,
    };
    const tok = els.tgBotToken.value.trim();
    if (tok !== "") {
      tgPatch.bot_token = tok;
      els.tgBotToken.value = "";
    }

    await api("/api/config", {
      method: "PUT",
      body: JSON.stringify({ telegram: tgPatch }),
    });
    els.tgSaveMsg.textContent = "Telegram settings saved!";
    await loadConfig();
    setTimeout(() => loadConfig(), 2500);
  });

  // Security & Auth flows
  function showLoginScreen(mfaEnabled = false) {
    if (state.videoWs) { state.videoWs.close(); state.videoWs = null; }
    if (state.audioWs) { state.audioWs.close(); state.audioWs = null; }
    if (state.eventsWs) { state.eventsWs.close(); state.eventsWs = null; }
    if (statusInterval) { clearInterval(statusInterval); statusInterval = null; }
    appBooted = false;
    els.loginError.classList.add("hidden");
    els.loginMfaField.classList.toggle("hidden", !mfaEnabled);
    els.loginTotp.value = "";
    els.loginOverlay.classList.remove("hidden");
    els.loginUsername.focus();
  }

  async function refreshAuthStatus() {
    const st = await api("/api/auth/status");
    els.btnLogout.classList.toggle("hidden", !st.auth_enabled);
    els.secAuthBadge.className = `status-pill ${st.auth_enabled ? "ok" : ""}`;
    els.secAuthBadge.querySelector(".label").textContent = st.auth_enabled
      ? `Enabled (${st.username || "active"})`
      : "Disabled";
    if (st.username) els.secUsername.value = st.username;
    els.btnSaveCredentials.textContent = st.auth_enabled ? "Update Credentials" : "Enable Login";
    els.btnDisableAuth.classList.toggle("hidden", !st.auth_enabled);

    els.secMfaBadge.className = `status-pill ${st.mfa_enabled ? "ok" : ""}`;
    els.secMfaBadge.querySelector(".label").textContent = st.mfa_enabled ? "Active" : "Off";
    els.btnSetupMfa.disabled = !st.auth_enabled;
    els.btnSetupMfa.style.opacity = st.auth_enabled ? "1" : "0.45";
    els.btnSetupMfa.textContent = st.mfa_enabled ? "Reconfigure 2FA" : "Set Up 2FA";
    els.btnDisableMfa.classList.toggle("hidden", !st.mfa_enabled);
    return st;
  }

  els.secCredentialsForm.addEventListener("submit", async (e) => {
    e.preventDefault();
    els.secMsg.textContent = "";
    try {
      await api("/api/auth/credentials", {
        method: "POST",
        body: JSON.stringify({
          username: els.secUsername.value.trim(),
          password: els.secPassword.value,
        }),
      });
      els.secPassword.value = "";
      await refreshAuthStatus();
      els.secMsg.textContent = "Login credentials saved!";
    } catch (err) {
      els.secMsg.textContent = err.message;
    }
  });

  els.btnDisableAuth.addEventListener("click", async () => {
    await api("/api/auth/disable", { method: "POST" });
    els.secMfaSetupBox.classList.add("hidden");
    await refreshAuthStatus();
    els.secMsg.textContent = "Login protection disabled.";
  });

  els.btnSetupMfa.addEventListener("click", async () => {
    els.secMsg.textContent = "";
    try {
      const res = await api("/api/auth/mfa/setup", { method: "POST" });
      els.secMfaQr.innerHTML = res.qr_svg;
      els.secMfaSecret.textContent = res.secret;
      els.secMfaCode.value = "";
      els.secMfaSetupBox.classList.remove("hidden");
      els.secMfaCode.focus();
    } catch (err) {
      els.secMsg.textContent = err.message;
    }
  });

  els.secMfaVerifyForm.addEventListener("submit", async (e) => {
    e.preventDefault();
    els.secMsg.textContent = "";
    try {
      await api("/api/auth/mfa/enable", {
        method: "POST",
        body: JSON.stringify({ code: els.secMfaCode.value.trim() }),
      });
      els.secMfaSetupBox.classList.add("hidden");
      await refreshAuthStatus();
      els.secMsg.textContent = "2FA enabled!";
    } catch (err) {
      els.secMsg.textContent = err.message;
    }
  });

  els.btnDisableMfa.addEventListener("click", async () => {
    await api("/api/auth/mfa/disable", { method: "POST" });
    els.secMfaSetupBox.classList.add("hidden");
    await refreshAuthStatus();
    els.secMsg.textContent = "2FA disabled.";
  });

  els.btnLogout.addEventListener("click", async () => {
    await api("/api/auth/logout", { method: "POST" });
    const st = await api("/api/auth/status");
    showLoginScreen(st.mfa_enabled);
  });

  els.loginForm.addEventListener("submit", async (e) => {
    e.preventDefault();
    els.loginError.classList.add("hidden");
    try {
      await api("/api/auth/login", {
        method: "POST",
        body: JSON.stringify({
          username: els.loginUsername.value.trim(),
          password: els.loginPassword.value,
          totp_code: els.loginTotp.value.trim(),
        }),
      });
      els.loginPassword.value = "";
      els.loginTotp.value = "";
      els.loginOverlay.classList.add("hidden");
      await bootApp();
    } catch (err) {
      if (err.data?.mfa_required) {
        els.loginMfaField.classList.remove("hidden");
        els.loginTotp.focus();
      }
      els.loginError.textContent = err.message || "Sign in failed";
      els.loginError.classList.remove("hidden");
    }
  });

  function escapeHtml(str) {
    return String(str ?? "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  async function bootApp() {
    if (appBooted) return;
    appBooted = true;
    await refreshAuthStatus();
    const pData = await api("/api/providers");
    state.providers = pData.providers || [];
    await loadConfig();
    await loadSessions();
    await refreshStatus();
    statusInterval = setInterval(refreshStatus, 5000);
    connectVideoWs();
    connectEventsWs();
  }

  async function init() {
    const authSt = await api("/api/auth/status");
    if (authSt.auth_enabled && !authSt.authenticated) {
      showLoginScreen(authSt.mfa_enabled);
      return;
    }
    await bootApp();
  }

  init().catch((err) => console.error("Init error:", err));
})();
