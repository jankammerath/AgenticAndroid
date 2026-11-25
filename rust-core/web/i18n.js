(() => {
  const SUPPORTED = ["en", "de", "zh-CN"];
  const STORAGE_KEY = "agentic_lang";
  const cache = {};
  let currentLang = "en";

  function normalizeLang(raw) {
    if (!raw) return "en";
    const low = String(raw).toLowerCase();
    if (low.startsWith("de")) return "de";
    if (low.startsWith("zh")) return "zh-CN";
    if (low.startsWith("en")) return "en";
    return SUPPORTED.includes(raw) ? raw : "en";
  }

  function detectInitialLang() {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved && SUPPORTED.includes(saved)) return saved;
    } catch {}
    const navList = navigator.languages && navigator.languages.length
      ? navigator.languages
      : [navigator.language || "en"];
    for (const candidate of navList) {
      const norm = normalizeLang(candidate);
      if (norm !== "en" || String(candidate).toLowerCase().startsWith("en")) {
        return norm;
      }
    }
    return "en";
  }

  async function loadDict(lang) {
    if (cache[lang]) return cache[lang];
    const res = await fetch(`/i18n/${encodeURIComponent(lang)}.json`, {
      credentials: "same-origin",
    });
    if (!res.ok) throw new Error(`Failed to load locale ${lang}`);
    const data = await res.json();
    cache[lang] = data;
    return data;
  }

  function interpolate(template, vars) {
    if (!vars) return template;
    return String(template).replace(/\{(\w+)\}/g, (_, k) =>
      vars[k] !== undefined && vars[k] !== null ? String(vars[k]) : `{${k}}`
    );
  }

  function t(key, vars) {
    const dict = cache[currentLang] || {};
    const fallback = cache.en || {};
    const raw = dict[key] !== undefined ? dict[key] : fallback[key] !== undefined ? fallback[key] : key;
    return interpolate(raw, vars);
  }

  function apply(root = document) {
    document.documentElement.lang = currentLang;
    root.querySelectorAll("[data-i18n]").forEach((el) => {
      el.textContent = t(el.getAttribute("data-i18n"));
    });
    root.querySelectorAll("[data-i18n-html]").forEach((el) => {
      el.innerHTML = t(el.getAttribute("data-i18n-html"));
    });
    root.querySelectorAll("[data-i18n-placeholder]").forEach((el) => {
      el.setAttribute("placeholder", t(el.getAttribute("data-i18n-placeholder")));
    });
    root.querySelectorAll("[data-i18n-title]").forEach((el) => {
      el.setAttribute("title", t(el.getAttribute("data-i18n-title")));
    });
    root.querySelectorAll("[data-i18n-aria-label]").forEach((el) => {
      el.setAttribute("aria-label", t(el.getAttribute("data-i18n-aria-label")));
    });
    document.querySelectorAll(".lang-select").forEach((sel) => {
      sel.value = currentLang;
    });
  }

  async function setLang(lang) {
    const target = normalizeLang(lang);
    try {
      if (!cache.en) await loadDict("en");
      if (target !== "en") await loadDict(target);
      currentLang = target;
      try {
        localStorage.setItem(STORAGE_KEY, currentLang);
      } catch {}
    } catch {
      currentLang = "en";
    }
    apply();
    window.dispatchEvent(new CustomEvent("agentic:langchange", { detail: { lang: currentLang } }));
    return currentLang;
  }

  async function init() {
    const initial = detectInitialLang();
    await loadDict("en").catch(() => {});
    if (initial !== "en") {
      await loadDict(initial).catch(() => {});
    }
    currentLang = cache[initial] ? initial : "en";
    apply();
    document.querySelectorAll(".lang-select").forEach((sel) => {
      sel.value = currentLang;
      sel.addEventListener("change", () => setLang(sel.value));
    });
    return currentLang;
  }

  window.I18n = {
    SUPPORTED,
    init,
    setLang,
    getLang: () => currentLang,
    t,
    apply,
  };
})();
