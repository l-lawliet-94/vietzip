import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { applyStaticTranslations, getLang, setLang, t, type Lang } from "./i18n";
import { DRAG_ICON_DATA_URL } from "./drag-icon";

interface EntryDto {
  name: string;
  size: number;
  is_dir: boolean;
}

/** Khớp với `CommandError` phía Rust (src-tauri/src/lib.rs). */
interface CommandError {
  kind: string;
  message: string;
}

// "001" cho phép chọn thẳng phần đầu của 1 file đã chia (`archive.zip.001`) trong hộp thoại
// Giải nén/Kiểm tra/Xem — core tự ghép ngầm (xem `split.rs::resolve_if_split`), khớp hành vi
// thật của 7-Zip khi mở trực tiếp `.7z.001`/`.zip.001` mà không cần bấm "Ghép file..." trước.
// cab/cpio/deb/rpm: định dạng chỉ-giải-nén mở rộng theo yêu cầu phủ toàn bộ danh sách 7-Zip
// hỗ trợ (xem CLAUDE.md mục "7-Zip feature parity").
// "exe" CỐ TÌNH không có trong danh sách này, khác mọi đuôi khác — installer NSIS chỉ là 1
// file .exe Windows bình thường (xem nsis_format.rs), nên khác mọi đuôi còn lại (hầu như luôn
// LÀ file nén), tuyệt đại đa số file .exe KHÔNG phải NSIS. Đưa "exe" vào đây sẽ khiến: (1) kéo-
// thả bất kỳ .exe nào vào cửa sổ app tự động thử "Xem..." rồi báo lỗi thay vì rơi về "Nén..."
// hợp lý hơn (xem onFilesDropped), và (2) mọi entry .exe bên trong 1 zip bình thường (rất phổ
// biến — phân phối chương trình Windows qua zip) hiện icon 📦 và thử mở như archive lồng khi
// nhấp đúp (xem renderCurrentFolder/isArchivePath) — cả 2 đều sai lệch, mơ hồ, trái NFR-11.
// Giải nén NSIS vẫn hoạt động đầy đủ qua CLI (đã verify) và qua Desktop bằng cách gõ thẳng tên
// file trong hộp thoại chọn file (không dựa vào bộ lọc theo đuôi này).
const ARCHIVE_EXTENSIONS = [
  "zip", "7z", "rar", "tar", "gz", "bz2", "zst", "xz", "tgz", "tbz2", "txz", "001",
  "cab", "cpio", "deb", "rpm", "lzh", "lha", "ext2", "ext3", "ext4", "arj", "chm", "udf",
];

function archiveFilters() {
  return [{ name: t("filter.archives"), extensions: ARCHIVE_EXTENSIONS }];
}

function isArchivePath(path: string): boolean {
  const lower = path.toLowerCase();
  return ARCHIVE_EXTENSIONS.some((ext) => lower.endsWith(`.${ext}`));
}

let statusEl: HTMLElement | null;
let statusTextEl: HTMLElement | null;
let entriesBodyEl: HTMLElement | null;
let entriesTableEl: HTMLElement | null;
let entriesEmptyEl: HTMLElement | null;
let resultsViewingEl: HTMLElement | null;
let addToArchiveBtnEl: HTMLElement | null;
let passwordEl: HTMLInputElement | null;
let levelEl: HTMLSelectElement | null;
let splitSizeEl: HTMLInputElement | null;
let benchmarkTableEl: HTMLElement | null;
let benchmarkBodyEl: HTMLElement | null;
let checksumResultEl: HTMLElement | null;
let dropOverlayEl: HTMLElement | null;

/** FR-18/19. Đường dẫn archive đang xem qua "Xem...", dùng cho Thêm/Xoá entry tại chỗ.
 * `null` nếu chưa xem file nào hoặc thao tác gần nhất không phải trên 1 file .zip cụ thể. */
let currentArchivePath: string | null = null;

/** Archive + mật khẩu đang xem, dùng cho kéo-thả entry ra ngoài — khác `currentArchivePath`
 * ở chỗ áp dụng cho MỌI định dạng (không chỉ .zip), vì `extract_entry_for_drag` hoạt động
 * xuyên suốt mọi định dạng core hỗ trợ, không chỉ định dạng có thể sửa tại chỗ. */
let currentViewedArchive: { path: string; password: string | undefined } | null = null;

/** Ngăn xếp các archive "ngoài" đang tạm rời đi để xem 1 archive lồng bên trong (double-click
 * 1 entry chính nó cũng là file nén nhận diện được — xem `openNestedArchive`). Rỗng nếu đang
 * xem archive gốc, không lồng trong archive nào khác. Mỗi phần tử nhớ đúng vị trí thư mục
 * đang xem trong archive ngoài đó, để quay lại đúng chỗ thay vì luôn về thư mục gốc. */
let archiveViewStack: { path: string; password: string | undefined; folder: string }[] = [];

type StatusKind = "idle" | "loading" | "success" | "error";

/** Trạng thái luôn hiển thị ngay dưới tiêu đề: sẵn sàng / đang xử lý / thành công / lỗi —
 * phân biệt bằng icon + màu nền, không chỉ đổi màu chữ, để người dùng luôn biết app đang
 * làm gì mà không phải đoán. */
function setStatus(message: string, kind: StatusKind = "success") {
  if (!statusEl || !statusTextEl) return;
  statusTextEl.textContent = message;
  statusEl.classList.remove("status-idle", "status-loading", "status-success", "status-error");
  statusEl.classList.add(`status-${kind}`);
}

/** Cờ toàn cục để biết app có đang bận không, dùng để bỏ qua thao tác kéo-thả trong lúc 1
 * lệnh khác đang chạy (tránh 2 thao tác chồng lên nhau — xem `onFilesDropped`). */
