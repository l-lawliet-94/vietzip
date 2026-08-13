import 'dart:convert';
import 'dart:ui';

import 'package:flutter/services.dart' show rootBundle;
import 'package:shared_preferences/shared_preferences.dart';

/// FR-30/31 (mobile) — Bước E ke-hoach-android.md: song ngữ dùng chung tinh thần key-value
/// như `apps/desktop/src/i18n.ts`, đọc từ `assets/locales/{vi,en}.json` (bản sao tập con
/// key phù hợp cho mobile, xem ghi chú trong `pubspec.yaml`).
class I18n {
  I18n._();
  static final I18n instance = I18n._();

  static const _prefsKey = 'vietzip.lang';
  String _lang = 'en';
  Map<String, String> _dict = {};
  Map<String, String> _fallbackDict = {};

  String get lang => _lang;

  Future<void> load() async {
    final prefs = await SharedPreferences.getInstance();
    final saved = prefs.getString(_prefsKey);
    _lang = saved ?? _detectDefaultLang();
    _fallbackDict = await _loadDict('vi');
    _dict = _lang == 'vi' ? _fallbackDict : await _loadDict('en');
  }

  Future<void> setLang(String lang) async {
    if (lang != 'vi' && lang != 'en') return;
    _lang = lang;
    _dict = lang == 'vi' ? await _loadDict('vi') : await _loadDict('en');
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_prefsKey, lang);
  }

  /// FR-29 (mobile): tự nhận diện ngôn ngữ hệ điều hành, giống cách Desktop dùng
  /// `navigator.language` — ở đây dùng `PlatformDispatcher.locale`.
  String _detectDefaultLang() {
    final code = PlatformDispatcher.instance.locale.languageCode.toLowerCase();
    return code == 'vi' ? 'vi' : 'en';
  }

  Future<Map<String, String>> _loadDict(String lang) async {
    final raw = await rootBundle.loadString('assets/locales/$lang.json');
    final decoded = jsonDecode(raw) as Map<String, dynamic>;
    return decoded.map((key, value) => MapEntry(key, value as String));
  }

  String t(String key, [Map<String, Object?>? vars]) {
    var text = _dict[key] ?? _fallbackDict[key] ?? key;
    if (vars != null) {
      for (final entry in vars.entries) {
        text = text.replaceAll('{${entry.key}}', '${entry.value}');
      }
    }
    return text;
  }
}

/// Rút gọn để gọi `t('key')` trực tiếp trong widget, giống `t()` bên Desktop.
String t(String key, [Map<String, Object?>? vars]) => I18n.instance.t(key, vars);
