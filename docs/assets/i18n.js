(function () {
  "use strict";

  var STORAGE_KEY = "vietzip-site-lang";

  var DICT = {
    vi: {
      nav_home: "Trang chủ",
      nav_github: "GitHub",
      nav_about: "Giới thiệu",
      nav_contact: "Liên hệ",
      footer_text: "Vietzip — mã nguồn mở theo giấy phép MIT.",

      home_title: "Vietzip — Công cụ nén/giải nén miễn phí, mã nguồn mở",
      home_hero_tagline: "Công cụ nén/giải nén miễn phí, mã nguồn mở — thay thế cho WinRAR/7-Zip. Hỗ trợ hàng chục định dạng, mật khẩu AES-256, và tích hợp trực tiếp vào Explorer/Nautilus.",
      home_cta_download: "⬇ Tải xuống",
      home_cta_source: "Xem mã nguồn",
      home_platforms_heading: "Nền tảng hỗ trợ",
      home_th_platform: "Nền tảng",
      home_th_status: "Trạng thái",
      home_platform_win64: "Windows (64-bit)",
      home_status_win64: "✅ MSI + NSIS, đã build và kiểm thử",
      home_platform_win32: "Windows (32-bit)",
      home_status_win32: "✅ MSI + NSIS, đã build và kiểm thử",
      home_platform_linux: "Linux (Ubuntu và tương đương)",
      home_status_linux: "✅ .deb / .rpm / AppImage, đã build và kiểm thử",
      home_platform_macos: "macOS",
      home_status_macos: "✅ DMG, build tự động qua GitHub Actions",
      home_platform_android: "Android",
      home_status_android: "🚧 Đang phát triển",
      home_features_heading: "Tính năng nổi bật",
      home_feat1_h: "Nhiều định dạng",
      home_feat1_p: "Nén .zip/.7z; giải nén thêm .rar, .tar và các biến thể, .cab, .cpio, .deb, .rpm, .lzh, .ext2-4, .arj, NSIS, .chm, .udf.",
      home_feat2_h: "Mật khẩu AES-256",
      home_feat2_p: "Mã hóa AES-256 cho cả .zip lẫn .7z, kèm ẩn luôn danh sách tên file khi nén .7z có mật khẩu.",
      home_feat3_h: "Tích hợp Explorer/Nautilus",
      home_feat3_p: "Menu chuột phải \"Nén thành .zip\", \"Giải nén tại đây\", cùng menu ngữ cảnh đầy đủ trên Windows.",
      home_feat4_h: "File tự giải nén (SFX)",
      home_feat4_p: "Tạo file .exe tự giải nén, có thể tự chạy 1 chương trình khác ngay sau khi giải nén xong.",
      home_feat5_h: "Sửa file nén bị lỗi",
      home_feat5_p: "Phục hồi dữ liệu từ file .zip bị hỏng central directory, cùng công cụ kiểm tra toàn vẹn/checksum.",
      home_feat6_h: "Song ngữ Việt/Anh",
      home_feat6_p: "Giao diện tự nhận diện ngôn ngữ hệ thống, chuyển đổi thủ công bất kỳ lúc nào.",

      github_title: "Vietzip trên GitHub",
      github_h1: "Vietzip trên GitHub",
      github_tagline: "Toàn bộ mã nguồn, lịch sử phát triển, và các bản phát hành đều công khai trên GitHub.",
      github_cta: "⭐ Xem repo trên GitHub",
      github_links_heading: "Đường dẫn nhanh",
      github_card1_h: "Mã nguồn",
      github_card1_p_prefix: "— Rust (core engine + CLI) và Tauri (ứng dụng desktop).",
      github_card2_h: "Tải bản cài đặt",
      github_card2_p_prefix: "— nơi đăng các file cài đặt chính thức cho từng nền tảng.",
      github_card3_h: "Báo lỗi / góp ý",
      github_card3_p_prefix: "— báo lỗi, đề xuất tính năng, hoặc đặt câu hỏi.",
      github_card4_h: "Build tự động",
      github_card4_p_prefix: "— mỗi bản build macOS được biên dịch tự động trên phần cứng Apple thật.",
      github_build_heading: "Tự build từ mã nguồn",
      github_build_p_prefix: "Hướng dẫn build và sử dụng chi tiết (song ngữ Việt/Anh) nằm ngay trong",
      github_build_p_suffix: "của repo. Tóm tắt:",
      github_readme_link_text: "README của repo",

      about_title: "Giới thiệu Vietzip",
      about_h1: "Giới thiệu",
      about_tagline: "Vietzip là công cụ nén/giải nén miễn phí, mã nguồn mở, hướng tới người dùng Việt Nam — thay thế cho các phần mềm thương mại như WinRAR.",
      about_formats_heading: "Định dạng hỗ trợ",
      about_formats_compress: "Nén:",
      about_formats_compress_text: ".zip, .7z (cả 2 hỗ trợ mật khẩu AES-256), cộng nén file đơn .gz/.bz2/.zst/.xz.",
      about_formats_extract: "Giải nén:",
      about_formats_extract_text: "tất cả định dạng trên, cộng thêm .rar (chỉ đọc), .tar và các biến thể nén, .cab, .cpio, .deb, .rpm, .lzh/.lha, .ext2/.ext3/.ext4, .arj, installer NSIS .exe, .chm, và .udf.",
      about_tech_heading: "Công nghệ",
      about_tech1_h: "Core engine",
      about_tech1_p: "Viết bằng Rust — an toàn bộ nhớ, hiệu năng cao, dùng chung cho CLI và ứng dụng desktop.",
      about_tech2_h: "Ứng dụng desktop",
      about_tech2_p_prefix: "Xây dựng trên",
      about_tech2_p_suffix: "— nhẹ, dùng WebView có sẵn của hệ điều hành thay vì đóng gói cả trình duyệt.",
      about_tech3_h: "Đa nền tảng",
      about_tech3_p: "Cùng 1 core engine chạy trên Windows, Linux, macOS — và Android đang trong quá trình phát triển.",
      about_license_heading: "Giấy phép",
      about_license_p1_prefix: "Mã nguồn Vietzip cấp phép theo",
      about_license_p1_suffix: "— miễn phí sử dụng, sửa đổi, phân phối lại.",
      about_license_p2: "Một thành phần duy nhất mang điều kiện riêng: phần đọc file .rar dùng mã nguồn UnRAR của RARLAB (chỉ giải nén, không bao giờ tạo file .rar), theo giấy phép riêng của RARLAB — được phép dùng miễn phí cho mục đích giải nén. Chi tiết đầy đủ trong",
      about_license_p2_suffix: "trên repo.",

      contact_title: "Liên hệ — Vietzip",
      contact_h1: "Liên hệ",
      contact_tagline: "Cách nhanh nhất để báo lỗi, đề xuất tính năng, hoặc đặt câu hỏi là qua GitHub Issues — mọi trao đổi đều công khai, giúp người dùng khác gặp vấn đề tương tự cũng tìm thấy câu trả lời.",
      contact_cta_new: "✉ Tạo Issue mới",
      contact_cta_view: "Xem Issues hiện có",
      contact_before_heading: "Trước khi tạo Issue mới",
      contact_before_p: "Giúp xử lý nhanh hơn bằng cách kèm theo:",
      contact_li1: "Hệ điều hành và phiên bản Vietzip đang dùng.",
      contact_li2: "Các bước tái hiện lỗi, nếu là báo lỗi.",
      contact_li3_prefix: "Kiểm tra nhanh",
      contact_li3_suffix: "xem vấn đề đã có ai báo chưa.",
      contact_issues_link_text: "danh sách Issues hiện có"
    },
    en: {
      nav_home: "Home",
      nav_github: "GitHub",
      nav_about: "About",
      nav_contact: "Contact",
      footer_text: "Vietzip — open source, MIT licensed.",

      home_title: "Vietzip — Free, open-source archive tool",
      home_hero_tagline: "A free, open-source archiver — an alternative to WinRAR/7-Zip. Supports dozens of formats, AES-256 passwords, and direct Explorer/Nautilus integration.",
      home_cta_download: "⬇ Download",
      home_cta_source: "View source",
      home_platforms_heading: "Supported platforms",
      home_th_platform: "Platform",
      home_th_status: "Status",
      home_platform_win64: "Windows (64-bit)",
      home_status_win64: "✅ MSI + NSIS, built and tested",
      home_platform_win32: "Windows (32-bit)",
      home_status_win32: "✅ MSI + NSIS, built and tested",
      home_platform_linux: "Linux (Ubuntu and equivalents)",
      home_status_linux: "✅ .deb / .rpm / AppImage, built and tested",
      home_platform_macos: "macOS",
      home_status_macos: "✅ DMG, built automatically via GitHub Actions",
      home_platform_android: "Android",
      home_status_android: "🚧 In development",
      home_features_heading: "Key features",
      home_feat1_h: "Many formats",
      home_feat1_p: "Compress .zip/.7z; extract .rar, .tar and its variants, .cab, .cpio, .deb, .rpm, .lzh, .ext2-4, .arj, NSIS, .chm, .udf, and more.",
      home_feat2_h: "AES-256 passwords",
      home_feat2_p: "AES-256 encryption for both .zip and .7z, including hiding the file list itself when a .7z is password-protected.",
      home_feat3_h: "Explorer/Nautilus integration",
      home_feat3_p: "Right-click \"Compress to .zip\", \"Extract Here\", plus a full context menu on Windows.",
      home_feat4_h: "Self-extracting files (SFX)",
      home_feat4_p: "Create a self-extracting .exe that can launch another program right after extraction.",
      home_feat5_h: "Repair damaged archives",
      home_feat5_p: "Recover data from a .zip with a damaged central directory, plus integrity-check/checksum tools.",
      home_feat6_h: "Vietnamese/English UI",
      home_feat6_p: "Auto-detects your system language, with a manual toggle at any time.",

      github_title: "Vietzip on GitHub",
      github_h1: "Vietzip on GitHub",
      github_tagline: "All source code, development history, and releases are public on GitHub.",
      github_cta: "⭐ View repo on GitHub",
      github_links_heading: "Quick links",
      github_card1_h: "Source code",
      github_card1_p_prefix: "— Rust (core engine + CLI) and Tauri (desktop app).",
      github_card2_h: "Download installers",
      github_card2_p_prefix: "— where official installers for each platform are published.",
      github_card3_h: "Report a bug / feedback",
      github_card3_p_prefix: "— report bugs, request features, or ask questions.",
      github_card4_h: "Automated builds",
      github_card4_p_prefix: "— every macOS build is compiled automatically on real Apple hardware.",
      github_build_heading: "Build it yourself",
      github_build_p_prefix: "Detailed build and usage instructions (bilingual VI/EN) live right in the repo's",
      github_build_p_suffix: ". Summary:",
      github_readme_link_text: "README",

      about_title: "About Vietzip",
      about_h1: "About",
      about_tagline: "Vietzip is a free, open-source archive tool aimed at Vietnamese users — an alternative to commercial software like WinRAR.",
      about_formats_heading: "Supported formats",
      about_formats_compress: "Compress:",
      about_formats_compress_text: ".zip, .7z (both support AES-256 passwords), plus single-file .gz/.bz2/.zst/.xz.",
      about_formats_extract: "Extract:",
      about_formats_extract_text: "everything above, plus .rar (read-only), .tar and its compressed variants, .cab, .cpio, .deb, .rpm, .lzh/.lha, .ext2/.ext3/.ext4, .arj, NSIS installer .exe, .chm, and .udf.",
      about_tech_heading: "Technology",
      about_tech1_h: "Core engine",
      about_tech1_p: "Written in Rust — memory-safe, high performance, shared by both the CLI and the desktop app.",
      about_tech2_h: "Desktop app",
      about_tech2_p_prefix: "Built on",
      about_tech2_p_suffix: "— lightweight, using the OS's built-in WebView instead of bundling a whole browser.",
      about_tech3_h: "Cross-platform",
      about_tech3_p: "The same core engine runs on Windows, Linux, and macOS — with Android still in development.",
      about_license_heading: "License",
      about_license_p1_prefix: "Vietzip's source code is licensed under the",
      about_license_p1_suffix: "— free to use, modify, and redistribute.",
      about_license_p2: "One component carries its own separate terms: the .rar reader uses RARLAB's own UnRAR source code (extraction only, never creates .rar files), under RARLAB's own license — free to use for extraction purposes. Full details in",
      about_license_p2_suffix: "in the repo.",

      contact_title: "Contact — Vietzip",
      contact_h1: "Contact",
      contact_tagline: "The fastest way to report a bug, suggest a feature, or ask a question is via GitHub Issues — every conversation is public, so other users hitting the same problem can find the answer too.",
      contact_cta_new: "✉ Open a new Issue",
      contact_cta_view: "View existing Issues",
      contact_before_heading: "Before opening a new Issue",
      contact_before_p: "Help us resolve it faster by including:",
      contact_li1: "Your operating system and the Vietzip version you're using.",
      contact_li2: "Steps to reproduce the problem, if it's a bug report.",
      contact_li3_prefix: "Quickly check the",
      contact_li3_suffix: "to see if it's already been reported.",
      contact_issues_link_text: "existing Issues list"
    }
  };

  function getLang() {
    return localStorage.getItem(STORAGE_KEY) || "vi";
  }

  function setLang(lang) {
    localStorage.setItem(STORAGE_KEY, lang);
    applyLang(lang);
  }

  function applyLang(lang) {
    var dict = DICT[lang] || DICT.vi;
    document.documentElement.lang = lang;

    document.querySelectorAll("[data-i18n]").forEach(function (el) {
      var key = el.getAttribute("data-i18n");
      if (dict[key] !== undefined) {
        el.textContent = dict[key];
      }
    });

    if (dict.home_title && document.body.dataset.page === "home") {
      document.title = dict.home_title;
    } else if (dict[document.body.dataset.page + "_title"]) {
      document.title = dict[document.body.dataset.page + "_title"];
    }

    document.querySelectorAll(".lang-btn").forEach(function (btn) {
      var isActive = btn.getAttribute("data-lang") === lang;
      btn.classList.toggle("active", isActive);
    });
  }

  window.vietzipI18n = { getLang: getLang, setLang: setLang, applyLang: applyLang };

  document.addEventListener("DOMContentLoaded", function () {
    applyLang(getLang());
    document.querySelectorAll(".lang-btn").forEach(function (btn) {
      btn.addEventListener("click", function () {
        setLang(btn.getAttribute("data-lang"));
      });
    });
  });
})();