let appBusy = false;

/** Vô hiệu hoá mọi nút thao tác trong lúc đang xử lý, tránh bấm trùng lặp và cho thấy rõ
 * ràng là app đang bận — nút chuyển ngôn ngữ vẫn dùng được vì không liên quan tới tác vụ. */
function setBusy(busy: boolean) {
  appBusy = busy;
  document.querySelectorAll<HTMLButtonElement>("button").forEach((btn) => {
    if (btn.classList.contains("lang-btn")) return;
    btn.disabled = busy;
  });
}

/** Bọc 1 thao tác bất đồng bộ: hiện trạng thái "đang xử lý" + khoá nút ngay khi bắt đầu,
 * và luôn mở khoá nút khi xong (dù thành công hay lỗi). `action` tự chịu trách nhiệm set
 * trạng thái thành công/lỗi cuối cùng. */
async function withProgress(loadingMessage: string, action: () => Promise<void>) {
  setStatus(loadingMessage, "loading");
  setBusy(true);
  try {
    await action();
  } finally {
    setBusy(false);
  }
}

function currentPassword(): string | undefined {
  const value = passwordEl?.value ?? "";
  return value.length > 0 ? value : undefined;
}

/** FR-03. Giá trị của `<select id="level">`: "fast" | "normal" | "ultra". */
function currentLevel(): string {
  return levelEl?.value ?? "normal";
}

function baseName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/** Điều hướng thư mục bên trong file nén (giống 1 file manager thu nhỏ, không phải bảng
 * phẳng như trước) — vẫn không hỗ trợ kéo-thả hay duyệt vào archive lồng trong archive,
 * chỉ điều hướng cây thư mục của chính file nén đang xem. `currentEntries` luôn là danh
 * sách ĐẦY ĐỦ (phẳng) từ lần gọi `list_archive_entries` gần nhất; `currentViewFolder` là
 * đường dẫn thư mục đang đứng (rỗng = gốc, luôn kết thúc bằng "/" nếu không rỗng). */
let currentEntries: EntryDto[] = [];
let currentViewFolder = "";
let breadcrumbEl: HTMLElement | null;

/** Đa chọn: tên đầy đủ (không rút gọn theo thư mục hiện tại) các file đang được tick chọn
 * trong thư mục đang xem, để kéo nhiều file ra ngoài cùng lúc — xem `scheduleDragStart`. Bị
 * xoá mỗi khi danh sách hiển thị đổi (điều hướng thư mục, nạp lại sau thêm/xoá/đổi tên),
 * tránh giữ lại 1 lựa chọn "vô hình" không còn khớp với những gì đang hiển thị. */
let selectedEntryNames = new Set<string>();

interface FolderView {
  folders: string[];
  files: EntryDto[];
}

/** Tính danh sách thư mục con + file con TRỰC TIẾP của `folder`, suy ra cả từ những entry
 * không có dòng thư mục riêng (không phải mọi file nén đều ghi entry cho từng thư mục cha —
 * ví dụ RAR/TAR thường chỉ có entry file, không có entry thư mục rỗng). */
function computeFolderView(entries: EntryDto[], folder: string): FolderView {
  const folderNames = new Set<string>();
  const files: EntryDto[] = [];
  for (const entry of entries) {
    if (folder && !entry.name.startsWith(folder)) continue;
    const rest = folder ? entry.name.slice(folder.length) : entry.name;
    if (rest === "") continue; // chính entry thư mục hiện tại (nếu có)
    const slashIdx = rest.indexOf("/");
    if (slashIdx === -1) {
      if (entry.is_dir) folderNames.add(rest);
      else files.push(entry);
    } else {
      folderNames.add(rest.slice(0, slashIdx));
    }
  }
  return { folders: Array.from(folderNames).sort((a, b) => a.localeCompare(b)), files };
}

function renderBreadcrumb() {
  if (!breadcrumbEl) return;
  breadcrumbEl.innerHTML = "";

  // Đang xem 1 archive lồng bên trong 1 archive khác — cho quay lại archive ngoài, khác hẳn
  // breadcrumb thư mục bên dưới (đó là điều hướng BÊN TRONG cùng 1 archive).
  if (archiveViewStack.length > 0) {
    const outer = archiveViewStack[archiveViewStack.length - 1];
    const backLink = document.createElement("button");
    backLink.className = "breadcrumb-link";
    backLink.textContent = `\u{2B05}\u{FE0F} ${baseName(outer.path)}`;
    backLink.title = t("results.backToOuterArchive");
    backLink.addEventListener("click", () => void goBackToOuterArchive());
    breadcrumbEl.appendChild(backLink);
    breadcrumbEl.appendChild(document.createTextNode(" \u{00BB} "));
  }

  const segments = currentViewFolder.split("/").filter((s) => s.length > 0);

  const rootLink = document.createElement("button");
  rootLink.className = "breadcrumb-link";
  rootLink.textContent = "\u{1F4C1}";
  rootLink.addEventListener("click", () => {
    currentViewFolder = "";
    renderCurrentFolder();
  });
  breadcrumbEl.appendChild(rootLink);

  let pathSoFar = "";
  for (const segment of segments) {
    pathSoFar += `${segment}/`;
    const targetPath = pathSoFar;
    breadcrumbEl.appendChild(document.createTextNode(" / "));
    const link = document.createElement("button");
    link.className = "breadcrumb-link";
    link.textContent = segment;
    link.addEventListener("click", () => {
      currentViewFolder = targetPath;
      renderCurrentFolder();
    });
    breadcrumbEl.appendChild(link);
  }
}

