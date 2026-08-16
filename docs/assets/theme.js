(function () {
  "use strict";

  var STORAGE_KEY = "vietzip-site-theme";

  function storedTheme() {
    return localStorage.getItem(STORAGE_KEY); // "light" | "dark" | null (= follow system)
  }

  function isDarkNow() {
    var stored = storedTheme();
    if (stored) return stored === "dark";
    return window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches;
  }

  function applyTheme() {
    var stored = storedTheme();
    if (stored) {
      document.documentElement.setAttribute("data-theme", stored);
    } else {
      document.documentElement.removeAttribute("data-theme");
    }
    document.querySelectorAll(".theme-btn").forEach(function (btn) {
      // Icon shows the action clicking it performs: moon = switch to dark, sun = switch to light.
      btn.textContent = isDarkNow() ? "☀️" : "🌙";
      btn.setAttribute("aria-label", isDarkNow() ? "Chuyển sang giao diện sáng" : "Chuyển sang giao diện tối");
    });
  }

  function toggleTheme() {
    localStorage.setItem(STORAGE_KEY, isDarkNow() ? "light" : "dark");
    applyTheme();
  }

  document.addEventListener("DOMContentLoaded", function () {
    applyTheme();
    document.querySelectorAll(".theme-btn").forEach(function (btn) {
      btn.addEventListener("click", toggleTheme);
    });
  });

  if (window.matchMedia) {
    window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", function () {
      if (!storedTheme()) applyTheme();
    });
  }
})();
