import 'package:flutter/services.dart';

/// Bước D (ke-hoach-android.md) — nhận đường dẫn file khi Vietzip được mở qua "Mở bằng"
/// (ACTION_VIEW) hoặc "Chia sẻ" (ACTION_SEND) từ app khác (File Manager, Zalo, Gmail...).
/// Phía Kotlin (`MainActivity.kt`) đã copy nội dung từ `content://` URI ra 1 file thật
/// trong cacheDir trước khi trả path về đây — core engine Rust vẫn chỉ nhận `&Path`, không
/// cần sửa lại để đọc stream (đơn giản hoá có chủ đích cho MVP, xem CLAUDE.md mục Android).
class NativeIntent {
  static const _channel = MethodChannel('com.vietzip/incoming_file');

  /// Trả về đường dẫn file vừa được mở qua Intent lúc khởi động app, hoặc `null` nếu app
  /// được mở bình thường (từ launcher). Chỉ có giá trị đúng 1 lần ngay sau khi khởi động.
  static Future<String?> getInitialFile() async {
    try {
      return await _channel.invokeMethod<String>('getInitialFile');
    } on PlatformException {
      return null;
    } on MissingPluginException {
      // Chưa cài kênh gốc (vd chạy trên nền tảng khác Android) — bỏ qua, không phải lỗi.
      return null;
    }
  }
}