function renderCurrentFolder() {
  if (!entriesBodyEl || !entriesTableEl || !entriesEmptyEl) return;
  selectedEntryNames = new Set(); // danh sách hiển thị sắp đổi — bỏ chọn cũ, xem khai báo ở trên
  renderBreadcrumb();
  entriesBodyEl.innerHTML = "";

  const { folders, files } = computeFolderView(currentEntries, currentViewFolder);

  if (currentViewFolder) {
    const row = document.createElement("tr");
    row.className = "entry-row-folder";
    const checkbox = document.createElement("td");
    const icon = document.createElement("td");
    icon.textContent = "\u{2B06}\u{FE0F}";
    const name = document.createElement("td");
    name.textContent = "..";
    row.append(checkbox, icon, name, document.createElement("td"), document.createElement("td"));
    row.addEventListener("click", () => {
      const trimmed = currentViewFolder.replace(/\/$/, "");
      const idx = trimmed.lastIndexOf("/");
      currentViewFolder = idx === -1 ? "" : trimmed.slice(0, idx + 1);
      renderCurrentFolder();
    });
    entriesBodyEl.appendChild(row);
  }

  for (const folderName of folders) {
    const row = document.createElement("tr");
    row.className = "entry-row-folder";
    const checkbox = document.createElement("td");
    const icon = document.createElement("td");
    icon.textContent = "\u{1F4C1}";
    const name = document.createElement("td");
    name.textContent = folderName;
    row.append(checkbox, icon, name, document.createElement("td"), document.createElement("td"));
    row.addEventListener("click", () => {
      currentViewFolder += `${folderName}/`;
      renderCurrentFolder();
    });
    entriesBodyEl.appendChild(row);
  }

  for (const entry of files) {
    const row = document.createElement("tr");
    row.className = "entry-row-file";
    row.addEventListener("mousedown", (event) => scheduleDragStart(entry.name, event));

    const isNestedArchive = isArchivePath(entry.name);
    if (isNestedArchive) {
      row.title = t("results.nestedArchiveHint");
      row.addEventListener("dblclick", () => {
        cancelPendingDragStart();
        void openNestedArchive(entry.name);
      });
    } else {
      row.title = t("results.dragOutHint");
    }

    const checkboxCell = document.createElement("td");
    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.setAttribute("aria-label", t("table.selectEntry"));
    // `mousedown` trên chính checkbox không được nổi bọt lên `scheduleDragStart` của dòng —
    // tick chọn chỉ để đánh dấu, không phải để bắt đầu kéo dòng đó đi ngay lập tức.
    checkbox.addEventListener("mousedown", (event) => event.stopPropagation());
    checkbox.addEventListener("change", () => {
      if (checkbox.checked) selectedEntryNames.add(entry.name);
      else selectedEntryNames.delete(entry.name);
    });
    checkboxCell.appendChild(checkbox);

    const kind = document.createElement("td");
    kind.textContent = isNestedArchive ? "\u{1F4E6}" : "\u{1F4C4}";
    const name = document.createElement("td");
    name.textContent = entry.name.slice(currentViewFolder.length);
    const size = document.createElement("td");
    size.textContent = String(entry.size);
    const actions = document.createElement("td");
    if (currentArchivePath) {
      const renameBtn = document.createElement("button");
      renameBtn.className = "btn-rename-entry";
      renameBtn.textContent = t("manage.renameEntry");
      renameBtn.addEventListener("click", () => onRenameEntry(entry.name));
      actions.appendChild(renameBtn);

      const removeBtn = document.createElement("button");
      removeBtn.className = "btn-remove-entry";
      removeBtn.textContent = t("manage.removeEntry");
      removeBtn.addEventListener("click", () => onRemoveEntry(entry.name));
      actions.appendChild(removeBtn);
    }
    row.append(checkboxCell, kind, name, size, actions);
    entriesBodyEl.appendChild(row);
  }

  const hasEntries = currentViewFolder.length > 0 || folders.length > 0 || files.length > 0;
  entriesTableEl.classList.toggle("hidden", !hasEntries);
  entriesEmptyEl.classList.toggle("hidden", hasEntries);
}

/** Nạp danh sách entry mới (sau "Xem..." hoặc sau thêm/xoá/đổi tên) — luôn quay về thư mục
 * gốc, vì cấu trúc thư mục có thể đã đổi (ví dụ vừa xoá đúng thư mục đang đứng trong đó). */
function renderEntries(entries: EntryDto[]) {
  currentEntries = entries;
  currentViewFolder = "";
  renderCurrentFolder();
}

/** Dịch lỗi từ core sang ngôn ngữ hiện tại nếu là lỗi đã biết (FR-30), giữ nguyên
 * thông điệp gốc (tiếng Việt) từ core cho các lỗi chưa có bản dịch riêng (Io, Archive). */
function friendlyError(err: unknown): string {
  const asCommandError = err as Partial<CommandError> | undefined;
  if (asCommandError && typeof asCommandError.kind === "string") {
    const key = `error.kind.${asCommandError.kind}`;
    const translated = t(key);
    return translated !== key ? translated : (asCommandError.message ?? String(err));
  }
  return typeof err === "string" ? err : String(err);
}

async function onCompress(sources: string[]) {
  if (sources.length === 0) return;
  const dest = await save({
    title: t("dialog.saveCompressedAs"),
    filters: [
      { name: t("filter.zip"), extensions: ["zip"] },
      { name: t("filter.sevenz"), extensions: ["7z"] },
    ],
  });
  if (!dest) return;

  await withProgress(t("status.compressing"), async () => {
    try {
      await invoke("compress_files", {
        sources,
        dest,
        password: currentPassword(),
        level: currentLevel(),
      });
      setStatus(t("status.compressed", { path: dest }), "success");
    } catch (err) {
      setStatus(t("error.compress", { detail: friendlyError(err) }), "error");
    }
  });
}

