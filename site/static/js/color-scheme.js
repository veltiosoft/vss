(() => {
  const key = "vss-color-scheme";
  const root = document.documentElement;
  const meta = document.querySelector('meta[name="color-scheme"]');
  const button = document.querySelector(".theme-toggle");
  const preference = matchMedia("(prefers-color-scheme: dark)");

  let storage;
  try {
    storage = window.localStorage;
  } catch {
    // Manual changes still work on this page without persistence.
  }

  const systemScheme = () => preference.matches ? "dark" : "light";
  const currentScheme = () => root.dataset.colorScheme ?? systemScheme();

  function updateButton() {
    button.setAttribute("aria-pressed", String(currentScheme() === "dark"));
  }

  function applyOverride(scheme) {
    if (scheme === "light" || scheme === "dark") {
      root.dataset.colorScheme = scheme;
      meta.content = scheme;
    } else {
      delete root.dataset.colorScheme;
      meta.content = "light dark";
    }
    updateButton();
  }

  button.addEventListener("click", () => {
    const target = currentScheme() === "dark" ? "light" : "dark";
    const override = target === systemScheme() ? null : target;
    applyOverride(override);

    try {
      if (override === null) {
        storage?.removeItem(key);
      } else {
        storage?.setItem(key, override);
      }
    } catch {
      // Keep the change on this page even if storage rejects the write.
    }
  });

  preference.addEventListener("change", updateButton);
  window.addEventListener("storage", event => {
    if (event.storageArea === storage && (event.key === key || event.key === null)) {
      applyOverride(event.newValue);
    }
  });

  updateButton();
  button.hidden = false;
})();
