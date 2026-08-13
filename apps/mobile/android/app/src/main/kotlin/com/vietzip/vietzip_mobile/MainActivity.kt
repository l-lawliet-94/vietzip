package com.vietzip.vietzip_mobile

import android.content.Intent
import android.net.Uri
import android.provider.OpenableColumns
import androidx.annotation.NonNull
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import java.io.File
import java.io.FileOutputStream

/**
 * Bước D (ke-hoach-android.md): nhận file được mở qua "Mở bằng" (ACTION_VIEW) hoặc "Chia sẻ"
 * (ACTION_SEND) từ app khác. URI trả về là `content://` (Storage Access Framework), không
 * phải path hệ thống trực tiếp — copy byte ra 1 file thật trong `cacheDir` ngay ở tầng native
 * này, để phía Dart/Rust chỉ cần làm việc với path thường như trên Desktop, không phải sửa
 * lại core engine (nhận `&Path`) để đọc stream. Đơn giản hoá có chủ đích cho MVP — xem
 * CLAUDE.md mục Android để biết lý do.
 */
class MainActivity : FlutterActivity() {
    private val channelName = "com.vietzip/incoming_file"
    private var pendingPath: String? = null

    override fun configureFlutterEngine(@NonNull flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        handleIntent(intent)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, channelName).setMethodCallHandler { call, result ->
            if (call.method == "getInitialFile") {
                result.success(pendingPath)
                pendingPath = null
            } else {
                result.notImplemented()
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handleIntent(intent)
    }

    private fun handleIntent(intent: Intent?) {
        val uri: Uri? = when (intent?.action) {
            Intent.ACTION_VIEW -> intent.data
            Intent.ACTION_SEND -> {
                @Suppress("DEPRECATION")
                intent.getParcelableExtra(Intent.EXTRA_STREAM) as? Uri
            }
            else -> null
        }
        if (uri != null) {
            pendingPath = copyUriToCache(uri)
        }
    }

    private fun copyUriToCache(uri: Uri): String? {
        return try {
            val name = queryDisplayName(uri) ?: "opened_${System.currentTimeMillis()}"
            val outFile = File(cacheDir, name)
            contentResolver.openInputStream(uri)?.use { input ->
                FileOutputStream(outFile).use { output -> input.copyTo(output) }
            } ?: return null
            outFile.absolutePath
        } catch (e: Exception) {
            null
        }
    }

    private fun queryDisplayName(uri: Uri): String? {
        if (uri.scheme == "file") {
            return uri.path?.let { File(it).name }
        }
        return contentResolver.query(uri, null, null, null, null)?.use { cursor ->
            val idx = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            if (idx >= 0 && cursor.moveToFirst()) cursor.getString(idx) else null
        }
    }
}