async function onExtract() {
  const archive = await open({ title: t("dialog.selectArchiveToExtract"), multiple: false, filters: archiveFilters() });
  if (!archive || Array.isArray(archive)) return;

  const destDir = await open({ title: t("dialog.extractTo"), directory: true, multiple: false });
  if (!destDir || Array.isArray(destDir)) return;

  await withProgress(t("status.extracting"), async () => {
    try {
      await invoke("extract_archive", { archive, destDir, password: currentPassword() });
      setStatus(t("status.extracted", { path: destDir }), "success");
    } catch (err) {
      setStatus(t("error.extract", { detail: friendlyError(err) }), "error");
    }
  });
}

async function onTest() {
  const archive = await open({ title: t("dialog.selectArchiveToTest"), multiple: false, filters: archiveFilters() });
  if (!archive || Array.isArray(archive)) return;

  await withProgress(t("status.testing"), async () => {
    try {
      const ok = await invoke<boolean>("test_archive_integrity", { archive, password: currentPassword() });
      setStatus(ok ? t("status.testOk") : t("status.testFail"), ok ? "success" : "error");
    } catch (err) {
      setStatus(t("error.test", { detail: friendlyError(err) }), "error");
    }
  });
}

/** Thân của "Xem..." tách riêng khỏi hộp thoại chọn file, để kéo-thả (xem
 * `onFilesDropped`) có thể gọi thẳng với 1 đường dẫn đã biết, không cần mở hộp thoại lại. */
async function viewArchive(archive: string) {
  await withProgress(t("status.loadingEntries"), async () => {
    try {
      const entries = await invoke<EntryDto[]>("list_archive_entries", { archive, password: currentPassword() });
      currentViewedArchive = { path: archive, password: currentPassword() };
      // FR-18/19 chỉ hỗ trợ .zip (xem crates/core/src/zip_format.rs) — chỉ hiện nút "Thêm
      // file..." khi chắc chắn thao tác đó sẽ thành công, tránh lỗi khó hiểu cho định dạng khác.
      currentArchivePath = archive.toLowerCase().endsWith(".zip") ? archive : null;
      addToArchiveBtnEl?.classList.toggle("hidden", !currentArchivePath);
      if (resultsViewingEl) {
        resultsViewingEl.textContent = t("results.viewing", { name: baseName(archive) });
        resultsViewingEl.classList.remove("hidden");
      }
      renderEntries(entries);
      setStatus(t("status.entriesCount", { count: entries.length }), "success");
    } catch (err) {
      setStatus(t("error.view", { detail: friendlyError(err) }), "error");
    }
  });
}

async function onView() {
  const archive = await open({ title: t("dialog.selectArchiveToView"), multiple: false, filters: archiveFilters() });
  if (!archive || Array.isArray(archive)) return;
  archiveViewStack = []; // đang mở 1 archive gốc mới, không phải đi tiếp/lùi trong ngăn xếp lồng nhau
  await viewArchive(archive);
}

/** Kéo 1 hoặc NHIỀU dòng file đã chọn trong bảng nội dung RA NGOÀI (thả vào Explorer/Desktop)
 * — nửa còn thiếu của mục "drag-and-drop" so với 7-Zip File Manager, giờ hỗ trợ cả đa chọn
 * (tick checkbox nhiều dòng rồi kéo 1 trong số đó — xem `scheduleDragStart`). Cố tình KHÔNG
 * dùng `draggable`/API `dragstart` chuẩn của trình duyệt — đó là drag-and-drop HTML5, chạy
 * trong webview, không mang được đường dẫn file thật ra ngoài hệ điều hành (đúng giới hạn đã
 * ghi ở `onFilesDropped`/kéo VÀO, nhưng ở chiều ngược lại không có giải pháp lách qua được).
 * Thay vào đó bắt sự kiện `mousedown` rồi tự gọi `startDrag()` của `tauri-plugin-drag` — plugin
 * này chủ động khởi tạo phiên kéo-thả ở tầng hệ điều hành (`DoDragDrop` trên Windows), độc
 * lập với cơ chế kéo-thả của trình duyệt, khớp đúng cách app gốc (SpaceDrive) dùng plugin này.
 * Vì các entry chưa tồn tại thật trên đĩa (còn nén trong archive), bước đầu tiên luôn phải
 * giải nén đúng các entry được yêu cầu ra file tạm (`extract_entries_for_drag`, giải nén
 * archive đúng 1 lần dù kéo bao nhiêu entry) rồi mới kéo các file tạm đó đi. */
async function onEntriesDragStart(entryNames: string[]) {
  if (!currentViewedArchive || appBusy || entryNames.length === 0) return;
  try {
    const tempPaths = await invoke<string[]>("extract_entries_for_drag", {
      archive: currentViewedArchive.path,
      entryNames,
      password: currentViewedArchive.password,
    });
    await startDrag({ item: tempPaths, icon: DRAG_ICON_DATA_URL });
  } catch (err) {
    setStatus(t("error.dragOut", { detail: friendlyError(err) }), "error");
  }
}

/** ID hẹn giờ đang chờ để bắt đầu kéo-thả 1 dòng, xem `scheduleDragStart`. */
let pendingDragTimer: ReturnType<typeof setTimeout> | null = null;

