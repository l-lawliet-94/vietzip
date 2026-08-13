import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';
import 'package:vietzip_mobile/src/i18n.dart';
import 'package:vietzip_mobile/src/native_intent.dart';
import 'package:vietzip_mobile/src/rust/api/simple.dart';
import 'package:vietzip_mobile/src/rust/frb_generated.dart';

const _archiveExtensions = ['zip', '7z', 'rar', 'tar', 'gz', 'bz2', 'zst', 'tgz', 'tbz2'];

String _fileName(String path) => path.split(RegExp(r'[\\/]')).last;

String _fileStem(String path) {
  final name = _fileName(path);
  final dot = name.lastIndexOf('.');
  return dot > 0 ? name.substring(0, dot) : name;
}

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  await I18n.instance.load();
  final initialFile = await NativeIntent.getInitialFile();
  runApp(MyApp(initialFile: initialFile));
}

class MyApp extends StatelessWidget {
  final String? initialFile;
  const MyApp({super.key, this.initialFile});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Vietzip',
      theme: ThemeData(colorSchemeSeed: const Color(0xFF396CD8), useMaterial3: true),
      darkTheme: ThemeData(
        colorSchemeSeed: const Color(0xFF396CD8),
        brightness: Brightness.dark,
        useMaterial3: true,
      ),
      home: HomePage(initialFile: initialFile),
    );
  }
}

enum StatusKind { idle, loading, success, error }

class HomePage extends StatefulWidget {
  final String? initialFile;
  const HomePage({super.key, this.initialFile});

  @override
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  String _statusMessage = t('status.idle');
  StatusKind _statusKind = StatusKind.idle;
  bool _busy = false;
  String? _openedFile;

  final _passwordController = TextEditingController();
  String _level = 'normal';

  List<EntryInfo> _entries = [];
  String? _viewingName;

  @override
  void initState() {
    super.initState();
    _openedFile = widget.initialFile;
  }

  String? _password() => _passwordController.text.isEmpty ? null : _passwordController.text;

  void _setStatus(String message, StatusKind kind) {
    if (!mounted) return;
    setState(() {
      _statusMessage = message;
      _statusKind = kind;
    });
  }

