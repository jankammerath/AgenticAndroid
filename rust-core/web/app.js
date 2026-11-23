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
    settingsForm: document.getElementById("settings-form"),
    settingsSaveMsg: document.getElementById("settings-save-msg"),
    uitreeModal: document.getElementById("uitree-modal"),
    btnCloseUitree: document.getElementById("btn-close-uitree"),
    uitreePre: document.getElementById("uitree-pre"),
  };

  async function api(path, opts = {}) {
    const res = await fetch(path, {
      headers: { "Content-Type": "application/json", ...(opts.headers || {}) },
      ...opts,
    });
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}`);
    }
    return res.json();
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
      const item = document.createElement("div");
      item.className = `session-item ${s.id === state.activeSessionId ? "active" : ""}`;
      item.innerHTML = `
        <div class="session-meta">
          <div class="session-title">${escapeHtml(s.title)}</div>
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

  function formatContent(text) {
    const escaped = escapeHtml(text || "");
    return escaped.replace(/`([^`]+)`/g, "<code>$1</code>");
  }

  function renderMessages() {
    const session = getActiveSession();
    els.chatMessages.innerHTML = "";
    if (!session) return;

    if (session.messages.length === 0) {
      const empty = document.createElement("div");
      empty.className = "msg welcome";
      empty.innerHTML = `<h3>Agentic Android</h3><p>Describe what you want done on your device, or interact with the live screen on the right.</p>`;
      els.chatMessages.appendChild(empty);
      return;
    }

    for (const m of session.messages) {
      const div = document.createElement("div");
      div.className = `msg ${m.role}`;
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
      div.innerHTML = `<div class="msg-body">${formatContent(m.content)}</div>${tracesHtml}`;
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

  async function loadConfig() {
    state.config = await api("/api/config");
    const masked = state.config.api_keys_masked || {};
    for (const id of ["claude", "gemini", "grok", "openai"]) {
      const el = document.getElementById(`status-${id}`);
      if (el) el.textContent = masked[id] ? `Saved (${masked[id]})` : "Not set";
    }
    document.getElementById("cfg-default-provider").value = state.config.default_provider;
    document.getElementById("cfg-max-steps").value = state.config.max_agent_steps;
    document.getElementById("cfg-step-delay").value = state.config.step_delay_ms;
    document.getElementById("cfg-include-screenshot").checked = !!state.config.include_screenshot;
    document.getElementById("cfg-system-prompt").value = state.config.system_prompt || "";
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

  // Settings modal
  const openSettings = () => {
    els.settingsSaveMsg.textContent = "";
    els.settingsModal.classList.remove("hidden");
  };
  els.btnOpenSettings.addEventListener("click", openSettings);
  els.btnWarnSettings.addEventListener("click", openSettings);
  els.btnCloseSettings.addEventListener("click", () => els.settingsModal.classList.add("hidden"));

  els.settingsForm.addEventListener("submit", async (e) => {
    e.preventDefault();
    const keyPatch = {};
    for (const id of ["claude", "gemini", "grok", "openai"]) {
      const input = document.getElementById(`key-${id}`);
      if (input.value.trim() !== "") {
        keyPatch[id] = input.value.trim();
        input.value = "";
      }
    }

    const payload = {
      default_provider: document.getElementById("cfg-default-provider").value,
      max_agent_steps: parseInt(document.getElementById("cfg-max-steps").value, 10) || 15,
      step_delay_ms: parseInt(document.getElementById("cfg-step-delay").value, 10) || 600,
      include_screenshot: document.getElementById("cfg-include-screenshot").checked,
      system_prompt: document.getElementById("cfg-system-prompt").value,
    };
    if (Object.keys(keyPatch).length > 0) {
      payload.api_keys = keyPatch;
    }

    await api("/api/config", {
      method: "PUT",
      body: JSON.stringify(payload),
    });
    state.providerModelsCache = {};
    await loadConfig();
    const active = getActiveSession();
    if (active) {
      await fetchModelsForProvider(active.provider, true);
    }
    els.settingsSaveMsg.textContent = "Saved!";
  });

  function escapeHtml(str) {
    return String(str ?? "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  async function init() {
    const pData = await api("/api/providers");
    state.providers = pData.providers || [];
    await loadConfig();
    await loadSessions();
    await refreshStatus();
    setInterval(refreshStatus, 5000);
    connectVideoWs();
    connectEventsWs();
  }

  init().catch((err) => console.error("Init error:", err));
})();