/** `mousedown` một mình không phân biệt được "định kéo đi" với cú đầu tiên của 1 double-click
 * (định mở archive lồng bên trong, xem `openNestedArchive`) — trình duyệt bắn `mousedown` ở
 * CẢ HAI cú click trước khi `dblclick` mới bắn thêm, nên gọi `onEntriesDragStart` thẳng từ
 * `mousedown` sẽ khởi động phiên kéo-thả cấp hệ điều hành 2 lần chồng lên nhau mỗi khi double
 * -click (rủi ro thật, không phải lý thuyết — API kéo-thả gốc như `DoDragDrop` không thiết kế
 * để gọi chồng trên cùng 1 cửa sổ). Trễ việc kéo-thả thật đúng bằng ngưỡng double-click chuẩn
 * (~200ms); nếu `dblclick` bắn ra trong lúc đang chờ, `cancelPendingDragStart()` huỷ hẹn giờ
 * này trước khi nó kịp chạy.
 *
 * Đa chọn: nếu dòng đang kéo (`entryName`) nằm trong tập đã tick VÀ có từ 2 dòng được tick
 * trở lên, kéo cả tập đã chọn; ngược lại chỉ kéo đúng 1 dòng vừa bấm — khớp quy ước file
 * manager chuẩn (kéo 1 dòng chưa được chọn thì chỉ kéo riêng dòng đó, không kéo theo lựa
 * chọn cũ không liên quan). */
function scheduleDragStart(entryName: string, event: MouseEvent) {
  // Bỏ qua nếu người dùng đang bấm nút Đổi tên/Xoá/checkbox bên trong dòng (mousedown nổi
  // bọt lên từ phần tử con) — không phải đang cố kéo dòng đi.
  if (event.target instanceof HTMLButtonElement) return;
  if (event.target instanceof HTMLInputElement) return;
  cancelPendingDragStart();
  pendingDragTimer = setTimeout(() => {
    pendingDragTimer = null;
    const names =
      selectedEntryNames.has(entryName) && selectedEntryNames.size > 1
        ? Array.from(selectedEntryNames)
        : [entryName];
    void onEntriesDragStart(names);
  }, 200);
}

function cancelPendingDragStart() {
  if (pendingDragTimer !== null) {
    clearTimeout(pendingDragTimer);
    pendingDragTimer = null;
  }
}

/** Mở 1 entry mà chính nó cũng là 1 file nén nhận diện được (vd 1 `.zip` bên trong 1 `.zip`
 * khác) mà không cần giải nén ra rồi mở lại thủ công — nửa còn thiếu của "nested-archive
 * browsing" so với 7-Zip File Manager. Cùng kỹ thuật với kéo-thả (`extract_entry_for_drag`
 * giải nén đúng 1 entry ra file tạm), chỉ khác bước tiếp theo là gọi `viewArchive` trên file
 * tạm đó thay vì `startDrag`. Đẩy archive đang xem vào `archiveViewStack` để còn quay lại
 * đúng chỗ — xem `goBackToOuterArchive`. Giới hạn có chủ đích (giữ tối giản, NFR-11): mật
 * khẩu của archive lồng bên trong dùng chung ô mật khẩu với archive ngoài, không hỏi riêng —
 * giống cách `onConvert` đã đơn giản hoá tương tự cho nguồn/đích. */
async function openNestedArchive(entryName: string) {
  if (!currentViewedArchive || appBusy) return;
  const outer = currentViewedArchive;
  const outerFolder = currentViewFolder;
  try {
    const tempPath = await invoke<string>("extract_entry_for_drag", {
      archive: outer.path,
      entryName,
      password: outer.password,
    });
    archiveViewStack.push({ path: outer.path, password: outer.password, folder: outerFolder });
    await viewArchive(tempPath);
  } catch (err) {
    archiveViewStack.pop();
    setStatus(t("error.view", { detail: friendlyError(err) }), "error");
  }
}

/** Quay lại archive ngoài sau khi đã "đi vào" xem 1 archive lồng bên trong (xem
 * `openNestedArchive`) — nạp lại đúng archive ngoài rồi khôi phục lại đúng vị trí thư mục
 * đang xem dở, thay vì luôn đưa người dùng về thư mục gốc của nó. */
async function goBackToOuterArchive() {
  const outer = archiveViewStack.pop();
  if (!outer) return;
  await viewArchive(outer.path);
  currentViewFolder = outer.folder;
  renderCurrentFolder();
}

/** Kéo-thả file/thư mục từ Explorer vào cửa sổ ứng dụng. Quy tắc cố định, không hỏi lại
 * người dùng (tránh mơ hồ — xem section-hint của phần nâng cao): thả đúng 1 file nén nhận
 * diện được thì Xem trước nội dung (giống quy ước của 7-Zip/WinRAR khi thả 1 archive vào cửa
 * sổ chính); mọi trường hợp khác (nhiều mục, hoặc 1 thư mục/file thường) thì Nén — hành động
 * chính của app, khớp icon đầu tiên trong lưới thao tác. */
async function onFilesDropped(paths: string[]) {
  if (paths.length === 0 || appBusy) return;
  if (paths.length === 1 && isArchivePath(paths[0])) {
    archiveViewStack = []; // đang mở 1 archive gốc mới qua kéo-thả, không phải điều hướng lồng nhau
    await viewArchive(paths[0]);
  } else {
    await onCompress(paths);
  }
}

/** FR-18/19. Nạp lại danh sách entry của archive đang xem, dùng sau khi thêm/xoá. */
async function refreshCurrentArchiveEntries() {
  if (!currentArchivePath) return;
  const entries = await invoke<EntryDto[]>("list_archive_entries", {
    archive: currentArchivePath,
    password: currentPassword(),
  });
  renderEntries(entries);
}

async function onAddToArchive() {
  if (!currentArchivePath) return;
  const files = await open({ title: t("dialog.selectFilesToCompress"), multiple: true, directory: false });
  if (!files) return;
  const sources = Array.isArray(files) ? files : [files];

  await withProgress(t("status.addingEntries"), async () => {
    try {
      await invoke("add_archive_entries", {
        archive: currentArchivePath,
        sources,
        password: currentPassword(),
        level: currentLevel(),
      });
      await refreshCurrentArchiveEntries();
      setStatus(t("status.entriesAdded", { count: sources.length }), "success");
    } catch (err) {
      setStatus(t("error.addEntries", { detail: friendlyError(err) }), "error");
    }
  });
}

