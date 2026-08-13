// Kiến trúc song ngữ (FR-30, FR-31): file ngôn ngữ dạng key-value JSON, cộng đồng có
// thể thêm ngôn ngữ mới bằng cách thêm 1 file locales/<mã>.json — không cần sửa code
// ở đây, chỉ cần đăng ký thêm vào DICTS bên dưới.
import vi from "./locales/vi.json";
import en from "./locales/en.json";

export type Lang = "vi" | "en";
type Dict = Record<string, string>;

const DICTS: Record<Lang, Dict> = { vi, en };
const STORAGE_KEY = "vietzip.lang";

/** FR-29: tự động nhận diện ngôn ngữ hệ điều hành để chọn mặc định. */
function detectDefaultLang(): Lang {
  const systemLang = navigator.language?.toLowerCase() ?? "";
  return systemLang.startsWith("vi") ? "vi" : "en";
}

function loadSavedLang(): Lang | null {
  const saved = localStorage.getItem(STORAGE_KEY);
  return saved === "vi" || saved === "en" ? saved : null;
}

let currentLang: Lang = loadSavedLang() ?? detectDefaultLang();

export function getLang(): Lang {
  return currentLang;
}

/** Người dùng đổi ngôn ngữ thủ công — được nhớ lại cho lần mở app sau. */
export function setLang(lang: Lang) {
  currentLang = lang;
  localStorage.setItem(STORAGE_KEY, lang);
  applyStaticTranslations();
}

export function t(key: string, vars?: Record<string, string | number>): string {
  const dict = DICTS[currentLang];
  let text = dict[key] ?? DICTS.vi[key] ?? key;
  if (vars) {
    for (const [name, value] of Object.entries(vars)) {
      text = text.replace(`{${name}}`, String(value));
    }
  }
  return text;
}

/** Dịch mọi phần tử tĩnh trong trang: [data-i18n] cho nội dung, [data-i18n-placeholder]
 * cho placeholder của input. Nội dung động (thông báo trạng thái, bảng entries) do
 * code gọi t() trực tiếp lúc render, không qua hàm này. */
export function applyStaticTranslations() {
  document.documentElement.lang = currentLang;
  document.querySelectorAll<HTMLElement>("[data-i18n]").forEach((el) => {
    const key = el.dataset.i18n;
    if (key) el.textContent = t(key);
  });
  document.querySelectorAll<HTMLInputElement>("[data-i18n-placeholder]").forEach((el) => {
    const key = el.dataset.i18nPlaceholder;
    if (key) el.placeholder = t(key);
  });
  document.querySelectorAll<HTMLElement>("[data-i18n-aria-label]").forEach((el) => {
    const key = el.dataset.i18nAriaLabel;
    if (key) el.setAttribute("aria-label", t(key));
  });
}