  /// Giữ đúng tinh thần `withProgress()` bên Desktop (`main.ts`): hiện trạng thái "đang xử
  /// lý" + khoá nút ngay khi bắt đầu, luôn mở khoá khi xong dù thành công hay lỗi.
  Future<void> _withProgress(String loadingMessage, Future<void> Function() action) async {
    _setStatus(loadingMessage, StatusKind.loading);
    setState(() => _busy = true);
    try {
      await action();
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _onCompress() async {
    final result = await FilePicker.pickFiles(allowMultiple: true, withData: false);
    if (result == null || result.files.isEmpty) return;
    final sources = result.files.map((f) => f.path).whereType<String>().toList();
    if (sources.isEmpty) return;

    await _withProgress(t('status.compressing'), () async {
      try {
        final tempDir = await getTemporaryDirectory();
        final baseName = _fileStem(sources.first);
        final destPath = '${tempDir.path}/$baseName.zip';
        await compressFiles(sources: sources, dest: destPath, password: _password(), level: _level);

        final bytes = await File(destPath).readAsBytes();
        final savedPath = await FilePicker.saveFile(
          dialogTitle: t('export.title'),
          fileName: '$baseName.zip',
          bytes: bytes,
        );
        if (savedPath != null) {
          _setStatus(t('status.compressed', {'path': savedPath}), StatusKind.success);
        } else {
          _setStatus(t('export.skipped'), StatusKind.success);
        }
      } catch (e) {
        _setStatus(t('error.compress', {'detail': e.toString()}), StatusKind.error);
      }
    });
  }

  Future<String?> _pickArchivePath() async {
    final result = await FilePicker.pickFiles(type: FileType.custom, allowedExtensions: _archiveExtensions);
    return result?.files.single.path;
  }

  Future<void> _extractArchive(String archivePath) async {
    await _withProgress(t('status.extracting'), () async {
      try {
        final baseDir = await getExternalStorageDirectory() ?? await getApplicationDocumentsDirectory();
        final destDir = '${baseDir.path}/Vietzip/${_fileStem(archivePath)}';
        await extractArchive(archive: archivePath, destDir: destDir, password: _password());
        _setStatus(t('extract.savedTo', {'path': destDir}), StatusKind.success);
      } catch (e) {
        _setStatus(t('error.extract', {'detail': e.toString()}), StatusKind.error);
      }
    });
  }

  Future<void> _onExtract() async {
    final path = await _pickArchivePath();
    if (path == null) return;
    await _extractArchive(path);
  }

  Future<void> _testArchive(String archivePath) async {
    await _withProgress(t('status.testing'), () async {
      try {
        final ok = await testArchiveIntegrity(archive: archivePath, password: _password());
        _setStatus(ok ? t('status.testOk') : t('status.testFail'), ok ? StatusKind.success : StatusKind.error);
      } catch (e) {
        _setStatus(t('error.test', {'detail': e.toString()}), StatusKind.error);
      }
    });
  }

  Future<void> _onTest() async {
    final path = await _pickArchivePath();
    if (path == null) return;
    await _testArchive(path);
  }

  Future<void> _viewArchive(String archivePath) async {
    await _withProgress(t('status.loadingEntries'), () async {
      try {
        final entries = await listArchiveEntries(archive: archivePath, password: _password());
        setState(() {
          _entries = entries;
          _viewingName = _fileName(archivePath);
        });
        _setStatus(t('status.entriesCount', {'count': entries.length}), StatusKind.success);
      } catch (e) {
        _setStatus(t('error.view', {'detail': e.toString()}), StatusKind.error);
      }
    });
  }

  Future<void> _onView() async {
    final path = await _pickArchivePath();
    if (path == null) return;
    await _viewArchive(path);
  }

  Future<void> _setLang(String lang) async {
    await I18n.instance.setLang(lang);
    setState(() {
      _statusMessage = t('status.idle');
      _statusKind = StatusKind.idle;
    });
  }

  Color _statusColor(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    switch (_statusKind) {
      case StatusKind.loading:
        return scheme.primaryContainer;
      case StatusKind.success:
        return Colors.green.withValues(alpha: 0.15);
      case StatusKind.error:
        return scheme.errorContainer;
      case StatusKind.idle:
        return scheme.surfaceContainerHighest;
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(t('app.title')),
        actions: [
          TextButton(
            onPressed: I18n.instance.lang == 'vi' ? null : () => _setLang('vi'),
            child: const Text('VI'),
          ),
          TextButton(
            onPressed: I18n.instance.lang == 'en' ? null : () => _setLang('en'),
            child: const Text('EN'),
          ),
        ],
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(color: _statusColor(context), borderRadius: BorderRadius.circular(8)),
            child: Row(
              children: [
                if (_statusKind == StatusKind.loading)
                  const Padding(
                    padding: EdgeInsets.only(right: 10),
                    child: SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2)),
                  ),
                Expanded(child: Text(_statusMessage)),
              ],
            ),
          ),
          if (_openedFile != null) ...[
            const SizedBox(height: 12),
            Card(
              child: Padding(
                padding: const EdgeInsets.all(12),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(t('opened.banner', {'name': _fileName(_openedFile!)}), style: const TextStyle(fontWeight: FontWeight.bold)),
                    const SizedBox(height: 8),
                    Wrap(
                      spacing: 8,
                      children: [
                        FilledButton(
                          onPressed: _busy ? null : () => _extractArchive(_openedFile!),
                          child: Text(t('opened.extract')),
                        ),
                        OutlinedButton(
                          onPressed: _busy ? null : () => _viewArchive(_openedFile!),
                          child: Text(t('opened.view')),
                        ),
                        TextButton(
                          onPressed: () => setState(() => _openedFile = null),
                          child: Text(t('opened.dismiss')),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
          ],
          const SizedBox(height: 16),
          GridView.count(
            crossAxisCount: 2,
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            mainAxisSpacing: 10,
            crossAxisSpacing: 10,
            childAspectRatio: 1.6,
            children: [
              _ActionButton(icon: Icons.archive, label: t('toolbar.compress'), onPressed: _busy ? null : _onCompress),
              _ActionButton(icon: Icons.folder_zip, label: t('toolbar.extract'), onPressed: _busy ? null : _onExtract),
              _ActionButton(icon: Icons.fact_check, label: t('toolbar.test'), onPressed: _busy ? null : _onTest),
              _ActionButton(icon: Icons.visibility, label: t('toolbar.view'), onPressed: _busy ? null : _onView),
            ],
          ),
          const SizedBox(height: 16),
          Text(t('password.label'), style: Theme.of(context).textTheme.labelLarge),
          TextField(
            controller: _passwordController,
            obscureText: true,
            decoration: InputDecoration(hintText: t('password.placeholder'), border: const OutlineInputBorder()),
          ),
          const SizedBox(height: 12),
          Text(t('level.label'), style: Theme.of(context).textTheme.labelLarge),
          DropdownButton<String>(
            value: _level,
            isExpanded: true,
            items: [
              DropdownMenuItem(value: 'fast', child: Text(t('level.fast'))),
              DropdownMenuItem(value: 'normal', child: Text(t('level.normal'))),
              DropdownMenuItem(value: 'ultra', child: Text(t('level.ultra'))),
            ],
            onChanged: (value) => setState(() => _level = value ?? 'normal'),
          ),
          if (_viewingName != null) ...[
            const SizedBox(height: 20),
            Text('${t('table.name')}: $_viewingName', style: Theme.of(context).textTheme.labelLarge),
            const SizedBox(height: 8),
            if (_entries.isEmpty)
              Text(t('table.empty'))
            else
              ..._entries.map(
                (entry) => ListTile(
                  dense: true,
                  leading: Icon(entry.isDir ? Icons.folder : Icons.insert_drive_file),
                  title: Text(entry.name),
                  trailing: entry.isDir ? null : Text('${entry.size}'),
                ),
              ),
          ],
        ],
      ),
    );
  }
}

class _ActionButton extends StatelessWidget {
  final IconData icon;
  final String label;
  final VoidCallback? onPressed;

  const _ActionButton({required this.icon, required this.label, required this.onPressed});

  @override
  Widget build(BuildContext context) {
    return FilledButton(
      onPressed: onPressed,
      style: FilledButton.styleFrom(padding: const EdgeInsets.all(8)),
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(icon, size: 28),
          const SizedBox(height: 6),
          Text(label, textAlign: TextAlign.center, style: const TextStyle(fontSize: 13)),
        ],
      ),
    );
  }
}