/** "Thêm vào archive" từ menu chuột phải (`crates/shell-menu`): khác `onAddToArchive` (thêm
 * NGUỒN vào archive đang XEM), ở đây có sẵn 1 NGUỒN cụ thể (`sourcePath`, file người dùng vừa
 * bấm chuột phải) và cần hỏi archive ĐÍCH để thêm vào — chiều ngược lại. */
async function onAddThisFileToArchive(sourcePath: string) {
  const target = await open({
    title: t("dialog.selectTargetArchiveForAdd"),
    multiple: false,
    directory: false,
    filters: [{ name: t("filter.zip"), extensions: ["zip"] }],
  });
  if (!target || Array.isArray(target)) return;

  await withProgress(t("status.addingEntries"), async () => {
    try {
      await invoke("add_archive_entries", {
        archive: target,
        sources: [sourcePath],
        password: currentPassword(),
        level: currentLevel(),
      });
      setStatus(t("status.entriesAdded", { count: 1 }), "success");
      if (currentArchivePath === target) {
        await refreshCurrentArchiveEntries();
      }
    } catch (err) {
      setStatus(t("error.addEntries", { detail: friendlyError(err) }), "error");
    }
  });
}

/** Áp dụng ý định khởi chạy (nếu app được mở từ mục menu chuột phải "Xem nội dung"/"Thêm vào
 * archive" của `crates/shell-menu`, thay vì mở bình thường) — xem
 * `apps-tauri/src/lib.rs::get_launch_intent`. Lỗi ở đây chỉ log ra console, không chặn app
 * khởi động bình thường — đây là tiện ích khởi động, không phải yêu cầu bắt buộc. */
async function applyLaunchIntent() {
  try {
    const intent = await invoke<{ action: string; path: string } | null>("get_launch_intent");
    if (!intent) return;
    if (intent.action === "view") {
      archiveViewStack = [];
      await viewArchive(intent.path);
    } else if (intent.action === "add-to") {
      await onAddThisFileToArchive(intent.path);
    }
  } catch (err) {
    console.error("Không áp dụng được ý định khởi chạy:", err);
  }
}

async function onRemoveEntry(name: string) {
  if (!currentArchivePath) return;
  if (!window.confirm(t("manage.removeConfirm", { name }))) return;

  await withProgress(t("status.removingEntry"), async () => {
    try {
      await invoke("remove_archive_entries", { archive: currentArchivePath, names: [name] });
      await refreshCurrentArchiveEntries();
      setStatus(t("status.entryRemoved", { name }), "success");
    } catch (err) {
      setStatus(t("error.removeEntry", { detail: friendlyError(err) }), "error");
    }
  });
}

/** FR-18/19. Đổi tên 1 entry — trước đây chỉ có ở CLI (`vietzip rename`), giờ thêm vào UI.
 * Dùng `window.prompt` thay vì sửa trực tiếp trong bảng để giữ đơn giản (đã dùng
 * `window.confirm` thành công cho Xoá, cùng họ hộp thoại chặn của WebView2). */
async function onRenameEntry(oldName: string) {
  if (!currentArchivePath) return;
  const newName = window.prompt(t("manage.renamePrompt", { name: oldName }), oldName);
  if (!newName || newName === oldName) return;

  await withProgress(t("status.renamingEntry"), async () => {
    try {
      await invoke("rename_archive_entry", { archive: currentArchivePath, oldName, newName });
      await refreshCurrentArchiveEntries();
      setStatus(t("status.entryRenamed", { oldName, newName }), "success");
    } catch (err) {
      setStatus(t("error.renameEntry", { detail: friendlyError(err) }), "error");
    }
  });
}

/** FR-04. Chọn 1 file bất kỳ để chia nhỏ theo kích thước (MB) nhập ở ô cạnh nút. */
async function onSplit() {
  const file = await open({ title: t("dialog.selectFileToSplit"), multiple: false, directory: false });
  if (!file || Array.isArray(file)) return;

  const sizeMb = Number(splitSizeEl?.value ?? "100") || 100;

  await withProgress(t("status.splitting"), async () => {
    try {
      const parts = await invoke<string[]>("split_file", { file, sizeMb });
      setStatus(t("status.split", { count: parts.length, path: parts[0] ?? "" }), "success");
    } catch (err) {
      setStatus(t("error.split", { detail: friendlyError(err) }), "error");
    }
  });
}

/** FR-04. Chọn phần đầu tiên (.001) rồi chọn nơi lưu file đã ghép lại. */
async function onJoin() {
  const firstPart = await open({
    title: t("dialog.selectFirstPartToJoin"),
    multiple: false,
    directory: false,
    filters: [{ name: t("filter.splitParts"), extensions: ["001"] }],
  });
  if (!firstPart || Array.isArray(firstPart)) return;

  const dest = await save({ title: t("dialog.saveJoinedAs") });
  if (!dest) return;

  await withProgress(t("status.joining"), async () => {
    try {
      await invoke("join_parts", { firstPart, dest });
      setStatus(t("status.joined", { path: dest }), "success");
    } catch (err) {
      setStatus(t("error.join", { detail: friendlyError(err) }), "error");
    }
  });
}

/** FR-07. Chọn 1 file .zip/.7z đã nén sẵn rồi tạo file .exe tự giải nén từ đó. */
async function onSfx() {
  const archive = await open({
    title: t("dialog.selectArchiveForSfx"),
    multiple: false,
    directory: false,
    filters: [
      { name: t("filter.zip"), extensions: ["zip"] },
      { name: t("filter.sevenz"), extensions: ["7z"] },
    ],
  });
  if (!archive || Array.isArray(archive)) return;

  const output = await save({
    title: t("dialog.saveSfxAs"),
    filters: [{ name: "EXE", extensions: ["exe"] }],
  });
  if (!output) return;

  const runAfterInput = document.querySelector<HTMLInputElement>("#sfx-run-after");
  const runAfterExtract = runAfterInput?.value.trim() || undefined;

  await withProgress(t("status.creatingSfx"), async () => {
    try {
      await invoke("create_sfx", { archive, output, runAfterExtract });
      setStatus(t("status.sfxCreated", { path: output }), "success");
    } catch (err) {
      setStatus(t("error.sfx", { detail: friendlyError(err) }), "error");
    }
  });
}

