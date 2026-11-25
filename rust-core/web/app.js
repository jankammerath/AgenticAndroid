(() => {
  const t = (key, vars) => (window.I18n ? window.I18n.t(key, vars) : key);

  const state = {
    providers: [],
    providerModelsCache: {},
    config: null,
    sessions: [],
    appMemory: [],
    credentials: [],
    sidebarTab: "sessions",
    openTabs: [],
    activeTabKey: null,
    activeSessionId: null,
    deviceWidth: 1080,
    deviceHeight: 2400,
    videoWs: null,
    audioWs: null,
    eventsWs: null,
    audioEnabled: false,
    highFpsActive: false,
    highFpsPending: false,
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
    btnHighFpsToggle: document.getElementById("btn-high-fps-toggle"),
    btnAudioToggle: document.getElementById("btn-audio-toggle"),
    btnOpenSettings: document.getElementById("btn-open-settings"),
    btnLogout: document.getElementById("btn-logout"),
    btnMobileMenu: document.getElementById("btn-mobile-menu"),
    btnCloseMobileMenu: document.getElementById("btn-close-mobile-menu"),
    btnMobileActions: document.getElementById("btn-mobile-actions"),
    btnCloseMobileActions: document.getElementById("btn-close-mobile-actions"),
    mobileBackdrop: document.getElementById("mobile-backdrop"),
    sidebar: document.getElementById("sidebar"),
    chatPane: document.getElementById("chat-pane"),
    deviceControls: document.getElementById("device-controls"),
    mobileChatHandle: document.getElementById("mobile-chat-handle"),
    mobileChatLabel: document.getElementById("mobile-chat-label"),
    mobileChatDot: document.getElementById("mobile-chat-dot"),
    mobileBadgeAccessibility: document.getElementById("mobile-badge-accessibility"),
    mobileBadgeVideo: document.getElementById("mobile-badge-video"),
    mobileBtnHighFps: document.getElementById("mobile-btn-high-fps"),
    mobileBtnAudio: document.getElementById("mobile-btn-audio"),
    mobileBtnSettings: document.getElementById("mobile-btn-settings"),
    mobileBtnLogout: document.getElementById("mobile-btn-logout"),
    mobileBtnUploadApk: document.getElementById("mobile-btn-upload-apk"),
    mobileBtnRefreshFrame: document.getElementById("mobile-btn-refresh-frame"),
    mobileBtnInspectUi: document.getElementById("mobile-btn-inspect-ui"),
    btnWarnSettings: document.getElementById("btn-warn-settings"),
    btnNewSession: document.getElementById("btn-new-session"),
    btnNewMemory: document.getElementById("btn-new-memory"),
    btnNewCredential: document.getElementById("btn-new-credential"),
    sidebarPanelSessions: document.getElementById("sidebar-panel-sessions"),
    sidebarPanelMemory: document.getElementById("sidebar-panel-memory"),
    sidebarPanelCredentials: document.getElementById("sidebar-panel-credentials"),
    sessionList: document.getElementById("session-list"),
    memorySidebarList: document.getElementById("memory-sidebar-list"),
    credentialsSidebarList: document.getElementById("credentials-sidebar-list"),
    centerTabsBar: document.getElementById("center-tabs-bar"),
    centerViewSession: document.getElementById("center-view-session"),
    centerViewMemory: document.getElementById("center-view-memory"),
    centerViewCredential: document.getElementById("center-view-credential"),
    centerViewSettings: document.getElementById("center-view-settings"),
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
    btnUploadApk: document.getElementById("btn-upload-apk"),
    btnRefreshFrame: document.getElementById("btn-refresh-frame"),
    btnInspectUi: document.getElementById("btn-inspect-ui"),
    quickTypeForm: document.getElementById("quick-type-form"),
    quickTypeInput: document.getElementById("quick-type-input"),
    addKeyForm: document.getElementById("add-key-form"),
    addKeyProvider: document.getElementById("add-key-provider"),
    addKeyInput: document.getElementById("add-key-input"),
    configuredKeysList: document.getElementById("configured-keys-list"),
    keysSaveMsg: document.getElementById("keys-save-msg"),
    tabPanelKeys: document.getElementById("tab-panel-keys"),
    tabPanelModels: document.getElementById("tab-panel-models"),
    tabPanelPrompt: document.getElementById("tab-panel-prompt"),
    appMemoryForm: document.getElementById("app-memory-form"),
    memEditorTitle: document.getElementById("mem-editor-title"),
    memPackage: document.getElementById("mem-package"),
    memLabel: document.getElementById("mem-label"),
    memPurpose: document.getElementById("mem-purpose"),
    memNotes: document.getElementById("mem-notes"),
    memSaveMsg: document.getElementById("mem-save-msg"),
    btnDeleteMemory: document.getElementById("btn-delete-memory"),
    credentialForm: document.getElementById("credential-form"),
    credEditorTitle: document.getElementById("cred-editor-title"),
    credLabel: document.getElementById("cred-label"),
    credTarget: document.getElementById("cred-target"),
    credUsername: document.getElementById("cred-username"),
    credPassword: document.getElementById("cred-password"),
    btnToggleCredPw: document.getElementById("btn-toggle-cred-pw"),
    credNotes: document.getElementById("cred-notes"),
    credSaveMsg: document.getElementById("cred-save-msg"),
    btnDeleteCredential: document.getElementById("btn-delete-credential"),
    tabPanelTelegram: document.getElementById("tab-panel-telegram"),
    tabPanelSecurity: document.getElementById("tab-panel-security"),
    tabBtnEmulator: document.getElementById("tab-btn-emulator"),
    tabPanelEmulator: document.getElementById("tab-panel-emulator"),
    emuCurrentAvd: document.getElementById("emu-current-avd"),
    emuMetaDesc: document.getElementById("emu-meta-desc"),
    btnEmuRestart: document.getElementById("btn-emu-restart"),
    emuAvdSelect: document.getElementById("emu-avd-select"),
    btnEmuSwitch: document.getElementById("btn-emu-switch"),
    emuCreateForm: document.getElementById("emu-create-form"),
    emuNewName: document.getElementById("emu-new-name"),
    emuNewDevice: document.getElementById("emu-new-device"),
    emuNewImage: document.getElementById("emu-new-image"),
    emuNewSwitch: document.getElementById("emu-new-switch"),
    btnEmuCreate: document.getElementById("btn-emu-create"),
    emuMsg: document.getElementById("emu-msg"),
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
    apkModal: document.getElementById("apk-modal"),
    btnCloseApk: document.getElementById("btn-close-apk"),
    apkDropzone: document.getElementById("apk-dropzone"),
    apkFileInput: document.getElementById("apk-file-input"),
    apkDropzoneLabel: document.getElementById("apk-dropzone-label"),
    apkFileMeta: document.getElementById("apk-file-meta"),
    apkProgressWrap: document.getElementById("apk-progress-wrap"),
    apkProgressBar: document.getElementById("apk-progress-bar"),
    apkStatus: document.getElementById("apk-status"),
    btnStartApkUpload: document.getElementById("btn-start-apk-upload"),
  };

  let appBooted = false;
  let statusInterval = null;
  let swRegistration = null;

  if ("serviceWorker" in navigator) {
    navigator.serviceWorker
      .register("/sw.js")
      .then((reg) => { swRegistration = reg; })
      .catch(() => {});
  }

  function closeMobileOverlays() {
    els.sidebar.classList.remove("mobile-open");
    els.deviceControls.classList.remove("mobile-open");
    els.chatPane.classList.remove("mobile-expanded");
    els.mobileChatHandle.setAttribute("aria-expanded", "false");
    els.mobileBackdrop.classList.add("hidden");
  }

  function syncMobileBackdrop() {
    const anyOpen =
      els.sidebar.classList.contains("mobile-open") ||
      els.deviceControls.classList.contains("mobile-open") ||
      els.chatPane.classList.contains("mobile-expanded");
    els.mobileBackdrop.classList.toggle("hidden", !anyOpen);
  }

  async function ensureNotificationPermission() {
    if (!("Notification" in window)) return;
    if (state.config?.notify_on_complete === false) return;
    if (Notification.permission === "default") {
      try {
        await Notification.requestPermission();
      } catch {}
    }
  }

  async function notifyAgentCompleted(message) {
    if (!("Notification" in window)) return;
    if (state.config?.notify_on_complete === false) return;
    if (Notification.permission !== "granted") return;

    const raw = String(message?.content || t("chat.task_finished"))
      .replace(/!\[[^\]]*\]\([^)]*\)/g, "[Image]")
      .replace(/[*_`#>-]/g, "")
      .trim();
    const body = raw.length > 160 ? `${raw.slice(0, 157)}...` : raw;
    const opts = {
      body,
      icon: "/icon-192.png",
      badge: "/favicon.png",
      tag: "agentic-android-complete",
      renotify: true,
    };

    try {
      if (swRegistration && typeof swRegistration.showNotification === "function") {
        await swRegistration.showNotification("Agentic Android", opts);
      } else {
        new Notification("Agentic Android", opts);
      }
    } catch {}
  }

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
      els.modelSourceHint.textContent = t("chat.models_count", { count: models.length });
      els.modelSourceHint.className = "model-source-hint live";
      els.modelSourceHint.title = t("chat.models_live_title");
    } else if (effSource === "loading") {
      els.modelSourceHint.textContent = t("chat.loading");
      els.modelSourceHint.className = "model-source-hint";
      els.modelSourceHint.title = t("chat.loading_models_title");
    } else {
      els.modelSourceHint.textContent = t("chat.add_api_key");
      els.modelSourceHint.className = "model-source-hint";
      els.modelSourceHint.title = cached?.error || t("chat.add_api_key_title");
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

  function setSidebarTab(tabName) {
    state.sidebarTab = tabName;
    document.querySelectorAll(".sidebar-tab-btn").forEach((btn) => {
      const active = btn.getAttribute("data-sidebar-tab") === tabName;
      btn.classList.toggle("active", active);
      btn.setAttribute("aria-selected", active ? "true" : "false");
    });
    els.sidebarPanelSessions.classList.toggle("hidden", tabName !== "sessions");
    els.sidebarPanelMemory.classList.toggle("hidden", tabName !== "memory");
    els.sidebarPanelCredentials.classList.toggle("hidden", tabName !== "credentials");
  }

  document.querySelectorAll(".sidebar-tab-btn").forEach((btn) => {
    btn.addEventListener("click", () => setSidebarTab(btn.getAttribute("data-sidebar-tab")));
  });

  // Desktop resizable left sidebar
  (() => {
    const resizer = document.getElementById("sidebar-resizer");
    if (!resizer) return;
    const STORAGE_KEY = "agentic_sidebar_width";
    const MIN_W = 190;
    const MAX_W = 520;
    const applyWidth = (w) => {
      const clamped = Math.max(MIN_W, Math.min(MAX_W, Math.round(w)));
      document.documentElement.style.setProperty("--sidebar-width", `${clamped}px`);
      return clamped;
    };
    try {
      const saved = parseInt(localStorage.getItem(STORAGE_KEY) || "", 10);
      if (saved >= MIN_W && saved <= MAX_W) applyWidth(saved);
    } catch {}

    let startX = 0;
    let startW = 280;
    resizer.addEventListener("pointerdown", (e) => {
      if (window.innerWidth <= 900) return;
      e.preventDefault();
      resizer.setPointerCapture(e.pointerId);
      resizer.classList.add("dragging");
      startX = e.clientX;
      startW = els.sidebar.getBoundingClientRect().width;
    });
    resizer.addEventListener("pointermove", (e) => {
      if (!resizer.classList.contains("dragging")) return;
      applyWidth(startW + (e.clientX - startX));
    });
    const finish = (e) => {
      if (!resizer.classList.contains("dragging")) return;
      resizer.classList.remove("dragging");
      try {
        resizer.releasePointerCapture(e.pointerId);
      } catch {}
      const finalW = applyWidth(startW + (e.clientX - startX));
      try {
        localStorage.setItem(STORAGE_KEY, String(finalW));
      } catch {}
    };
    resizer.addEventListener("pointerup", finish);
    resizer.addEventListener("pointercancel", finish);
  })();

  function tabIconSvg(kind) {
    if (kind === "memory") {
      return `<svg class="center-tab-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/></svg>`;
    }
    if (kind === "credential") {
      return `<svg class="center-tab-icon" viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>`;
    }
    if (kind === "settings") {
      return `<svg class="center-tab-icon" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9c.26.6.85 1 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>`;
    }
    return `<svg class="center-tab-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg>`;
  }

  function getActiveTab() {
    return state.openTabs.find((tb) => tb.key === state.activeTabKey) || state.openTabs[0] || null;
  }

  function ensureAtLeastOneTab() {
    if (state.openTabs.length === 0 && state.sessions.length > 0) {
      const first = state.sessions.find((s) => s.id === state.activeSessionId) || state.sessions[0];
      state.activeSessionId = first.id;
      const key = `session:${first.id}`;
      state.openTabs.push({ key, kind: "session", id: first.id, title: first.title });
      state.activeTabKey = key;
    }
  }

  function renderCenterTabs() {
    ensureAtLeastOneTab();
    els.centerTabsBar.innerHTML = "";
    for (const tb of state.openTabs) {
      const active = tb.key === state.activeTabKey;
      const btn = document.createElement("div");
      btn.className = `center-tab ${active ? "active" : ""}`;
      btn.setAttribute("role", "tab");
      btn.setAttribute("aria-selected", active ? "true" : "false");
      btn.innerHTML = `
        ${tabIconSvg(tb.kind)}
        <span class="center-tab-title">${escapeHtml(tb.title)}</span>
        <button type="button" class="center-tab-close" title="Close">&times;</button>
      `;
      btn.addEventListener("click", () => {
        activateCenterTab(tb.key);
      });
      btn.querySelector(".center-tab-close").addEventListener("click", (e) => {
        e.stopPropagation();
        closeCenterTab(tb.key);
      });
      els.centerTabsBar.appendChild(btn);
    }
  }

  async function populateMemoryEditorForTab(tab) {
    els.memSaveMsg.textContent = "";
    const existing = tab.id ? state.appMemory.find((p) => p.package_name === tab.id) : null;
    if (existing) {
      els.memEditorTitle.textContent = existing.app_label || existing.package_name;
      els.memPackage.value = existing.package_name;
      els.memLabel.value = existing.app_label || "";
      els.memPurpose.value = existing.purpose || "";
      els.memNotes.value = (existing.notes || []).join("\n");
      els.btnDeleteMemory.classList.remove("hidden");
    } else {
      els.memEditorTitle.textContent = t("memory.new_title");
      els.memPackage.value = "";
      els.memLabel.value = "";
      els.memPurpose.value = "";
      els.memNotes.value = "";
      els.btnDeleteMemory.classList.add("hidden");
    }
  }

  async function populateCredentialEditorForTab(tab) {
    els.credSaveMsg.textContent = "";
    els.credPassword.type = "password";
    els.btnToggleCredPw.textContent = t("credentials.show");
    if (tab.id) {
      els.btnDeleteCredential.classList.remove("hidden");
      try {
        const full = await api(`/api/credentials/${encodeURIComponent(tab.id)}`);
        els.credEditorTitle.textContent = full.label || full.target || t("credentials.form_title");
        els.credLabel.value = full.label || "";
        els.credTarget.value = full.target || "";
        els.credUsername.value = full.username || "";
        els.credPassword.value = full.password || "";
        els.credNotes.value = full.notes || "";
      } catch {
        els.credEditorTitle.textContent = t("credentials.form_title");
      }
    } else {
      els.credEditorTitle.textContent = t("credentials.new_title");
      els.credLabel.value = "";
      els.credTarget.value = "";
      els.credUsername.value = "";
      els.credPassword.value = "";
      els.credNotes.value = "";
      els.btnDeleteCredential.classList.add("hidden");
    }
  }

  function renderCenterWorkspace() {
    ensureAtLeastOneTab();
    renderCenterTabs();
    const tab = getActiveTab();
    if (!tab) return;

    els.centerViewSession.classList.toggle("hidden", tab.kind !== "session");
    els.centerViewMemory.classList.toggle("hidden", tab.kind !== "memory");
    els.centerViewCredential.classList.toggle("hidden", tab.kind !== "credential");
    els.centerViewSettings.classList.toggle("hidden", tab.kind !== "settings");

    if (tab.kind === "session") {
      state.activeSessionId = tab.id;
      const session = getActiveSession();
      if (session) {
        els.sessionProvider.value = session.provider;
        populateModels(session.provider, session.model);
        if (!state.providerModelsCache[session.provider]) {
          fetchModelsForProvider(session.provider);
        }
      }
      renderMessages();
      updateKeyWarning();
    } else if (tab.kind === "memory") {
      populateMemoryEditorForTab(tab);
    } else if (tab.kind === "credential") {
      populateCredentialEditorForTab(tab);
    }

    renderSessions();
    renderMemorySidebar();
    renderCredentialsSidebar();
  }

  function activateCenterTab(key, expandMobile = false) {
    const tab = state.openTabs.find((tb) => tb.key === key);
    if (!tab) return;
    state.activeTabKey = key;
    if (tab.kind === "session") {
      state.activeSessionId = tab.id;
    }
    renderCenterWorkspace();
    if (expandMobile && window.innerWidth <= 900) {
      closeMobileOverlays();
      els.chatPane.classList.add("mobile-expanded");
      els.mobileChatHandle.setAttribute("aria-expanded", "true");
      syncMobileBackdrop();
    }
  }

  function openCenterTab(tabSpec, expandMobile = false) {
    const existing = state.openTabs.find((tb) => tb.key === tabSpec.key);
    if (existing) {
      existing.title = tabSpec.title;
      activateCenterTab(existing.key, expandMobile);
      return;
    }
    state.openTabs.push({ ...tabSpec });
    activateCenterTab(tabSpec.key, expandMobile);
  }

  function closeCenterTab(key) {
    const idx = state.openTabs.findIndex((tb) => tb.key === key);
    if (idx === -1) return;
    state.openTabs.splice(idx, 1);
    if (state.activeTabKey === key) {
      const next = state.openTabs[Math.min(idx, state.openTabs.length - 1)] || null;
      state.activeTabKey = next ? next.key : null;
      if (next && next.kind === "session") {
        state.activeSessionId = next.id;
      }
    }
    renderCenterWorkspace();
  }

  function renderSessions() {
    els.sessionList.innerHTML = "";
    const activeTab = getActiveTab();
    for (const s of state.sessions) {
      const isTelegram = s.source === "telegram" || !!s.telegram_chat_id;
      const tgBadge = isTelegram
        ? `<span class="session-source-badge" title="${escapeHtml(t("sessions.telegram_badge_title"))}">TG</span>`
        : "";
      const isSelected = activeTab?.kind === "session" && activeTab.id === s.id;
      const item = document.createElement("div");
      item.className = `session-item ${isSelected ? "active" : ""}`;
      item.innerHTML = `
        <div class="session-meta">
          <div class="session-title-row">${tgBadge}<span class="session-title">${escapeHtml(s.title)}</span></div>
          <div class="session-sub">${escapeHtml(s.provider)} · ${escapeHtml(s.model)}</div>
        </div>
        <button class="session-del" title="${escapeHtml(t("sessions.delete_title"))}">&times;</button>
      `;
      item.querySelector(".session-meta").addEventListener("click", () => {
        openCenterTab({ key: `session:${s.id}`, kind: "session", id: s.id, title: s.title }, true);
      });
      item.querySelector(".session-del").addEventListener("click", async (e) => {
        e.stopPropagation();
        await api(`/api/sessions/${s.id}`, { method: "DELETE" });
        closeCenterTab(`session:${s.id}`);
        await loadSessions();
      });
      els.sessionList.appendChild(item);
    }
  }

  function renderMemorySidebar() {
    els.memorySidebarList.innerHTML = "";
    if (!state.appMemory || state.appMemory.length === 0) {
      const empty = document.createElement("div");
      empty.className = "sidebar-empty";
      empty.textContent = t("memory.empty");
      els.memorySidebarList.appendChild(empty);
      return;
    }
    const activeTab = getActiveTab();
    for (const pb of state.appMemory) {
      const title = pb.app_label || pb.package_name;
      const sub = pb.purpose || pb.package_name;
      const isSelected = activeTab?.kind === "memory" && activeTab.id === pb.package_name;
      const item = document.createElement("div");
      item.className = `session-item ${isSelected ? "active" : ""}`;
      item.innerHTML = `
        <div class="session-meta">
          <div class="session-title-row"><span class="session-title">${escapeHtml(title)}</span></div>
          <div class="session-sub">${escapeHtml(sub)}</div>
        </div>
        <button class="session-del" title="${escapeHtml(t("memory.delete"))}">&times;</button>
      `;
      item.querySelector(".session-meta").addEventListener("click", () => {
        openCenterTab(
          { key: `memory:${pb.package_name}`, kind: "memory", id: pb.package_name, title },
          true
        );
      });
      item.querySelector(".session-del").addEventListener("click", async (e) => {
        e.stopPropagation();
        await api(`/api/app-memory/${encodeURIComponent(pb.package_name)}`, { method: "DELETE" });
        closeCenterTab(`memory:${pb.package_name}`);
        await loadAppMemory();
      });
      els.memorySidebarList.appendChild(item);
    }
  }

  function renderCredentialsSidebar() {
    els.credentialsSidebarList.innerHTML = "";
    if (!state.credentials || state.credentials.length === 0) {
      const empty = document.createElement("div");
      empty.className = "sidebar-empty";
      empty.textContent = t("credentials.empty");
      els.credentialsSidebarList.appendChild(empty);
      return;
    }
    const activeTab = getActiveTab();
    for (const c of state.credentials) {
      const title = c.label || c.target || c.username;
      const sub = [c.target, c.username].filter(Boolean).join(" · ") || "••••••••";
      const isSelected = activeTab?.kind === "credential" && activeTab.id === c.id;
      const item = document.createElement("div");
      item.className = `session-item ${isSelected ? "active" : ""}`;
      item.innerHTML = `
        <div class="session-meta">
          <div class="session-title-row"><span class="session-title">${escapeHtml(title)}</span></div>
          <div class="session-sub">${escapeHtml(sub)}</div>
        </div>
        <button class="session-del" title="${escapeHtml(t("memory.delete"))}">&times;</button>
      `;
      item.querySelector(".session-meta").addEventListener("click", () => {
        openCenterTab(
          { key: `credential:${c.id}`, kind: "credential", id: c.id, title },
          true
        );
      });
      item.querySelector(".session-del").addEventListener("click", async (e) => {
        e.stopPropagation();
        await api(`/api/credentials/${encodeURIComponent(c.id)}`, { method: "DELETE" });
        closeCenterTab(`credential:${c.id}`);
        await loadCredentials();
      });
      els.credentialsSidebarList.appendChild(item);
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
    let title = t("chat.request_failed");
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
      empty.innerHTML = `<img src="/logo.png" alt="Agentic Android" class="welcome-logo" /><h3>Agentic Android</h3><p>${escapeHtml(t("chat.welcome_desc"))}</p>`;
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
          ? `<details class="error-details"><summary>${escapeHtml(t("chat.raw_details"))}</summary><pre>${escapeHtml(err.details)}</pre></details>`
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
            (tr) =>
              `<div class="trace-item"><strong>${escapeHtml(tr.tool_name)}</strong>(${escapeHtml(
                JSON.stringify(tr.arguments)
              )})<br/>→ ${escapeHtml(tr.result)}</div>`
          )
          .join("");
        const actionSummary =
          m.tool_traces.length === 1
            ? t("chat.device_action_one")
            : t("chat.device_action_many", { count: m.tool_traces.length });
        tracesHtml = `<details class="traces"><summary>${escapeHtml(actionSummary)}</summary><div class="trace-list">${items}</div></details>`;
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
    // Sync open session tab titles & prune deleted sessions
    const sessionById = new Map(state.sessions.map((s) => [s.id, s]));
    state.openTabs = state.openTabs.filter((tb) => {
      if (tb.kind !== "session") return true;
      const s = sessionById.get(tb.id);
      if (!s) return false;
      tb.title = s.title;
      return true;
    });
    if (state.activeTabKey && !state.openTabs.some((tb) => tb.key === state.activeTabKey)) {
      state.activeTabKey = state.openTabs[0]?.key || null;
    }
    ensureAtLeastOneTab();
    renderCenterWorkspace();
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
      empty.textContent = t("keys.empty");
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
        <button type="button" class="pill-btn pill-btn-danger">${escapeHtml(t("keys.remove"))}</button>
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
      els.cfgModelSourceHint.textContent = t("chat.models_count", { count: models.length });
      els.cfgModelSourceHint.className = "model-source-hint live";
    } else if (effSource === "loading") {
      els.cfgModelSourceHint.textContent = t("chat.loading");
      els.cfgModelSourceHint.className = "model-source-hint";
    } else {
      els.cfgModelSourceHint.textContent = t("chat.add_api_key");
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
    document.getElementById("cfg-notify-complete").checked = state.config.notify_on_complete !== false;
    document.getElementById("cfg-system-prompt").value = state.config.system_prompt || "";
    const tg = state.config.telegram || {};
    els.tgEnabled.checked = !!tg.enabled;
    els.tgTokenStatus.textContent = tg.token_masked
      ? t("telegram.token_saved", { masked: tg.token_masked })
      : t("telegram.token_not_set");
    els.tgAllowedIds.value = (tg.allowed_chat_ids || []).join(", ");
    const tgStatus = tg.status || "Disabled";
    const tgOk = tgStatus.startsWith("Connected");
    const tgWarn = tg.enabled && !tgOk;
    els.tgStatusBadge.className = `status-pill ${tgOk ? "ok" : tgWarn ? "warn" : ""}`;
    els.tgStatusBadge.querySelector(".label").textContent =
      tgStatus === "Disabled" ? t("telegram.disabled") : tgStatus;
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
      const accClass = `status-pill ${accEnabled ? "ok" : "warn"}`;
      const accText = accEnabled ? t("topbar.accessibility") : t("topbar.accessibility_off");
      els.badgeAccessibility.className = accClass;
      els.badgeAccessibility.querySelector(".label").textContent = accText;
      els.mobileBadgeAccessibility.className = accClass;
      els.mobileBadgeAccessibility.querySelector(".label").textContent = accText;
      if (st.device?.device_name) {
        document.title = `${st.device.device_name} (Agentic Android)`;
      }
      if (st.device?.screen_width && st.device?.screen_height) {
        state.deviceWidth = st.device.screen_width;
        state.deviceHeight = st.device.screen_height;
        els.deviceDims.textContent = `${state.deviceWidth} × ${state.deviceHeight}`;
      }
      const mediaCast = !!st.device?.media_cast_active;
      state.highFpsActive = mediaCast;
      if (mediaCast) state.highFpsPending = false;
      updateHighFpsButtons();
    } catch {
      els.badgeAccessibility.className = "status-pill warn";
      els.badgeAccessibility.querySelector(".label").textContent = t("topbar.disconnected");
      els.mobileBadgeAccessibility.className = "status-pill warn";
      els.mobileBadgeAccessibility.querySelector(".label").textContent = t("topbar.disconnected");
    }
  }

  function connectVideoWs() {
    if (state.videoWs) state.videoWs.close();
    const ws = new WebSocket(wsUrl("/ws/video"));
    ws.binaryType = "blob";
    state.videoWs = ws;

    ws.onopen = () => {
      els.badgeVideo.className = "status-pill ok";
      els.badgeVideo.querySelector(".label").textContent = t("topbar.screen");
      els.mobileBadgeVideo.className = "status-pill ok";
      els.mobileBadgeVideo.querySelector(".label").textContent = t("topbar.screen");
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
      els.badgeVideo.querySelector(".label").textContent = t("topbar.reconnecting");
      els.mobileBadgeVideo.className = "status-pill warn";
      els.mobileBadgeVideo.querySelector(".label").textContent = t("topbar.reconnecting");
      setTimeout(connectVideoWs, 2000);
    };
  }

  function updateHighFpsButtons() {
    const label = state.highFpsActive
      ? t("topbar.high_fps_on")
      : state.highFpsPending
        ? t("topbar.high_fps_pending")
        : t("topbar.high_fps_off");
    els.btnHighFpsToggle.textContent = label;
    els.mobileBtnHighFps.textContent = label;
    els.btnHighFpsToggle.classList.toggle("active", state.highFpsActive);
    els.mobileBtnHighFps.classList.toggle("active", state.highFpsActive);
  }

  function updateAudioButtons() {
    const label = state.audioEnabled ? t("topbar.audio_on") : t("topbar.audio_off");
    els.btnAudioToggle.textContent = label;
    els.mobileBtnAudio.textContent = label;
    els.btnAudioToggle.classList.toggle("active", state.audioEnabled);
    els.mobileBtnAudio.classList.toggle("active", state.audioEnabled);
  }

  async function toggleHighFps() {
    if (state.highFpsActive) {
      state.highFpsActive = false;
      state.highFpsPending = false;
      updateHighFpsButtons();
      try {
        await api("/api/device/action", {
          method: "POST",
          body: JSON.stringify({ action: "stop_high_fps", params: {} }),
        });
      } catch {}
      setTimeout(refreshStatus, 400);
      return;
    }

    state.highFpsPending = true;
    updateHighFpsButtons();
    closeMobileOverlays();
    try {
      const res = await api("/api/device/action", {
        method: "POST",
        body: JSON.stringify({ action: "start_high_fps", params: {} }),
      });
      if (res?.media_cast_active) {
        state.highFpsActive = true;
        state.highFpsPending = false;
        updateHighFpsButtons();
        return;
      }
    } catch {
      state.highFpsPending = false;
      updateHighFpsButtons();
      return;
    }
    setTimeout(refreshStatus, 1000);
    setTimeout(refreshStatus, 3000);
    setTimeout(() => {
      state.highFpsPending = false;
      refreshStatus();
    }, 6000);
  }

  function toggleAudio() {
    if (state.audioEnabled) {
      state.audioEnabled = false;
      if (state.audioWs) {
        state.audioWs.close();
        state.audioWs = null;
      }
      updateAudioButtons();
      return;
    }

    state.audioEnabled = true;
    updateAudioButtons();
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
        if (evt.type === "agent_status") {
          els.mobileChatDot.classList.toggle("hidden", !evt.running);
          els.mobileChatLabel.textContent = evt.running ? t("chat.working") : t("chat.label");
          if (evt.session_id === state.activeSessionId) {
            els.agentActivity.classList.toggle("hidden", !evt.running);
            els.agentActivityText.textContent = evt.status || t("chat.working");
          }
        } else if (evt.type === "tool_call" && evt.session_id === state.activeSessionId) {
          els.agentActivityText.textContent = t("chat.executed_tool", { tool: evt.tool_name });
        } else if (evt.type === "assistant_message") {
          await loadSessions();
          notifyAgentCompleted(evt.message);
        } else if (evt.type === "session_updated" || evt.type === "error") {
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
    try { els.screenImg.setPointerCapture(e.pointerId); } catch {}
    pointerStart = toDeviceCoords(e.clientX, e.clientY);
  });

  // Mobile navigation & collapsible chat controls
  els.btnMobileMenu.addEventListener("click", () => {
    const opening = !els.sidebar.classList.contains("mobile-open");
    closeMobileOverlays();
    if (opening) {
      els.sidebar.classList.add("mobile-open");
      syncMobileBackdrop();
    }
  });

  els.btnCloseMobileMenu.addEventListener("click", closeMobileOverlays);

  els.btnMobileActions.addEventListener("click", () => {
    const opening = !els.deviceControls.classList.contains("mobile-open");
    closeMobileOverlays();
    if (opening) {
      els.deviceControls.classList.add("mobile-open");
      syncMobileBackdrop();
    }
  });

  els.btnCloseMobileActions.addEventListener("click", closeMobileOverlays);

  els.mobileChatHandle.addEventListener("click", () => {
    const expanding = !els.chatPane.classList.contains("mobile-expanded");
    closeMobileOverlays();
    if (expanding) {
      els.chatPane.classList.add("mobile-expanded");
      els.mobileChatHandle.setAttribute("aria-expanded", "true");
      syncMobileBackdrop();
      els.chatMessages.scrollTop = els.chatMessages.scrollHeight;
    }
  });

  els.mobileBackdrop.addEventListener("click", closeMobileOverlays);
  els.mobileBtnHighFps.addEventListener("click", toggleHighFps);
  els.mobileBtnAudio.addEventListener("click", toggleAudio);
  els.mobileBtnSettings.addEventListener("click", () => {
    closeMobileOverlays();
    openSettings();
  });
  els.mobileBtnLogout.addEventListener("click", () => {
    closeMobileOverlays();
    els.btnLogout.click();
  });
  els.mobileBtnUploadApk.addEventListener("click", () => {
    closeMobileOverlays();
    els.btnUploadApk.click();
  });
  els.mobileBtnRefreshFrame.addEventListener("click", () => els.btnRefreshFrame.click());
  els.mobileBtnInspectUi.addEventListener("click", () => {
    closeMobileOverlays();
    els.btnInspectUi.click();
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

  // Session, Memory & Credential creation controls
  els.btnNewSession.addEventListener("click", async () => {
    const s = await api("/api/sessions", {
      method: "POST",
      body: JSON.stringify({}),
    });
    state.activeSessionId = s.id;
    await loadSessions();
    openCenterTab({ key: `session:${s.id}`, kind: "session", id: s.id, title: s.title }, true);
  });

  els.btnNewMemory.addEventListener("click", () => {
    openCenterTab({ key: "memory:__new__", kind: "memory", id: null, title: t("memory.new_title") }, true);
  });

  els.btnNewCredential.addEventListener("click", () => {
    openCenterTab(
      { key: "credential:__new__", kind: "credential", id: null, title: t("credentials.new_title") },
      true
    );
  });

  els.btnToggleCredPw.addEventListener("click", () => {
    const showing = els.credPassword.type === "text";
    els.credPassword.type = showing ? "password" : "text";
    els.btnToggleCredPw.textContent = showing ? t("credentials.show") : t("credentials.hide");
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
    ensureNotificationPermission();
    els.chatInput.value = "";
    els.agentActivity.classList.remove("hidden");
    els.agentActivityText.textContent = t("chat.starting_agent");
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
    els.uitreePre.textContent = t("uitree.fetching");
    try {
      const tree = await api("/api/device/ui-tree");
      els.uitreePre.textContent = JSON.stringify(tree, null, 2);
    } catch (err) {
      els.uitreePre.textContent = t("uitree.error", { message: err.message });
    }
  });

  els.btnCloseUitree.addEventListener("click", () => {
    els.uitreeModal.classList.add("hidden");
  });

  // Sideload APK modal & upload with progress
  let selectedApkFile = null;
  let apkUploading = false;

  const formatBytes = (bytes) => {
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const setSelectedApk = (file) => {
    if (!file) return;
    const lower = file.name.toLowerCase();
    const valid =
      lower.endsWith(".apk") ||
      lower.endsWith(".xapk") ||
      lower.endsWith(".apks") ||
      lower.endsWith(".apkm");
    if (!valid) {
      els.apkStatus.textContent = t("apk.invalid_file");
      els.apkStatus.className = "apk-status err";
      return;
    }
    selectedApkFile = file;
    els.apkDropzoneLabel.textContent = file.name;
    els.apkFileMeta.textContent = formatBytes(file.size);
    els.apkStatus.textContent = "";
    els.apkStatus.className = "apk-status";
    els.apkProgressWrap.classList.add("hidden");
    els.apkProgressBar.style.width = "0%";
    els.btnStartApkUpload.disabled = false;
  };

  const openApkModal = () => {
    selectedApkFile = null;
    apkUploading = false;
    els.apkFileInput.value = "";
    els.apkDropzoneLabel.textContent = t("apk.drop_hint");
    els.apkFileMeta.textContent = "";
    els.apkStatus.textContent = "";
    els.apkStatus.className = "apk-status";
    els.apkProgressWrap.classList.add("hidden");
    els.apkProgressBar.style.width = "0%";
    els.btnStartApkUpload.disabled = true;
    els.apkModal.classList.remove("hidden");
  };

  els.btnUploadApk.addEventListener("click", openApkModal);
  els.btnCloseApk.addEventListener("click", () => {
    if (!apkUploading) els.apkModal.classList.add("hidden");
  });

  els.apkFileInput.addEventListener("change", () => {
    if (els.apkFileInput.files?.[0]) setSelectedApk(els.apkFileInput.files[0]);
  });

  ["dragenter", "dragover"].forEach((ev) => {
    els.apkDropzone.addEventListener(ev, (e) => {
      e.preventDefault();
      els.apkDropzone.classList.add("dragover");
    });
  });
  ["dragleave", "drop"].forEach((ev) => {
    els.apkDropzone.addEventListener(ev, (e) => {
      e.preventDefault();
      els.apkDropzone.classList.remove("dragover");
    });
  });
  els.apkDropzone.addEventListener("drop", (e) => {
    const file = e.dataTransfer?.files?.[0];
    if (file) setSelectedApk(file);
  });

  els.btnStartApkUpload.addEventListener("click", () => {
    if (!selectedApkFile || apkUploading) return;
    apkUploading = true;
    els.btnStartApkUpload.disabled = true;
    els.apkProgressWrap.classList.remove("hidden");
    els.apkProgressBar.style.width = "0%";
    els.apkStatus.className = "apk-status";
    els.apkStatus.textContent = t("apk.uploading", { pct: 0 });

    const xhr = new XMLHttpRequest();
    xhr.open("POST", "/api/device/install-apk");
    xhr.withCredentials = true;
    xhr.setRequestHeader("Content-Type", "application/octet-stream");
    xhr.setRequestHeader("X-Filename", selectedApkFile.name);

    xhr.upload.onprogress = (ev) => {
      if (ev.lengthComputable) {
        const pct = Math.min(100, Math.round((ev.loaded / ev.total) * 100));
        els.apkProgressBar.style.width = `${pct}%`;
        els.apkStatus.textContent =
          pct >= 100 ? t("apk.launching") : t("apk.uploading", { pct });
      }
    };

    xhr.onload = () => {
      apkUploading = false;
      let data = {};
      try {
        data = JSON.parse(xhr.responseText || "{}");
      } catch {}
      if (xhr.status === 401) {
        els.apkModal.classList.add("hidden");
        showLoginScreen();
        return;
      }
      if (xhr.status >= 200 && xhr.status < 300 && data.ok) {
        els.apkProgressBar.style.width = "100%";
        els.apkStatus.className = "apk-status ok";
        els.apkStatus.textContent = data.needs_permission
          ? t("apk.needs_permission")
          : t("apk.launched");
        setTimeout(() => {
          if (!apkUploading) els.apkModal.classList.add("hidden");
        }, 1600);
      } else {
        els.apkStatus.className = "apk-status err";
        els.apkStatus.textContent = data.error || data.result?.error || `HTTP ${xhr.status}`;
        els.btnStartApkUpload.disabled = false;
      }
    };

    xhr.onerror = () => {
      apkUploading = false;
      els.apkStatus.className = "apk-status err";
      els.apkStatus.textContent = t("apk.upload_failed");
      els.btnStartApkUpload.disabled = false;
    };

    xhr.send(selectedApkFile);
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

  els.btnHighFpsToggle.addEventListener("click", toggleHighFps);
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
    els.tabPanelTelegram.classList.toggle("hidden", tabName !== "telegram");
    els.tabPanelSecurity.classList.toggle("hidden", tabName !== "security");
    els.tabPanelEmulator.classList.toggle("hidden", tabName !== "emulator");
    if (tabName === "models") {
      fetchDefaultModels(els.cfgDefaultProvider.value, state.config?.default_model);
    } else if (tabName === "telegram") {
      loadConfig();
    } else if (tabName === "security") {
      refreshAuthStatus();
    } else if (tabName === "emulator") {
      loadEmulatorConfig();
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
    openCenterTab({ key: "settings", kind: "settings", id: "settings", title: t("settings.title") }, true);
  };
  els.btnOpenSettings.addEventListener("click", openSettings);
  els.btnWarnSettings.addEventListener("click", openSettings);

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
    els.keysSaveMsg.textContent = t("keys.saved", { provider: PROVIDER_LABELS[provider] || provider });
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
      notify_on_complete: document.getElementById("cfg-notify-complete").checked,
    };
    if (payload.notify_on_complete) {
      ensureNotificationPermission();
    }
    await api("/api/config", {
      method: "PUT",
      body: JSON.stringify(payload),
    });
    await loadConfig();
    els.settingsSaveMsg.textContent = t("models.saved");
  });

  els.tabPanelPrompt.addEventListener("submit", async (e) => {
    e.preventDefault();
    const systemPrompt = document.getElementById("cfg-system-prompt").value;
    await api("/api/config", {
      method: "PUT",
      body: JSON.stringify({ system_prompt: systemPrompt }),
    });
    await loadConfig();
    els.promptSaveMsg.textContent = t("prompt.saved");
  });

  async function loadAppMemory() {
    state.appMemory = (await api("/api/app-memory")) || [];
    const byPkg = new Map(state.appMemory.map((p) => [p.package_name, p]));
    for (const tb of state.openTabs) {
      if (tb.kind === "memory" && tb.id) {
        const pb = byPkg.get(tb.id);
        if (pb) tb.title = pb.app_label || pb.package_name;
      }
    }
    renderMemorySidebar();
    renderCenterTabs();
  }

  async function loadCredentials() {
    state.credentials = (await api("/api/credentials")) || [];
    const byId = new Map(state.credentials.map((c) => [c.id, c]));
    for (const tb of state.openTabs) {
      if (tb.kind === "credential" && tb.id) {
        const c = byId.get(tb.id);
        if (c) tb.title = c.label || c.target || c.username;
      }
    }
    renderCredentialsSidebar();
    renderCenterTabs();
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
    const label = els.memLabel.value.trim();
    const purpose = els.memPurpose.value.trim();
    await api(`/api/app-memory/${encodeURIComponent(pkg)}`, {
      method: "PUT",
      body: JSON.stringify({ app_label: label, purpose, notes }),
    });
    await loadAppMemory();
    const curTab = getActiveTab();
    const newKey = `memory:${pkg}`;
    if (curTab && curTab.kind === "memory" && curTab.key !== newKey) {
      curTab.key = newKey;
      curTab.id = pkg;
      curTab.title = label || pkg;
      state.activeTabKey = newKey;
    }
    els.memEditorTitle.textContent = label || pkg;
    els.btnDeleteMemory.classList.remove("hidden");
    renderCenterTabs();
    renderMemorySidebar();
    els.memSaveMsg.textContent = t("memory.saved");
  });

  els.btnDeleteMemory.addEventListener("click", async () => {
    const tab = getActiveTab();
    const pkg = tab?.kind === "memory" && tab.id ? tab.id : els.memPackage.value.trim();
    if (!pkg) return;
    await api(`/api/app-memory/${encodeURIComponent(pkg)}`, { method: "DELETE" });
    closeCenterTab(`memory:${pkg}`);
    await loadAppMemory();
  });

  els.credentialForm.addEventListener("submit", async (e) => {
    e.preventDefault();
    els.credSaveMsg.textContent = "";
    const payload = {
      label: els.credLabel.value.trim(),
      target: els.credTarget.value.trim(),
      username: els.credUsername.value.trim(),
      password: els.credPassword.value,
      notes: els.credNotes.value.trim(),
    };
    if (!payload.label && !payload.target) return;

    const curTab = getActiveTab();
    let saved;
    if (curTab && curTab.kind === "credential" && curTab.id) {
      saved = await api(`/api/credentials/${encodeURIComponent(curTab.id)}`, {
        method: "PUT",
        body: JSON.stringify(payload),
      });
    } else {
      saved = await api("/api/credentials", {
        method: "POST",
        body: JSON.stringify(payload),
      });
    }

    await loadCredentials();
    if (curTab && curTab.kind === "credential") {
      const newKey = `credential:${saved.id}`;
      curTab.key = newKey;
      curTab.id = saved.id;
      curTab.title = saved.label || saved.target || saved.username;
      state.activeTabKey = newKey;
    }
    els.credEditorTitle.textContent = saved.label || saved.target || t("credentials.form_title");
    els.btnDeleteCredential.classList.remove("hidden");
    renderCenterTabs();
    renderCredentialsSidebar();
    els.credSaveMsg.textContent = t("credentials.saved");
  });

  els.btnDeleteCredential.addEventListener("click", async () => {
    const tab = getActiveTab();
    if (!tab || tab.kind !== "credential" || !tab.id) return;
    await api(`/api/credentials/${encodeURIComponent(tab.id)}`, { method: "DELETE" });
    closeCenterTab(tab.key);
    await loadCredentials();
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
    els.tgSaveMsg.textContent = t("telegram.saved");
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
    els.mobileBtnLogout.classList.toggle("hidden", !st.auth_enabled);
    els.secAuthBadge.className = `status-pill ${st.auth_enabled ? "ok" : ""}`;
    els.secAuthBadge.querySelector(".label").textContent = st.auth_enabled
      ? t("security.enabled_user", { user: st.username || t("security.active") })
      : t("security.disabled");
    if (st.username) els.secUsername.value = st.username;
    els.btnSaveCredentials.textContent = st.auth_enabled
      ? t("security.update_credentials")
      : t("security.enable_login");
    els.btnDisableAuth.classList.toggle("hidden", !st.auth_enabled);

    els.secMfaBadge.className = `status-pill ${st.mfa_enabled ? "ok" : ""}`;
    els.secMfaBadge.querySelector(".label").textContent = st.mfa_enabled
      ? t("security.active")
      : t("security.off");
    els.btnSetupMfa.disabled = !st.auth_enabled;
    els.btnSetupMfa.style.opacity = st.auth_enabled ? "1" : "0.45";
    els.btnSetupMfa.textContent = st.mfa_enabled
      ? t("security.reconfigure_mfa")
      : t("security.setup_mfa");
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
      els.secMsg.textContent = t("security.credentials_saved");
    } catch (err) {
      els.secMsg.textContent = err.message;
    }
  });

  els.btnDisableAuth.addEventListener("click", async () => {
    await api("/api/auth/disable", { method: "POST" });
    els.secMfaSetupBox.classList.add("hidden");
    await refreshAuthStatus();
    els.secMsg.textContent = t("security.login_disabled");
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
      els.secMsg.textContent = t("security.mfa_enabled");
    } catch (err) {
      els.secMsg.textContent = err.message;
    }
  });

  els.btnDisableMfa.addEventListener("click", async () => {
    await api("/api/auth/mfa/disable", { method: "POST" });
    els.secMfaSetupBox.classList.add("hidden");
    await refreshAuthStatus();
    els.secMsg.textContent = t("security.mfa_disabled");
  });

  // Managed Emulator Config (/emulator-config)
  let emuPollTimer = null;

  async function loadEmulatorConfig() {
    try {
      const data = await api("/emulator-config");
      const managed = !!data?.managed;
      els.tabBtnEmulator.classList.toggle("hidden", !managed);
      if (!managed) return;

      els.emuCurrentAvd.textContent = data.current_avd || "--";
      els.emuMetaDesc.textContent = `ABI: ${data.abi || "--"} · GPU: ${data.gpu || "--"}${data.task_status ? ` · ${data.task_status}` : ""}`;

      const prevAvd = els.emuAvdSelect.value;
      els.emuAvdSelect.innerHTML = "";
      for (const avd of data.avds || []) {
        const opt = document.createElement("option");
        opt.value = avd;
        opt.textContent = avd === data.current_avd ? `${avd} (${t("emulator.active")})` : avd;
        if (avd === (prevAvd || data.current_avd)) opt.selected = true;
        els.emuAvdSelect.appendChild(opt);
      }

      const prevDev = els.emuNewDevice.value;
      if (Array.isArray(data.available_devices) && data.available_devices.length > 0) {
        els.emuNewDevice.innerHTML = "";
        for (const dev of data.available_devices) {
          const opt = document.createElement("option");
          opt.value = dev;
          opt.textContent = dev
            .split("_")
            .map((w) => (w ? w[0].toUpperCase() + w.slice(1) : ""))
            .join(" ");
          if (dev === (prevDev || "pixel_6")) opt.selected = true;
          els.emuNewDevice.appendChild(opt);
        }
      }

      const prevImg = els.emuNewImage.value;
      const installedSet = new Set(data.installed_images || []);
      const allImgs = Array.from(
        new Set([...(data.available_images || []), ...(data.installed_images || [])])
      );
      els.emuNewImage.innerHTML = "";
      for (const img of allImgs) {
        const opt = document.createElement("option");
        opt.value = img;
        opt.textContent = installedSet.has(img) ? `${img} (${t("emulator.installed")})` : img;
        if (img === prevImg) opt.selected = true;
        els.emuNewImage.appendChild(opt);
      }

      const busy = !!data.busy;
      els.btnEmuRestart.disabled = busy;
      els.btnEmuSwitch.disabled = busy;
      els.btnEmuCreate.disabled = busy;
      if (data.task_status) {
        els.emuMsg.textContent = data.task_status;
      }

      if (busy && !emuPollTimer) {
        emuPollTimer = setInterval(loadEmulatorConfig, 3000);
      } else if (!busy && emuPollTimer) {
        clearInterval(emuPollTimer);
        emuPollTimer = null;
      }
    } catch {
      els.tabBtnEmulator.classList.add("hidden");
    }
  }

  els.btnEmuRestart.addEventListener("click", async () => {
    els.emuMsg.textContent = t("emulator.restarting");
    try {
      await api("/emulator-config/restart", { method: "POST", body: JSON.stringify({}) });
      setTimeout(loadEmulatorConfig, 1500);
    } catch (err) {
      els.emuMsg.textContent = err.message;
    }
  });

  els.btnEmuSwitch.addEventListener("click", async () => {
    const avd = els.emuAvdSelect.value;
    if (!avd) return;
    els.emuMsg.textContent = t("emulator.switching", { avd });
    try {
      await api("/emulator-config/switch-avd", {
        method: "POST",
        body: JSON.stringify({ avd }),
      });
      setTimeout(loadEmulatorConfig, 1500);
    } catch (err) {
      els.emuMsg.textContent = err.message;
    }
  });

  els.emuCreateForm.addEventListener("submit", async (e) => {
    e.preventDefault();
    const name = els.emuNewName.value.trim();
    const system_image = els.emuNewImage.value;
    const device = els.emuNewDevice.value;
    const switch_now = els.emuNewSwitch.checked;
    if (!name || !system_image) return;
    els.emuMsg.textContent = t("emulator.downloading");
    try {
      await api("/emulator-config/create-avd", {
        method: "POST",
        body: JSON.stringify({ name, system_image, device, switch_now }),
      });
      els.emuNewName.value = "";
      loadEmulatorConfig();
    } catch (err) {
      els.emuMsg.textContent = err.message;
    }
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
      els.loginError.textContent = err.message || t("login.failed");
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

  window.addEventListener("agentic:langchange", () => {
    updateHighFpsButtons();
    updateAudioButtons();
    for (const tb of state.openTabs) {
      if (tb.kind === "settings") tb.title = t("settings.title");
      else if (tb.kind === "memory" && !tb.id) tb.title = t("memory.new_title");
      else if (tb.kind === "credential" && !tb.id) tb.title = t("credentials.new_title");
    }
    if (appBooted) {
      renderAll();
      renderConfiguredKeys();
      refreshStatus();
      refreshAuthStatus().catch(() => {});
      if (state.config) {
        populateDefaultModels(els.cfgDefaultProvider.value, els.cfgDefaultModel.value);
      }
    }
  });

  async function bootApp() {
    if (appBooted) return;
    appBooted = true;
    await refreshAuthStatus();
    const pData = await api("/api/providers");
    state.providers = pData.providers || [];
    await loadConfig();
    await Promise.all([loadSessions(), loadAppMemory(), loadCredentials(), loadEmulatorConfig()]);
    await refreshStatus();
    statusInterval = setInterval(refreshStatus, 5000);
    connectVideoWs();
    connectEventsWs();
  }

  async function init() {
    if (window.I18n) {
      await window.I18n.init();
    }
    const authSt = await api("/api/auth/status");
    if (authSt.auth_enabled && !authSt.authenticated) {
      showLoginScreen(authSt.mfa_enabled);
      return;
    }
    await bootApp();
  }

  init().catch((err) => console.error("Init error:", err));
})();