interface BenchmarkResultDto {
  format: string;
  compress_mb_per_sec: number;
  decompress_mb_per_sec: number;
  compression_ratio_percent: number;
}

/** Tương đương "Tools > Benchmark" của 7-Zip — đo tốc độ nén/giải nén thật trên máy đang
 * chạy app (64MB dữ liệu tổng hợp, mức nén hiện đang chọn). */
async function onBenchmark() {
  await withProgress(t("status.benchmarking"), async () => {
    try {
      const results = await invoke<BenchmarkResultDto[]>("run_benchmark", {
        sizeMb: 64,
        level: currentLevel(),
      });
      renderBenchmarkResults(results);
      setStatus(t("status.benchmarkDone"), "success");
    } catch (err) {
      setStatus(t("error.benchmark", { detail: friendlyError(err) }), "error");
    }
  });
}

function renderBenchmarkResults(results: BenchmarkResultDto[]) {
  if (!benchmarkBodyEl || !benchmarkTableEl) return;
  benchmarkBodyEl.innerHTML = "";
  for (const r of results) {
    const row = document.createElement("tr");
    const format = document.createElement("td");
    format.textContent = r.format;
    const compress = document.createElement("td");
    compress.textContent = `${r.compress_mb_per_sec.toFixed(1)} MB/s`;
    const decompress = document.createElement("td");
    decompress.textContent = `${r.decompress_mb_per_sec.toFixed(1)} MB/s`;
    const ratio = document.createElement("td");
    ratio.textContent = `${r.compression_ratio_percent.toFixed(1)}%`;
    row.append(format, compress, decompress, ratio);
    benchmarkBodyEl.appendChild(row);
  }
  benchmarkTableEl.classList.remove("hidden");
}

interface FileChecksumDto {
  crc32_hex: string;
  sha256_hex: string;
  size_bytes: number;
}

/** Công cụ CRC/hash độc lập, không liên quan tới thao tác archive nào — tương đương mục
 * "CRC SHA" trong menu chuột phải của 7-Zip. */
async function onChecksum() {
  const file = await open({ title: t("dialog.selectFileForChecksum"), multiple: false, directory: false });
  if (!file || Array.isArray(file)) return;

  await withProgress(t("status.checksumming"), async () => {
    try {
      const result = await invoke<FileChecksumDto>("checksum_file", { file });
      if (checksumResultEl) {
        checksumResultEl.textContent = t("checksum.result", {
          name: baseName(file),
          size: result.size_bytes,
          crc32: result.crc32_hex,
          sha256: result.sha256_hex,
        });
        checksumResultEl.classList.remove("hidden");
      }
      setStatus(t("status.checksumDone"), "success");
    } catch (err) {
      setStatus(t("error.checksum", { detail: friendlyError(err) }), "error");
    }
  });
}

/** Chuyển đổi 1 file nén sang định dạng khác — giải nén rồi nén lại (7-Zip cũng không có
 * "chuyển đổi trực tiếp" thật). Mật khẩu nguồn dùng ô mật khẩu hiện tại; mật khẩu đích được
 * hỏi riêng vì có thể khác (vd bỏ mật khẩu, hoặc đặt mật khẩu mới). */
async function onConvert() {
  const source = await open({
    title: t("dialog.selectArchiveToConvert"),
    multiple: false,
    directory: false,
    filters: archiveFilters(),
  });
  if (!source || Array.isArray(source)) return;

  const dest = await save({
    title: t("dialog.saveConvertedAs"),
    filters: [
      { name: t("filter.zip"), extensions: ["zip"] },
      { name: t("filter.sevenz"), extensions: ["7z"] },
    ],
  });
  if (!dest) return;

  await withProgress(t("status.converting"), async () => {
    try {
      await invoke("convert_archive", {
        source,
        dest,
        sourcePassword: currentPassword(),
        destPassword: currentPassword(),
        level: currentLevel(),
      });
      setStatus(t("status.converted", { path: dest }), "success");
    } catch (err) {
      setStatus(t("error.convert", { detail: friendlyError(err) }), "error");
    }
  });
}

interface RepairReportDto {
  recovered: string[];
  unrecoverable: string[];
}

/** FR-17 — Sửa file nén bị lỗi. Mức hỗ trợ tuỳ định dạng (xem `vietzip_core::repair`): ZIP
 * phục hồi dữ liệu thật, .7z chỉ phát hiện lỗi, định dạng khác báo lỗi rõ ràng thay vì âm
 * thầm không làm gì. Dùng ô mật khẩu hiện tại, giống cách `onConvert` tái dùng ô đó — chỉ để
 * thử mở archive theo đường bình thường trước khi quét thô (xem doc `repair.rs`). */
async function onRepair() {
  const archive = await open({
    title: t("dialog.selectArchiveToRepair"),
    multiple: false,
    directory: false,
    filters: archiveFilters(),
  });
  if (!archive || Array.isArray(archive)) return;

  const dest = await save({ title: t("dialog.saveRepairedAs") });
  if (!dest) return;

  const resultEl = document.querySelector<HTMLElement>("#repair-result");

  await withProgress(t("status.repairing"), async () => {
    try {
      const report = await invoke<RepairReportDto>("repair_archive", {
        archive,
        dest,
        password: currentPassword(),
      });
      if (resultEl) {
        const unrecoverableSuffix =
          report.unrecoverable.length > 0
            ? t("repair.unrecoverableSuffix", { count: report.unrecoverable.length })
            : "";
        resultEl.textContent = t("repair.result", {
          recovered: report.recovered.length,
          unrecoverableSuffix,
        });
        resultEl.classList.remove("hidden");
      }
      setStatus(t("status.repaired", { path: dest }), "success");
    } catch (err) {
      setStatus(t("error.repair", { detail: friendlyError(err) }), "error");
    }
  });
}

interface AboutInfo {
  name: string;
  version: string;
  license: string;
  third_party_licenses: string;
}

/** Màn hình About — trước đây `LICENSES.md` được đóng gói vào MSI (`THIRD-PARTY-LICENSES.md`)
 * nhưng không có nơi nào trong UI hiển thị nó; giờ đọc qua `get_about_info` (Rust nhúng nội
 * dung bằng `include_str!` lúc build, không cần đọc file lúc chạy). */
async function onAbout() {
  const dialog = document.querySelector<HTMLDialogElement>("#about-dialog");
  if (!dialog) return;
  try {
    const info = await invoke<AboutInfo>("get_about_info");
    const nameEl = document.querySelector("#about-name");
    const versionEl = document.querySelector("#about-version");
    const licenseEl = document.querySelector("#about-license");
    const thirdPartyEl = document.querySelector("#about-third-party");
    if (nameEl) nameEl.textContent = info.name;
    if (versionEl) versionEl.textContent = t("about.version", { version: info.version });
    if (licenseEl) licenseEl.textContent = t("about.license", { license: info.license });
    if (thirdPartyEl) thirdPartyEl.textContent = info.third_party_licenses;
    dialog.showModal();
  } catch (err) {
    setStatus(t("error.about", { detail: friendlyError(err) }), "error");
  }
}

function updateLangButtons() {
  const lang = getLang();
  document.querySelectorAll<HTMLButtonElement>(".lang-btn").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.lang === lang);
  });
}

window.addEventListener("DOMContentLoaded", () => {
  statusEl = document.querySelector("#status");
  statusTextEl = document.querySelector("#status-text");
  entriesBodyEl = document.querySelector("#entries-body");
  entriesTableEl = document.querySelector("#entries-table");
  entriesEmptyEl = document.querySelector("#entries-empty");
  resultsViewingEl = document.querySelector("#results-viewing");
  breadcrumbEl = document.querySelector("#results-breadcrumb");
  addToArchiveBtnEl = document.querySelector("#btn-add-to-archive");
  passwordEl = document.querySelector("#password");
  levelEl = document.querySelector("#level");
  splitSizeEl = document.querySelector("#split-size-mb");
  checksumResultEl = document.querySelector("#checksum-result");
  benchmarkTableEl = document.querySelector("#benchmark-results");
  benchmarkBodyEl = document.querySelector("#benchmark-results-body");
  dropOverlayEl = document.querySelector("#drop-overlay");

  applyStaticTranslations();
  document.querySelector<HTMLButtonElement>("#lang-vi")!.dataset.lang = "vi";
  document.querySelector<HTMLButtonElement>("#lang-en")!.dataset.lang = "en";
  updateLangButtons();
  renderEntries([]);

  document.querySelectorAll<HTMLButtonElement>(".lang-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      setLang(btn.dataset.lang as Lang);
      updateLangButtons();
    });
  });

  document.querySelector("#btn-compress")?.addEventListener("click", async () => {
    const files = await open({ title: t("dialog.selectFilesToCompress"), multiple: true, directory: false });
    if (files) await onCompress(Array.isArray(files) ? files : [files]);
  });

  document.querySelector("#btn-compress-folder")?.addEventListener("click", async () => {
    const folder = await open({ title: t("dialog.selectFolderToCompress"), multiple: false, directory: true });
    if (folder && !Array.isArray(folder)) await onCompress([folder]);
  });

  document.querySelector("#btn-extract")?.addEventListener("click", onExtract);
  document.querySelector("#btn-test")?.addEventListener("click", onTest);
  document.querySelector("#btn-view")?.addEventListener("click", onView);
  document.querySelector("#btn-split")?.addEventListener("click", onSplit);
  document.querySelector("#btn-join")?.addEventListener("click", onJoin);
  document.querySelector("#btn-sfx")?.addEventListener("click", onSfx);
  document.querySelector("#btn-benchmark")?.addEventListener("click", onBenchmark);
  document.querySelector("#btn-checksum")?.addEventListener("click", onChecksum);
  document.querySelector("#btn-convert")?.addEventListener("click", onConvert);
  document.querySelector("#btn-repair")?.addEventListener("click", onRepair);
  document.querySelector("#btn-add-to-archive")?.addEventListener("click", onAddToArchive);
  document.querySelector("#btn-about")?.addEventListener("click", onAbout);
  document.querySelector("#btn-about-close")?.addEventListener("click", () => {
    document.querySelector<HTMLDialogElement>("#about-dialog")?.close();
  });

  // Kéo-thả từ Explorer/Finder/Nautilus vào cửa sổ — Tauri chặn ở tầng webview và trả về
  // đường dẫn thật trên đĩa (khác Drag-and-Drop API của trình duyệt, vốn không cho path thật).
  void getCurrentWebview().onDragDropEvent((event) => {
    if (event.payload.type === "enter") {
      dropOverlayEl?.classList.remove("hidden");
    } else if (event.payload.type === "drop") {
      dropOverlayEl?.classList.add("hidden");
      void onFilesDropped(event.payload.paths);
    } else if (event.payload.type === "leave") {
      dropOverlayEl?.classList.add("hidden");
    }
  });

  void applyLaunchIntent();
});
